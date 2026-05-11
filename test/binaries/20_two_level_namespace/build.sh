#!/bin/bash
set -xe
clang hello_world.c -D DYLIB=1 -dynamiclib -install_name @rpath/libfoo.dylib -o libfoo.dylib
clang hello_world.c -L. -lfoo -Wl,-twolevel_namespace -Wl,-rpath,@executable_path -o hello_world
