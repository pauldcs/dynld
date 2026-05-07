#include <stdio.h>

__attribute__((constructor(413))) void c13(void) { fputc('\n', stderr); }
__attribute__((constructor(401))) void c01(void) { fputc('e', stderr); }
__attribute__((constructor(407))) void c07(void) { fputc('W', stderr); }
__attribute__((constructor(405))) void c05(void) { fputc(',', stderr); }
__attribute__((constructor(412))) void c12(void) { fputc('!', stderr); }
__attribute__((constructor(402))) void c02(void) { fputc('l', stderr); }
__attribute__((constructor(410))) void c10(void) { fputc('l', stderr); }
__attribute__((constructor(406))) void c06(void) { fputc(' ', stderr); }
__attribute__((constructor(404))) void c04(void) { fputc('o', stderr); }
__attribute__((constructor(409))) void c09(void) { fputc('r', stderr); }
__attribute__((constructor(403))) void c03(void) { fputc('l', stderr); }
__attribute__((constructor(411))) void c11(void) { fputc('d', stderr); }
__attribute__((constructor(408))) void c08(void) { fputc('o', stderr); }
__attribute__((constructor(400))) void c00(void) { fputc('H', stderr); }

int main(int ac, char **av) {
  (void)ac;
  (void)av;
  return (0);
}