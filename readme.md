# dynld: macOS Dynamic Linker

A little dynamic linker experiment: it runs without `libSystem`, looks up symbols by walking the dyld shared cache in memory, rebases itself, talks to the kernel through raw Mach traps, and supports Mach-O Universal binaries (currently arm64/arm64e).

Disclaimer: The XNU kernel does not allow you to decide on the dynamic linker you choose to run your programs with. Any dynamic linker declared via `LC_LOAD_DYLINKER` is checked by the kernel and must correspond to `/usr/lib/dyld`.

The responsibility that Apple's `dyld` carries is respectable. No program, apart from `dyld`, can run without it. It knows precisely how to handle Mach-O in all its forms. This project will unfortunately never reach such a level. Especially since it's difficult to find information on this topic, there's little available on what's required to implement a dynamic linker on Apple platforms. This project aims to help those who want to run programs manually without using `dlopen`, `dlsym`, `execve`, etc.

### Implementing a Dynamic Linker

On Apple platforms, your dynamic linker is practically the only program that runs statically. Everything else must link to at least `libSystem.dylib`. This is because the ABI for system calls is unstable. There are other reasons not to do without `libSystem`, but if you're writing the dynamic linker itself, you don't really have a choice. Many things that work normally stop working, often in ways that are difficult to debug. Most of these problems fall into "implementation details." There are few online resources explaining how to do this in practice. The best reference remains Apple's dyld, which is open source: https://github.com/apple-oss-distributions/dyld.

This project is just a basic implementation of an Apple dynamic linker. It will be inferior to dyld in every way. The goal is to provide a useful starting point for anyone who needs it.

It's also worth noting that invoking this dynamic linker is slightly different from invoking `dyld`. For `dynld`, in addition to providing the address of the executable to run, the kernel must also provide the image size.

### Testing

A tool located in `tools/launcher` replicates how the kernel would invoke the dynamic linker during an `execve`. It loads the executable, loads the dynamic linker, prepares the stack (argc/argv, etc.), performs a basic parse of the linker to map its segments, locates the entry point, and jumps to it. XNU does many other things besides this, mostly unrelated to the dynamic linker, but this is essentially what `execve` is designed to do.

```sh
$ cd tools/laucher
$ make
...
$ file ../../target/aarch64-apple-darwin/debug/dynld
Mach-O 64-bit dynamic linker arm64
$ ./run /bin/ls ../../target/aarch64-apple-darwin/release/dynld
```
You can also compile with `make g` to take advantage of debugging tools, such as `fsanitize`, when the dynamic linker isn't working correctly. Debugging `dynld` is no easy task, but the excellent `lldb` remains very useful. It's difficult to create breakpoints, whether in the dynamic linker or the program it executes. Don't hesitate to recompile the tested programs and manually add `__builtin_debugtrap()` statements to the places you want to inspect, `fsanitize` will help you find the problems and where to look.

The `tests/binaries` folder contains several test programs. They are all designed to write `Hello, World!` to stdout in different ways, testing various things. You can also take a look at the CI/CD pipeline by clicking the red X next to the latest commit to see what works and what doesn't without having to rerun the tests on your machine. One major missing feature is support for Objective-C programs.