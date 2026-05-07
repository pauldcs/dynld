#!/bin/bash

set -xe

clang -arch x86_64 -o x86 hello_world.c
clang -arch arm64 -o arm64 hello_world.c
lipo -create -output hello_world x86 arm64 && rm x86 arm64