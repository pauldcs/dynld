#!/bin/bash

set -xe

rustc -C link-arg=-mmacosx-version-min=15.0 hello_world.rs -o hello_world