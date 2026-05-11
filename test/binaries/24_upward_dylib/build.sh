#!/bin/bash
set -xe
clang hello_world.c -D LIB_A=1 -dynamiclib -install_name @rpath/libA.dylib \
    -Wl,-undefined,dynamic_lookup -o libA.dylib
clang hello_world.c -D LIB_B=1 -dynamiclib -install_name @rpath/libB.dylib \
    -L. -Wl,-upward_library,libA.dylib -o libB.dylib
clang hello_world.c -D LIB_A=1 -dynamiclib -install_name @rpath/libA.dylib \
    -L. -lB -o libA.dylib
clang hello_world.c -L. -lA -Wl,-rpath,@executable_path -o hello_world
