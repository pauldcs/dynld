#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <mach-o/loader.h>
#include <mach/mach.h>
#include <mach/mach_vm.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <unistd.h>

/*
 *    This is how the stack has to look like before jumpting to the
 *    entrypoint of dyld (`__dyld_start`). A little bit modified
 */

struct info_struct {
	// a pointer to the header of the dynamic linker.
	// note that we do NOT supports FAT headers, i don't
	// know if it is even possible for a dynamic linker to
	// be fat but regardless of this, we don't support it
	struct mach_header_64  *dylinker_header;
	// a pointer to the header of the executable image.
	// this can be a FAT arch containing an arm64 binary
	// or a regular arm64 executable
	struct mach_header_64  *executable_header;
	// the total size in bytes of the dynamic linker
	size_t		              dylinker_size;
	// the total size in bytes of the executable
	size_t		              executable_size;
	// argc passed to the executable
	size_t		              argc;
	// argv passed to the executable
	const char *const       *argv;
	// envp passed to the executable
	const char *const       *envp;
	// the apple args passed to the executable
	const char *const       *apple;
};

/*
 *    This function places the pointer to the `info_struct` in
 *    in the stack pointer. Dyld expects the stack pointer to point
 *    to this struct before being called
 */
__attribute__((noreturn)) static void
transfer_control(struct info_struct *init_info, void *start)
{
	__asm__ __volatile__("mov sp, %0" ::"r"(init_info));
	((void (*)(void))start)();
	__builtin_unreachable();
}

/*
 *    Loads a mach-o file into memory using mmap.
 *
 *    We use a private, anonymous mapping rather than mapping the file
 *    directly because we need the resulting region to be writable for
 *    the rebasing/protection adjustments performed later. A file-backed
 *    MAP_PRIVATE mapping would also work, but switching protections on
 *    individual segments later (via mach_vm_protect) is cleaner against
 *    an anonymous region that we own outright.
 */
static void *load_macho(
    const char        *path,
    mach_vm_address_t *out_addr,
    mach_vm_size_t    *out_size
) {
    int fd = open(path, O_RDONLY);
    if (fd == -1) {
        (void)fprintf(stderr, "%s: %s\n", path, strerror(errno));
        return (NULL);
    }
    struct stat st;
    if (fstat(fd, &st) == -1) {
        (void)fprintf(stderr, "fstat: %s: %s\n", path, strerror(errno));
        (void)close(fd);
        return (NULL);
    }
    if (st.st_size <= 0) {
        (void)fprintf(stderr, "%s: empty file\n", path);
        (void)close(fd);
        return (NULL);
    }
    size_t file_size = (size_t)st.st_size;

    void *addr = mmap(
        NULL,
        file_size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE,
        fd,
        0
    );
    (void)close(fd);
    if (addr == MAP_FAILED) {
        (void)fprintf(stderr, "mmap: %s\n", strerror(errno));
        return (NULL);
    }

    if (out_addr) {
        *out_addr = (mach_vm_address_t)(uintptr_t)addr;
    }
    if (out_size) {
        *out_size = file_size;
    }
    return (addr);
}

int main(int ac, char *av[], char *ep[], char *apple[])
{
	mach_vm_address_t macho_addr;
	mach_vm_address_t dyld_addr;
	mach_vm_size_t	  macho_size;
	mach_vm_size_t	  dyld_size;

	if (ac < 3) {
		(void)fprintf(
			stderr,
			"usage: %s <macho> <dylinker>\n",
			av[0]
		);

		return (EXIT_FAILURE);
	}

	const char *macho_path	  = av[1];
	const char *dylinker_path = av[ac - 1];

	if (!load_macho(macho_path, &macho_addr, &macho_size)) {
		return (EXIT_FAILURE);
	}

	if (!load_macho(dylinker_path, &dyld_addr, &dyld_size)) {
		return (EXIT_FAILURE);
	}

	const struct mach_header_64 *mach_header =
		(const struct mach_header_64 *)dyld_addr;

	assert(mach_header->magic == MH_MAGIC_64);
	assert(mach_header->filetype == MH_DYLINKER);

	/*
	 *    We need to parse the load commands of the program to be
	 *    able to apply memory protections to it. We cannot simply make
	 *    the whole blob exectable as dyld does some rebasing of where
	 *    it needs write permissions to some segments of itself
	 */

	struct load_command *lc	= (void *)(mach_header + 1);
	intptr_t slide = 0;
	uint32_t i = 0;

	while (i < mach_header->ncmds) {
		if (lc->cmd == LC_SEGMENT_64) {
			struct segment_command_64 *seg = (void *)lc;
			slide = (intptr_t)mach_header - (intptr_t)seg->vmaddr;
			break;
		}

		lc = (void *)((char *)lc + lc->cmdsize);
		i++;
	}

	lc = (void *)(mach_header + 1);
	i  = 0;

	while (i < mach_header->ncmds) {
		if (lc->cmd == LC_SEGMENT_64) {
			struct segment_command_64 *seg = (void *)lc;
			(void)mach_vm_protect(
				mach_task_self(),
				seg->vmaddr + slide,
				seg->vmsize,
				FALSE,
				seg->initprot
			);
		}
		lc = (void *)((char *)lc + lc->cmdsize);
		i++;
	}

	/*
	 *    The following segment of code is there to find the
	 *    entrypoint of the the dynamic linker.
	 *
	 *    We look for `LC_UNIXTHREAD`, and once we find it we extract
	 *    the thread state to get the entry point. We have to do this
	 *    due to the fact that the linker is under constant development
	 *    and the entrypoint varies accros compilations.
	 *
	 *    Once it stabilizes the entrypoint will be made somehow
	 *    persistent and we will be able to skip this part.
	 */

	uint64_t dyld_entry = 0;
	lc = (void *)(mach_header + 1);
	i  = 0;
	while (i < mach_header->ncmds) {
		if (lc->cmd == LC_UNIXTHREAD) {
			uint32_t *p = (uint32_t *)((char *)lc + sizeof(struct thread_command));
			uint32_t flavor = p[0];
			void *state = &p[2];

			if (flavor == ARM_THREAD_STATE64) {
				arm_thread_state64_t *ts =
					(arm_thread_state64_t *)state;
				dyld_entry = ts->__pc;
			}
			break;
		}
		lc = (void *)((char *)lc + lc->cmdsize);
		i++;
	}

	assert(dyld_entry != 0);

	void *entry = (void *)((uintptr_t)dyld_entry + slide);

	av++;          /* drop argv[0]   */
	ac--;
	av[ac] = NULL; /* drop last argv */
	ac--;

	struct info_struct info = {
		.dylinker_header   = (struct mach_header_64 *)dyld_addr,
		.executable_header = (struct mach_header_64 *)macho_addr,
		.dylinker_size	   = dyld_size,
		.executable_size   = macho_size,
		.argc		           = ac,
		.argv		           = (const char *const *)av,
		.envp		           = (const char *const *)ep,
		.apple	           = (const char *const *)apple,
	};

	/*
	 *    This jumps to dyld, we expect this to not return
	 */
	transfer_control(&info, entry);
}