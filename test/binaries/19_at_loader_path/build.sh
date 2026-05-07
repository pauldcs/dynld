#!/bin/bash
set -xe

clang -shared -o libgreeter.dylib greeter.c \
    -install_name @loader_path/libgreeter.dylib

clang -o hello_world main.c \
    -L. -lgreeter
