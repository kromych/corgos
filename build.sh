#!/bin/sh

cargo build --target boot/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo build --release --target boot/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
