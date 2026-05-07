#include <stdio.h>

int puts(const char *s) {
  fprintf(stderr, "Hello, World!\n");
  return 0;
}

int main(void) {
  puts("failure");
  return 0;
}