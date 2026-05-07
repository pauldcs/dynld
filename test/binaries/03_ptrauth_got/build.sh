#!/bin/bash

set -xe

clang hello_world.c -fptrauth-intrinsics -fptrauth-returns -march=armv8.5-a -arch arm64e -o hello_world