#include <cstdio>
struct Greeter {
    virtual void hello() { std::fprintf(stderr, "Hello, World!\n"); }
    virtual ~Greeter() = default;
};
int main() {
    Greeter g;
    Greeter *p = &g;
    p->hello();
}
