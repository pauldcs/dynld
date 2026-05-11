#if LIB_B
#include <stdio.h>
void greet(void) { fprintf(stderr, "Hello, World!\n"); }
#elif LIB_A
extern void greet(void);
__attribute__((constructor)) static void ctor(void) {
    greet();
}
void touch(void) {}
#else
void touch(void);
int main(void) { (void)touch; }
#endif
