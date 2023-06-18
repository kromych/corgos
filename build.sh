#!/bin/sh

cargo clippy --target corgboot/targets/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo clippy --target corgboot/targets/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo clippy --target corgboot/targets/aarch64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo clippy --target corgboot/targets/aarch64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem

cargo build --release --target corgboot/targets/x86_64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
cargo build --release --target corgboot/targets/aarch64-boot.json -Zbuild-std=core -Zbuild-std-features=compiler-builtins-mem
