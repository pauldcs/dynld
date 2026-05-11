#!/bin/bash
set -xe
mkdir -p subdir
clang hello_world.c -D DYLIB=1 -dynamiclib -install_name @rpath/libfoo.dylib -o subdir/libfoo.dylib
clang hello_world.c -L./subdir -lfoo \
    -Wl,-rpath,@executable_path/nope_does_not_exist \
    -Wl,-rpath,@executable_path/subdir \
    -o hello_world
