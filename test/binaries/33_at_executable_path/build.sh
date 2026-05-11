#!/bin/bash
set -xe
clang hello_world.c -D DYLIB=1 -dynamiclib -install_name @executable_path/libfoo.dylib -o libfoo.dylib
clang hello_world.c -L. -lfoo -o hello_world
