#include <stdio.h>

static const char *string = "Hello, World!";

int main(int ac, char **av) {
  (void)ac;
  (void)av;

  (void)fprintf(stderr, "%s\n", string);
  return (0);
}
