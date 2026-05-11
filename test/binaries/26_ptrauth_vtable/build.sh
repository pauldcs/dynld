#!/bin/bash
set -xe
clang++ hello_world.cxx -arch arm64e -fptrauth-intrinsics -fptrauth-returns -march=armv8.5-a -o hello_world
