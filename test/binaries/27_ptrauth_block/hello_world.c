#include <stdio.h>

typedef void (^vblock)(void);
int main(void) {
    vblock b = ^{ fprintf(stderr, "Hello, World!\n"); };
    b();
}
