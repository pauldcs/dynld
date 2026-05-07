#if LIB_A
#include <stdio.h>
void hello_world(void) { fprintf(stderr, "Hello, World!\n"); }

#elif LIB_B
extern void hello_world(void);

#else
#include <stdio.h>
void hello_world(void);

int main(void) { hello_world(); }
#endif