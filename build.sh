#!/bin/sh

cargo build --release --target aarch64-unknown-uefi
cargo build --release --target x86_64-unknown-uefi
cargo build --target aarch64-unknown-uefi
cargo build --target x86_64-unknown-uefi
