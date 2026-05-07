#include <stdio.h>

__attribute__((weak)) void hello_world() {
  (void)fprintf(stderr, "%s\n", "Hello, World!");
}

int main(int ac, char **av) {
  (void)ac;
  (void)av;

  hello_world();
  return (0);
}
