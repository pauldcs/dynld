// The entrypoint of the dynamic linker. This is where the kernel jumps once
// it has loaded the dynamic linker as well as the program to execute into memory.
// The kernel is responsible for making the stack pointer point to a specific struct,
// and this code calls the `start` function with that struct as it's first parameter.
//
// This assembly code also needs to align the stack. It nulls out the frame pointer
// and the link register, which means `start` can never return here.
//
// This was copied from this:
// https://github.com/apple-oss-distributions/dyld/blob/main/dyld/dyldStartup.s

.text
.align 4
.globl __dyld_start

__dyld_start:
    mov  x0, sp        // get pointer to info struct into parameter register
    and  sp, x0, #~15  // force 16-byte alignment of stack
    mov  fp, #0        // first frame
    mov  lr, #0        // no return address
    b    _start
