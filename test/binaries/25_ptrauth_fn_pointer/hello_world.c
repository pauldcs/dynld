#include <stdio.h>
static void greet(void) { fprintf(stderr, "Hello, World!\n"); }

void (*const fp)(void) = greet;
int main(void) { fp(); }
