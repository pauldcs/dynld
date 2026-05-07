#include <unistd.h>

int main(int ac, char **av) {
  (void)ac;
  (void)av;

  (void)write(STDERR_FILENO, "Hello, World!\n", 14);
  return (0);
}