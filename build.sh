#!/bin/sh

set -e

cargo build --release --target aarch64-unknown-uefi -p boot_loader
cargo build --release --target x86_64-unknown-uefi -p boot_loader
cargo build --target aarch64-unknown-uefi -p boot_loader
cargo build --target x86_64-unknown-uefi -p boot_loader
