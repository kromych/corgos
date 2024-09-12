#!/bin/sh

#C-a h    print this help
#C-a x    exit emulator
#C-a s    save disk data back to file (if -snapshot)
#C-a t    toggle console timestamps
#C-a b    send break (magic sysrq)
#C-a c    switch between console and monitor
#C-a C-a  sends C-a

SILENT="silent-"

OVMF_CODE=${PWD}/edk2-uefi/aarch64/QEMU_EFI-${SILENT}pflash.raw
OVMF_VARS=${PWD}/edk2-uefi/aarch64/vars-template-pflash.raw
BUILD_DIR=${PWD}/target/aarch64-unknown-uefi/release
EFI_DIR=${PWD}/esp
OVMF_DIR=${PWD}/ovmf
NUM_PROC=8
REVISION=`git log -1 --oneline`
BOOT_INI_FILE=$EFI_DIR/corgos-boot-aarch64.ini

rm -rf $EFI_DIR
rm -rf $OVMF_DIR

mkdir -p $EFI_DIR/efi/boot
mkdir -p $OVMF_DIR

cp $OVMF_CODE $OVMF_DIR
cp $OVMF_VARS $OVMF_DIR
cp $BUILD_DIR/corgos-boot.efi $EFI_DIR/efi/boot/bootaa64.efi
echo "revision = \"$REVISION\"" > $BOOT_INI_FILE
echo "log_device = \"pl011@9000000\"" >> $BOOT_INI_FILE
echo "log_level = trace" >> $BOOT_INI_FILE
echo "wait_for_start = false" >> $BOOT_INI_FILE
echo "walk_page_tables = false" >> $BOOT_INI_FILE

qemu-system-aarch64 \
    -nodefaults -s \
    -cpu max \
    -machine virt -smp $NUM_PROC \
    -m 256M \
    --semihosting \
    -drive if=pflash,format=raw,file=$OVMF_DIR/QEMU_EFI-${SILENT}pflash.raw,readonly=on \
    -drive if=pflash,format=raw,file=$OVMF_DIR/vars-template-pflash.raw \
    -drive format=raw,file=fat:rw:$EFI_DIR \
    -chardev stdio,id=char0,mux=on,logfile=serial1.log,signal=off \
    -serial chardev:char0 \
    -mon chardev=char0 \
    -chardev file,path=serial2.log,id=char1 \
    -serial chardev:char1 \
    -nographic 
    #-device VGA \
