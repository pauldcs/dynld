#!/bin/bash
set -xe
clang hello_world.m -D LIB_A=1 -dynamiclib -install_name @rpath/libA.dylib     -framework Foundation -o libA.dylib
clang hello_world.m -D LIB_B=1 -dynamiclib -install_name @rpath/libB.dylib     -L. -lA -framework Foundation -o libB.dylib
clang hello_world.m -L. -lB -Wl,-rpath,@executable_path     -framework Foundation -o hello_world
