#!/bin/bash
set -xe
clang hello_world.c -D DYLIB=1 -dynamiclib -install_name @rpath/libfoo.dylib -o libfoo.dylib
clang hello_world.c -Wl,-weak_library,libfoo.dylib -Wl,-rpath,@executable_path -o hello_world
