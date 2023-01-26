#!/bin/bash

OBJCOPY=/opt/homebrew/Cellar/binutils/2.40/bin/objcopy
STRIP=/opt/homebrew/Cellar/binutils/2.40/bin/strip

$OBJCOPY \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/debug/corgos-boot  \
    ../target/x86_64-boot/debug/corgos-boot.efi.debugsym
$STRIP ../target/x86_64-boot/debug/corgos-boot
$OBJCOPY \
    -j .text \
    -j .reloc \
    -j .eh_fram \
    -j .data \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/debug/corgos-boot  \
    ../target/x86_64-boot/debug/corgos-boot.efi

ls -l ../target/x86_64-boot/debug/corgos-boot*

$OBJCOPY \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/release/corgos-boot  \
    ../target/x86_64-boot/release/corgos-boot.efi.debugsym
$STRIP ../target/x86_64-boot/release/corgos-boot
$OBJCOPY \
    -j .text \
    -j .reloc \
    -j .eh_fram \
    -j .data \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/release/corgos-boot  \
    ../target/x86_64-boot/release/corgos-boot.efi

ls -l ../target/x86_64-boot/release/corgos-boot*
