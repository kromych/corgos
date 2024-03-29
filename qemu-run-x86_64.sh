#!/bin/sh

#C-a h    print this help
#C-a x    exit emulator
#C-a s    save disk data back to file (if -snapshot)
#C-a t    toggle console timestamps
#C-a b    send break (magic sysrq)
#C-a c    switch between console and monitor
#C-a C-a  sends C-a

OVMF_CODE=${PWD}/edk2-uefi/ovmf-x64-4m/OVMF_CODE.fd
OVMF_VARS=${PWD}/edk2-uefi/ovmf-x64-4m/OVMF_VARS.fd
BUILD_DIR=${PWD}/target/x86_64-boot/release
EFI_DIR=${PWD}/esp
OVMF_DIR=${PWD}/ovmf
NUM_PROC=8
REVISION=`git log -1 --oneline`
BOOT_INI_FILE=$EFI_DIR/corgos-boot-x86_64.ini

rm -rf $EFI_DIR
rm -rf $OVMF_DIR

mkdir -p $EFI_DIR/efi/boot
mkdir -p $OVMF_DIR

cp $OVMF_CODE $OVMF_DIR
cp $OVMF_VARS $OVMF_DIR
cp $BUILD_DIR/corgos-boot.efi $EFI_DIR/efi/boot/bootx64.efi
echo "revision = \"$REVISION\"" > $BOOT_INI_FILE
echo "log_device = com2" >> $BOOT_INI_FILE
echo "log_level = trace" >> $BOOT_INI_FILE
echo "wait_for_start = false" >> $BOOT_INI_FILE
echo "wait_for_start = false" >> $BOOT_INI_FILE

qemu-system-x86_64 \
    -nodefaults -s \
    -machine q35 -smp $NUM_PROC \
    -m 256M \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -chardev file,id=fwdebug,path=fw.log \
    -device isa-debugcon,iobase=0x402,chardev=fwdebug \
    -drive if=pflash,format=raw,file=$OVMF_DIR/OVMF_CODE.fd,readonly=on \
    -drive if=pflash,format=raw,file=$OVMF_DIR/OVMF_VARS.fd,readonly=on \
    -drive format=raw,file=fat:rw:$EFI_DIR \
    -chardev stdio,id=char0,mux=on,logfile=serial1.log,signal=off \
    -serial chardev:char0 \
    -mon chardev=char0 \
    -chardev file,path=serial2.log,id=char1 \
    -serial chardev:char1 \
    -nographic \
#    -vga std \
#    -d guest_errors -d cpu_reset -d int -D qemu.log -no-reboot -no-shutdown
