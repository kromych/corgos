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
BUILD_DIR=$(PWD)/target/x86_64-unknown-uefi/debug
EFI_DIR=$(PWD)/esp
OVMF_DIR=$(PWD)/ovmf

mkdir -p $EFI_DIR/efi/boot
mkdir -p $OVMF_DIR
cp $OVMF_CODE $OVMF_DIR
cp $OVMF_VARS $OVMF_DIR
cp $BUILD_DIR/corgos-boot.efi $EFI_DIR/efi/boot/bootx64.efi
echo "log_device = stdout" > $EFI_DIR/efi/boot/corgos-boot.ini
echo "log_level = debug" >> $EFI_DIR/efi/boot/corgos-boot.ini

qemu-system-x86_64 \
    -nodefaults \
    -machine q35 -smp $(nproc) \
    -nographic \
    -m 64M \
    -drive if=pflash,format=raw,file=$OVMF_DIR/OVMF_CODE.fd,readonly=on \
    -drive if=pflash,format=raw,file=$OVMF_DIR/OVMF_VARS.fd,readonly=on \
    -drive format=raw,file=fat:rw:$EFI_DIR \
    -nographic \
    -chardev stdio,id=char0,mux=on,logfile=serial.log,signal=off \
    -serial chardev:char0 \
    -mon chardev=char0 \
#    -d guest_errors -d cpu_reset -d int -D qemu.log -no-reboot -no-shutdown
