#include <iostream>

struct X {
  X() { std::cerr << "Hello, World!\n"; }
};

static X x;

int main() {}