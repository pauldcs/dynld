int main(int argc, char **argv) {
  (void)argc;
  (void)argv;

  long ret;
  const int syswrite = 0x2000004;
  const char *str = "Hello, World!\n";

  __asm__ volatile("mov x0, #2\n" // STDERR_FILENO
                   "ldr x1, %[msg_ptr]\n"
                   "mov x2, #14\n"
                   "ldr x16, %[syscall]\n"
                   "svc #0\n"
                   "mov %[ret], x0\n"
                   : [ret] "=r"(ret)
                   : [msg_ptr] "m"(str), [syscall] "m"(syswrite)
                   : "x0", "x1", "x2", "x16", "memory");

  return 0;
}