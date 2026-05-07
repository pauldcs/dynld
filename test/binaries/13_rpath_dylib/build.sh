#!/bin/bash

set -xe

clang hello_world.c -D DYLIB=1 -dynamiclib -install_name @rpath/libfoo.dylib -o libfoo.dylib
clang hello_world.c -o hello_world -L. -lfoo -Wl,-rpath,@executable_path