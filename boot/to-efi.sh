#!/bin/bash

OBJCOPY=/opt/homebrew/Cellar/binutils/2.40/bin/objcopy

$OBJCOPY -j .text -j .sdata -j .data -j .dynamic -j .dynsym -j .rel -j .rela -j .reloc \
    --target=efi-app-x86_64 \
    ../target/x86_64-unknown-none/debug/corgos-boot  \
    ../target/x86_64-unknown-none/debug/corgos-boot.efi

$OBJCOPY \
    --target=efi-app-x86_64 \
    ../target/x86_64-unknown-none/debug/corgos-boot  \
    ../target/x86_64-unknown-none/debug/corgos-boot.efi.debugsym

ls -l ../target/x86_64-unknown-none/debug/corgos-boot*