#if DYLIB
#include <stdio.h>
void hello_world(void) { fprintf(stderr, "Hello, World!\n"); }
#else
#include <stdio.h>
__attribute__((weak)) extern void hello_world(void);
int main(void) {
    if (&hello_world) hello_world();
    else fprintf(stderr, "weak symbol missing but dylib should be loaded\n");
}
#endif
