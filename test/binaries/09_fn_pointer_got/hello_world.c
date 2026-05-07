#include <stdio.h>

int main(void) {
  int (*fp)(FILE *, const char *, ...) = fprintf;
  fp(stderr, "Hello, World!\n");
  return 0;
}
