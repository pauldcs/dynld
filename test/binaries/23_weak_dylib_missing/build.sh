#!/bin/bash
set -xe
clang hello_world.c -D DYLIB=1 -dynamiclib -install_name @rpath/libmissing.dylib -o libmissing.dylib
clang hello_world.c -Wl,-weak_library,libmissing.dylib -Wl,-rpath,@executable_path -o hello_world
rm -f libmissing.dylib
