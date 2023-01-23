#!/bin/bash

OBJCOPY=/opt/homebrew/Cellar/binutils/2.40/bin/objcopy

$OBJCOPY \
    -j .start \
    -j .text \
    -j .bss \
    -j .data \
    -j .rodata \
    -j .sdata \
    -j .dynamic \
    -j .dynsym \
    -j .eh_frame \
    -j .eh_frame_hdr \
    -j .rel \
    -j .rela \
    -j .reloc \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/debug/corgos-boot  \
    ../target/x86_64-boot/debug/corgos-boot.efi

$OBJCOPY \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/debug/corgos-boot  \
    ../target/x86_64-boot/debug/corgos-boot.efi.debugsym

ls -l ../target/x86_64-boot/debug/corgos-boot*

$OBJCOPY \
    -j .start \
    -j .text \
    -j .bss \
    -j .data \
    -j .rodata \
    -j .sdata \
    -j .dynamic \
    -j .dynsym \
    -j .eh_frame \
    -j .eh_frame_hdr \
    -j .rel \
    -j .rela \
    -j .reloc \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/release/corgos-boot  \
    ../target/x86_64-boot/release/corgos-boot.efi

$OBJCOPY \
    --target=efi-app-x86_64 \
    ../target/x86_64-boot/release/corgos-boot  \
    ../target/x86_64-boot/release/corgos-boot.efi.debugsym

ls -l ../target/x86_64-boot/release/corgos-boot*
