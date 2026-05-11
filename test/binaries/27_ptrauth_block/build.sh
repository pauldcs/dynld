#!/bin/bash
set -xe
clang hello_world.c -arch arm64e -fptrauth-intrinsics -fptrauth-returns -march=armv8.5-a \
    -fblocks -o hello_world
