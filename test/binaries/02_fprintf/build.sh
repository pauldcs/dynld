#!/bin/bash

set -x

clang hello_world.c -arch x86_64 -o x86
clang hello_world.c -arch arm64 -o arm64
lipo -create -output hello_world x86 arm64
rm x86 arm64