#!/usr/bin/env python3

import os
import platform
import subprocess
import shutil
import argparse


NUM_PROC = 8
MEMORY = "256M"
SILENT = "silent-"
PWD = os.getcwd()

ARCH_CONFIG = {
    "x86_64": {
        "ovmf_code": f"{PWD}/edk2-uefi/ovmf-x64-4m/OVMF_CODE.fd",
        "ovmf_vars": f"{PWD}/edk2-uefi/ovmf-x64-4m/OVMF_VARS.fd",
        "build_dir": f"{PWD}/target/x86_64-unknown-uefi/release",
        "boot_efi": "bootx64.efi",
        "boot_ini": "corgos-boot-x86_64.ini",
        "log_device": "com2",
        "machine": "q35",
        "semihosting": "-device isa-debug-exit,iobase=0xf4,iosize=0x04"
    },
    "aarch64": {
        "ovmf_code": f"{PWD}/edk2-uefi/aarch64/QEMU_EFI-{SILENT}pflash.raw",
        "ovmf_vars": f"{PWD}/edk2-uefi/aarch64/vars-template-pflash.raw",
        "build_dir": f"{PWD}/target/aarch64-unknown-uefi/release",
        "boot_efi": "bootaa64.efi",
        "boot_ini": "corgos-boot-aarch64.ini",
        "log_device": "\"pl011@9000000\"",
        "machine": "virt",
        "semihosting": "--semihosting"
    }
}

EFI_DIR = f"{PWD}/esp"
OVMF_DIR = f"{PWD}/ovmf"


def get_git_info():
    revision = subprocess.check_output(r"git rev-parse --short HEAD".split()).strip().decode('utf-8')
    branch = subprocess.check_output(r"git rev-parse --abbrev-ref HEAD".split()).strip().decode('utf-8')
    date = subprocess.check_output(r"git --no-pager log -1 --pretty=format:%cd --date=format:%Y-%m-%d@%H:%M:%S".split()).strip().decode('utf-8')
    dirty = "(dirty)" if subprocess.check_output(r"git status --porcelain".split()).strip() else ""
    return revision, branch, dirty, date


def setup_directories():
    if os.path.exists(EFI_DIR):
        shutil.rmtree(EFI_DIR)
    if os.path.exists(OVMF_DIR):
        shutil.rmtree(OVMF_DIR)
    os.makedirs(f"{EFI_DIR}/efi/boot", exist_ok=True)
    os.makedirs(OVMF_DIR, exist_ok=True)


def copy_files(arch):
    config = ARCH_CONFIG[arch]
    shutil.copy(config["ovmf_code"], OVMF_DIR)
    shutil.copy(config["ovmf_vars"], OVMF_DIR)
    shutil.copy(f"{config['build_dir']}/boot_loader.efi", f"{EFI_DIR}/efi/boot/{config['boot_efi']}")


def write_boot_ini(arch):
    config = ARCH_CONFIG[arch]
    revision, branch, dirty, date = get_git_info()
    boot_ini_file = f"{EFI_DIR}/{config['boot_ini']}"
    with open(boot_ini_file, 'w') as ini_file:
        ini_file.write(f'revision = "{revision}{dirty} {date}, branch \'{branch}\'"\n')
        ini_file.write(f'log_device = {config["log_device"]}\n')
        ini_file.write('log_level = trace\n')
        ini_file.write('wait_for_start = false\n')
        ini_file.write('walk_page_tables = false\n')


def get_arch_name_normalized(arch_name):
    arch_normalized = arch_name.lower()
    if arch_name == "arm64":
        arch_normalized = "aarch64"
    elif arch_name == "amd64":
        arch_normalized = "x86_64"
    elif arch_name == "x64":
        arch_normalized = "x86_64"
    return arch_normalized


def get_accelerator(target_arch, accel):
    system_arch = platform.machine().lower()

    # Disable acceleration if not native architecture or user requested no acceleration
    if not accel:
        return ""
    if get_arch_name_normalized(system_arch) != get_arch_name_normalized(target_arch):
        raise Exception("Can't enable hardware acceleration for a non-native guest")

    system = platform.system().lower()
    if "linux" in system:
        return "-enable-kvm"
    elif "darwin" in system:
        return "-accel hvf"
    return ""

def run_qemu(arch, accel):
    config = ARCH_CONFIG[arch]
    
    setup_directories()
    copy_files(arch)
    write_boot_ini(arch)

    accel_option = get_accelerator(arch, accel)
    qemu_command = f"""
        qemu-system-{arch} 
            -nodefaults -s
            -machine {config['machine']}
            {accel_option}
            -cpu max
            -m {MEMORY}
            -smp {NUM_PROC}
            {config['semihosting']}
            -chardev stdio,id=char0,mux=on,logfile=serial1.log,signal=off
            -chardev file,id=fwdebug,path=fw.log
            -serial chardev:char0
            -mon chardev=char0
            -chardev file,path=serial2.log,id=char1
            -serial chardev:char1
            -drive format=raw,file=fat:rw:{EFI_DIR}
            -drive if=pflash,format=raw,file={OVMF_DIR}/{os.path.basename(config['ovmf_code'])},readonly=on
            -drive if=pflash,format=raw,file={OVMF_DIR}/{os.path.basename(config['ovmf_vars'])}
            -nographic
    """.split()
    subprocess.run(qemu_command, shell=False, check=True)


def main():
    parser = argparse.ArgumentParser(description="Run QEMU for CorgOS development")
    parser.add_argument(dest='arch', choices=['x86_64', 'aarch64'],  help="Target architecture (x86_64 or aarch64)")
    parser.add_argument('--accel', action='store_true', help="Enable hardware acceleration")
    args = parser.parse_args()

    try:
        run_qemu(args.arch, args.accel)
    except Exception as e:
        print(f"Error {e}")


if __name__ == "__main__":
    main()
