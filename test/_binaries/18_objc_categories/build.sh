#!/bin/bash
set -xe

clang -o hello_world main.m \
    -framework Foundation \
    -ObjC
