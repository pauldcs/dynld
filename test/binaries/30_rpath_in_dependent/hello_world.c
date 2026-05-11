#if LEAF
#include <stdio.h>
void leaf(void) { fprintf(stderr, "Hello, World!\n"); }
#elif MIDDLE
extern void leaf(void);
void middle(void) { leaf(); }
#else
void middle(void);
int main(void) { middle(); }
#endif
