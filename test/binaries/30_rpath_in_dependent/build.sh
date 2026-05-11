#!/bin/bash
set -xe
mkdir -p sub
clang hello_world.c -D LEAF=1 -dynamiclib -install_name @rpath/libleaf.dylib -o sub/libleaf.dylib
clang hello_world.c -D MIDDLE=1 -dynamiclib -install_name @rpath/libmiddle.dylib \
    -L./sub -lleaf -Wl,-rpath,@loader_path/sub -o libmiddle.dylib
clang hello_world.c -L. -lmiddle -Wl,-rpath,@executable_path -o hello_world
