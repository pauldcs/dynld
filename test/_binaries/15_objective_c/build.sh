#!/bin/bash

set -xe

gcc -lobjc -framework Foundation hello_world.m -o hello_world