#include <stdio.h>

__thread char *str = "Hello, World!";

int main(void) {
  fprintf(stderr, "%s\n", str);
}