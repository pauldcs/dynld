#if DYLIB
#include <stdio.h>
void other_symbol(void) { fprintf(stderr, "should not run\n"); }
#else
#include <stdio.h>
__attribute__((weak)) extern void other_symbol(void);
int main(void) {
    if (&other_symbol) other_symbol();
    fprintf(stderr, "Hello, World!\n");
}
#endif
