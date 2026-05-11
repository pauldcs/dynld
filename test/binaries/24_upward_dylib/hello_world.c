#if LIB_A
#include <stdio.h>
extern void from_b(void);
void from_a(void) { fprintf(stderr, "Hello, World!\n"); }
void entry(void) { from_b(); }
#elif LIB_B
extern void from_a(void);
void from_b(void) { from_a(); }
#else
void entry(void);
int main(void) { entry(); }
#endif
