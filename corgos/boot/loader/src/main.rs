#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_arch = "aarch64")]
mod aarch64_regs;

use boot_logger::BootLoaderConfig;
use boot_logger::LogDevice;
use core::arch::asm;
use log::LevelFilter;
use uefi::boot;
use uefi::mem::memory_map::MemoryMap;
use uefi::mem::memory_map::MemoryMapMut;
use uefi::proto::console::text::Input;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::runtime;
use uefi::system;
use uefi::table::boot::MemoryType;
use uefi::table::runtime::ResetType;
use uefi::CStr16;
use uefi::Status;

/// The name of the configuration file in the ESP partition alongside the loader.
#[cfg(target_arch = "x86_64")]
const CORGOS_INI: &CStr16 = uefi::cstr16!("corgos-boot-x86_64.ini");
#[cfg(target_arch = "aarch64")]
const CORGOS_INI: &CStr16 = uefi::cstr16!("corgos-boot-aarch64.ini");

/// Upon panic, b"CORGBARF" is loaded into R8. R9 contains the address of the file name,
/// R10 contains the line number in the least significant 32 bits, and the column number
/// in the most significant 32 bits.
/// The interrupts are disabled and the processor is halted.
const CORGOS_BARF: u64 = u64::from_le_bytes([0x46, 0x52, 0x41, 0x42, 0x47, 0x52, 0x4f, 0x43]);

/// Timeout for the boot services.
const WATCHDOG_TIMEOUT_CODE: u64 = CORGOS_BARF;

fn parse_config(bytes: &[u8]) -> Option<BootLoaderConfig> {
    let mut config = BootLoaderConfig::default();
    let mut parser = ini_file::Parser::new(bytes);

    while let Ok(Some(ini_file::KeyValue { key, value })) = parser.parse() {
        match key {
            b"log_device" => match value {
                b"null" => config.log_device = LogDevice::Null,
                b"com1" => config.log_device = LogDevice::Com1,
                b"com2" => config.log_device = LogDevice::Com2,
                b"stdout" => config.log_device = LogDevice::StdOut,
                _ => {
                    // TODO: must be Device Tree or ACPI
                    if value.starts_with(b"pl011@") {
                        if let Ok(base_addr) = u64::from_str_radix(
                            core::str::from_utf8(&value[b"pl011@".len()..]).unwrap_or_default(),
                            16,
                        ) {
                            config.log_device = LogDevice::Pl011(base_addr)
                        } else {
                            config.log_device = LogDevice::StdOut
                        }
                    }
                }
            },
            b"log_level" => match value {
                b"info" => config.log_level = LevelFilter::Info,
                b"warn" => config.log_level = LevelFilter::Warn,
                b"error" => config.log_level = LevelFilter::Error,
                b"debug" => config.log_level = LevelFilter::Debug,
                b"trace" => config.log_level = LevelFilter::Trace,
                _ => continue,
            },
            b"log_source_path" => {
                config.log_source_path =
                    value == b"yes" || value == b"on" || value == b"1" || value == b"true"
            }
            b"wait_for_start" => {
                config.wait_for_start =
                    value == b"yes" || value == b"on" || value == b"1" || value == b"true"
            }
            b"walk_page_tables" => {
                config.walk_page_tables =
                    value == b"yes" || value == b"on" || value == b"1" || value == b"true"
            }
            b"revision" => log::trace!("Revision '{}'", unsafe {
                core::str::from_utf8_unchecked(value)
            }),
            b"watchdog_seconds" => {
                if let Ok(watchdog_seconds) =
                    core::str::from_utf8(value).unwrap_or_default().parse()
                {
                    config.watchdog_seconds = Some(watchdog_seconds);
                }
            }
            _ => continue,
        }
    }

    Some(config)
}

fn get_config() -> BootLoaderConfig {
    let mut config = BootLoaderConfig::default();
    if let Ok(fs_handle) = boot::get_handle_for_protocol::<SimpleFileSystem>() {
        if let Ok(mut fs) = boot::open_protocol_exclusive::<SimpleFileSystem>(fs_handle) {
            if let Ok(mut root_directory) = fs.open_volume() {
                if let Ok(file) =
                    root_directory.open(CORGOS_INI, FileMode::Read, FileAttribute::empty())
                {
                    if let Some(mut file) = file.into_regular_file() {
                        let mut buf = [0_u8; 4096];
                        let bytes_read: usize = file.read(&mut buf).unwrap_or_default();
                        if let Some(file_config) = parse_config(&buf[..bytes_read]) {
                            config = file_config;
                        }
                    }
                }
            }
        }
    }

    config
}

fn report_boot_processor_info() {
    #[cfg(target_arch = "x86_64")]
    {
        use raw_cpuid::CpuId;

        let cpuid = CpuId::new();
        let cpu_vendor = cpuid
            .get_vendor_info()
            .expect("Must be able to get CPU vendor");
        let brand_str = cpuid.get_processor_brand_string();

        log::info!(
            "Boot processor: {} {}",
            cpu_vendor.as_str(),
            if let Some(b) = &brand_str {
                b.as_str()
            } else {
                ""
            }
        );

        if let Some(hv_info) = cpuid.get_hypervisor_info() {
            log::info!("Hypervisor detected: {:?}", hv_info.identify());
        } else {
            log::info!("No hypervisor detected (wasn't trying too hard though)");
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        use crate::aarch64_regs::access::Aarch64Register;
        use crate::aarch64_regs::*;

        let regs = [
            register!(MainIdEl1),
            register!(ProcessorFeatures0El1),
            register!(ProcessorFeatures1El1),
            register!(MmFeatures0El1),
            register!(MmFeatures1El1),
            register!(MmFeatures2El1),
            register!(MmFeatures3El1),
            register!(MmFeatures4El1),
            register!(CurrentEl),
            register!(SystemControlEl1),
            register!(VectorBaseEl1),
            register!(MemoryAttributeIndirectionEl1),
            register!(TranslationControlEl1),
            register!(TranslationBase0El1),
            register!(TranslationBase1El1),
            register!(ExceptionLinkEl1),
            register!(ExceptionSyndromeEl1),
            register!(SavedProgramStateEl1),
        ];

        for r in regs {
            r.load();

            let raw: u64 = r.bits();
            let name = r.name();
            log::info!("{name}\t{raw:#016x?}: {r:x?}");
        }
    }
}

fn walk_page_tables() {
    #[cfg(target_arch = "aarch64")]
    {
        use crate::aarch64_regs::access::Aarch64Register;
        use crate::aarch64_regs::*;

        // Traverse page tables assuming 4K pages (check TCR!)

        let mut ttbr0_el1 = TranslationBase0El1::new();
        ttbr0_el1.load();

        let lvl4_table =
            unsafe { core::slice::from_raw_parts(ttbr0_el1.baddr() as *const PageTableEntry, 512) };

        let lvl3_table = unsafe {
            core::slice::from_raw_parts(
                (lvl4_table[0].next_table_pfn() << 12) as *const PageBlockEntry,
                512,
            )
        };

        log::info!("{:x?}", lvl3_table[5]);

        let mut dfs_stack = [(0u64, 0u64); 512];
        let mut dfs_stack_top = 0;
        dfs_stack[dfs_stack_top] = (0, ttbr0_el1.baddr() | 0b11);
        dfs_stack_top += 1;

        while dfs_stack_top > 0 {
            dfs_stack_top -= 1;
            let (level, entry) = dfs_stack[dfs_stack_top];

            if entry & 1 == 0 {
                // Not valid for hardware, skip. In general, might be valid when an OS is running
                // for software PTEs and swapping.
                continue;
            }

            assert!(entry & 0b11 == 0b11);

            // This a table pointer.
            let entry = PageTableEntry::from(entry);
            let entry_raw = u64::from(entry);
            log::info!("PTE {entry_raw:#x}: {entry:x?}");

            // Assuming 4K pages (check TCR!)
            let next_table_entries = unsafe {
                core::slice::from_raw_parts((entry.next_table_pfn() << 12) as *const u64, 512)
            };

            for &entry in next_table_entries.iter().rev() {
                if level >= 3 {
                    // This is a block pointer (a leaf).
                    let entry = PageBlockEntry::from(entry);
                    let entry_raw = u64::from(entry);
                    log::info!("PBE {entry_raw:#x}: {entry:x?}");
                    continue;
                }

                if entry & 0b11 == 0b11 {
                    dfs_stack[dfs_stack_top] = (level + 1, entry);
                    dfs_stack_top += 1;
                } else if entry & 1 == 1 {
                    // This is a block pointer (a leaf).
                    let entry = PageBlockEntry::from(entry);
                    let entry_raw = u64::from(entry);
                    log::info!("PBE {entry_raw:#x}: {entry:x?}");
                }
            }
        }
    }
}

fn report_uefi_info() {
    let fw_vendor = system::firmware_vendor();
    let fw_revision = system::firmware_revision();
    let uefi_revision = system::uefi_revision();
    log::info!(
        "Firmware {fw_vendor} {:x}.{:x}, UEFI revision {uefi_revision}",
        fw_revision >> 16,
        fw_revision as u16
    );
}

fn arch_name() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    {
        "x86_64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        "aarch64"
    }
}

fn boot_wait_for_key_press() {
    let stdin = if let Some(stdin) = boot::get_handle_for_protocol::<Input>().ok() {
        stdin
    } else {
        return;
    };
    let stdin = if let Some(stdin) = boot::open_protocol_exclusive::<Input>(stdin).ok() {
        stdin
    } else {
        return;
    };
    let event = if let Some(event) = stdin.wait_for_key_event() {
        event
    } else {
        return;
    };
    boot::wait_for_event(&mut [event]).ok();
}

#[allow(dead_code)]
fn reset() {
    runtime::reset(ResetType::WARM, Status::ABORTED, None);
}

#[allow(dead_code)]
fn dead_loop() -> ! {
    #[cfg(target_arch = "x86_64")]
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
    #[cfg(target_arch = "aarch64")]
    loop {
        unsafe {
            asm!("wfe", options(nomem, nostack));
        }
    }
}

// Write 0 to R9(X9) to break the loop.
fn wait_for_start() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!(
            r#"
                1:
                    cmpq    %r9, 0
                    pause
                    jne     1b
            "#,
            in("r9") 1,
            options(att_syntax, nostack),
        );
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!(
            r#"
            1:
                    cmp     x9, 0
                    yield
                    bne     1b
            "#,
            in("x9") 1,
            options(nostack),
        );
    }
}

#[cfg(target_os = "uefi")]
#[panic_handler]
fn panic(panic: &core::panic::PanicInfo<'_>) -> ! {
    log::error!("{panic}");

    let (_file_name_addr, _line_col) = if let Some(location) = panic.location() {
        (
            location.file().as_ptr() as u64,
            (location.line() as u64) | (location.column() as u64) << 32_u64,
        )
    } else {
        (0, 0)
    };

    // On the real hardware, the qemu exit shouldn't work.
    // Prob no harm.
    use qemu_exit::QEMUExit;

    #[cfg(target_arch = "x86_64")]
    #[allow(unreachable_code)]
    {
        let qemu_exit_handle = qemu_exit::X86::new(0xf4, 0xf);
        qemu_exit_handle.exit_failure();

        loop {
            unsafe {
                asm!("cli", options(nomem, nostack));
                asm!(
                    "hlt",
                    in("r8") CORGOS_BARF,
                    in("r9") _file_name_addr,
                    in("r10") _line_col,
                    options(att_syntax, nomem, nostack),
                );
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[allow(unreachable_code)]
    {
        // needs `-semihosting` on the qemu's command line.
        let qemu_exit_handle = qemu_exit::AArch64::new();
        qemu_exit_handle.exit_failure();

        loop {
            unsafe {
                asm!("wfe",
                    in("x0") CORGOS_BARF,
                    in("x1") _file_name_addr,
                    in("x2") _line_col,
                    options(nomem, nostack),
                );
            }
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
extern "efiapi" fn __chkstk() {}

#[uefi::entry]
fn main() -> Status {
    let config = get_config();
    if config.wait_for_start {
        wait_for_start();
    }
    boot_logger::setup_logger(&config);

    log::info!("Loading **CorgOS/{}**", arch_name());
    report_boot_processor_info();
    if config.walk_page_tables {
        walk_page_tables();
    }
    report_uefi_info();

    if let Some(watchdog_seconds) = config.watchdog_seconds {
        boot::set_watchdog_timer(watchdog_seconds, WATCHDOG_TIMEOUT_CODE, None).unwrap();
        log::info!(
            "Hit a key to exit loader, otherwise the system will reboot. Timeout {watchdog_seconds} seconds"
        );

        boot_wait_for_key_press();
        return Status::ABORTED;
    }

    let mut memory_map = unsafe { boot::exit_boot_services(MemoryType(0x70000000)) };

    memory_map.sort();
    log::info!("Memory map has {} entries", memory_map.entries().len());
    for entry in memory_map.entries() {
        log::info!("Memory map: {entry:x?}")
    }

    panic!("Could not load the system");
}
