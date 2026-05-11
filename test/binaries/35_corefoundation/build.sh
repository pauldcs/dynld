#!/bin/bash
set -xe
clang hello_world.c -framework CoreFoundation -o hello_world
