#!/bin/sh

cargo clippy --target corgboot/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo clippy --target corgboot/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem

cargo build --target corgboot/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo build --release --target corgboot/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
