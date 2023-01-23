#!/bin/sh

#!/bin/bash

#C-a h    print this help
#C-a x    exit emulator
#C-a s    save disk data back to file (if -snapshot)
#C-a t    toggle console timestamps
#C-a b    send break (magic sysrq)
#C-a c    switch between console and monitor
#C-a C-a  sends C-a

OVMF_CODE=$(PWD)/edk2-uefi/ovmf-x64-4m/OVMF_CODE.fd
OVMF_VARS=$(PWD)/edk2-uefi/ovmf-x64-4m/OVMF_VARS.fd
BUILD_DIR=$(PWD)/target/x86_64-unknown-uefi/release
EFI_DIR=$(PWD)/esp
OVMF_DIR=$(PWD)/ovmf
NUM_PROC=8
REVISION=`git log -1 --oneline`

# SECTIONS=.text .sdata .data .dynamic .dynsym .rel .rela .reloc
# DEBUG_SECTIONS=.debug_info .debug_abbrev .debug_loc .debug_aranges \
# 	.debug_line .debug_macinfo .debug_str 
# OBJCOPY=/opt/homebrew/Cellar/binutils/2.40/bin/objcopy

# $(OBJCOPY) -j .text -j .sdata -j .data -j .dynamic -j .dynsym -j .rel -j .rela -j .reloc  \
#         --target=efi-app-x86_64 \
#         $BUILD_DIR/corgos-boot $BUILD_DIR/corgos-boot.efi
# $(OBJCOPY) $(foreach sec,$(SECTIONS) $(DEBUG_SECTIONS),-j $(sec)) --target=efi-app-x86_64 \
#     $BUILD_DIR/corgos-boot $BUILD_DIR/corgos-boot.debug

mkdir -p $EFI_DIR/efi/boot
mkdir -p $OVMF_DIR
cp $OVMF_CODE $OVMF_DIR
cp $OVMF_VARS $OVMF_DIR
cp $BUILD_DIR/corgos-boot.efi $EFI_DIR/efi/boot/bootx64.efi
echo "revision = \"$REVISION\"" > $EFI_DIR/efi/boot/corgos-boot.ini
echo "log_device = stdout" >> $EFI_DIR/efi/boot/corgos-boot.ini
echo "log_level = debug" >> $EFI_DIR/efi/boot/corgos-boot.ini

qemu-system-x86_64 \
    -nodefaults \
    -machine q35 -smp $NUM_PROC \
    -m 64M \
    -drive if=pflash,format=raw,file=$OVMF_DIR/OVMF_CODE.fd,readonly=on \
    -drive if=pflash,format=raw,file=$OVMF_DIR/OVMF_VARS.fd,readonly=on \
    -drive format=raw,file=fat:rw:$EFI_DIR \
    -chardev stdio,id=char0,mux=on,logfile=serial.log,signal=off \
    -serial chardev:char0 \
    -mon chardev=char0 \
    -nographic \
#    -vga std \
#    -d guest_errors -d cpu_reset -d int -D qemu.log -no-reboot -no-shutdown
