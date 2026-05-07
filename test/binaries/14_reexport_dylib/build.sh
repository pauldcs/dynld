#!/bin/bash

set -xe

clang hello_world.c -dynamiclib -D LIB_A=1 -install_name @rpath/libA.dylib -o libA.dylib
clang hello_world.c -dynamiclib -D LIB_B=1 -install_name @rpath/libB.dylib -Wl,-reexport_library,libA.dylib -o libB.dylib
clang hello_world.c -L. -lB -Wl,-rpath,@loader_path -o hello_world