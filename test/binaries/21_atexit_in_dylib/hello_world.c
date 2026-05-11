#if DYLIB
#include <stdio.h>
#include <stdlib.h>
static void bye(void) { fprintf(stderr, "Hello, World!\n"); }
void register_bye(void) { atexit(bye); }
#else
void register_bye(void);
int main(void) { register_bye(); }
#endif
