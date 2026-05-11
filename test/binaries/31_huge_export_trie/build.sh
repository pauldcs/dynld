#!/bin/bash
set -xe
clang generated.c -D DYLIB=1 -dynamiclib -install_name @rpath/libfoo.dylib -o libfoo.dylib
clang hello_world.c -L. -lfoo -Wl,-rpath,@executable_path -o hello_world
