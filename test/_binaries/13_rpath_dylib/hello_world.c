#if DYLIB
#include <stdio.h>
void hello_world(void) {
  fprintf(stderr, "Hello, World!\n");
}
#else

void hello_world(void);

int main(void) {
  hello_world();
}
#endif