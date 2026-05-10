#include <stdint.h>
#include <stdbool.h>

// This does a shared_region_check_np(uint64_t *start_address).
// it is the bsd syscall 294
//
// This function takes a pointer and populated it with the shared caches base
// address if it returned 0. -1 means an error happened
//
// this syscall is initially intended for dyld. This is copied from apples
// comment in vm_unix.c:
// dyld calls this when any process starts to see if the process's shared
// region is already set up and ready to use.
// This call returns the base address of the first mapping in the
// process's shared region's first mapping.
// dyld will then check what's mapped at that address.
__attribute__((weak))
bool dyld_shared_cache_base_address_get(uint64_t *start_address) {
    register uint64_t x0 __asm__("x0") = (uint64_t)start_address;
    register uint64_t x16 __asm__("x16") = 294;
    uint64_t cf;

    __asm__ volatile(
        "svc #0x80\n\t"
        "cset %[cf], cs\n\t"
        : "+r"(x0), [cf] "=r"(cf)
        : "r"(x16)
        : "cc", "memory"
    );

    return (!cf);
}
