#!/bin/sh
set -o xtrace

# just a simple function
rustc --remap-path-prefix $PWD=root-comp-dir --target x86_64-unknown-linux-gnu -O -C debuginfo=1 --crate-type staticlib --emit obj simple.rs
clang -Wl,--gc-sections -shared -o simple.so simple.o
objcopy --only-keep-debug simple.so simple.debug

# a simple function being inlined into another function
rustc --remap-path-prefix $PWD=root-comp-dir --target x86_64-unknown-linux-gnu -O -C debuginfo=1 --crate-type staticlib --emit obj inlined.rs
clang -Wl,--gc-sections -shared -o inlined.so inlined.o
objcopy --only-keep-debug inlined.so inlined.debug
