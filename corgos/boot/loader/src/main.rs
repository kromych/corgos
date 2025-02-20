#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_arch = "aarch64")]
mod aarch64_regs;

use boot_logger::BootLoaderConfig;
use boot_logger::LogDevice;
use core::arch::asm;
use core::num::NonZero;
use elf::endian::LittleEndian;
use elf::ElfBytes;
use log::LevelFilter;
use page_bitmap::DefaultPageBitmap;
use page_bitmap::PageFrameNumber;
use page_bitmap::PageRange;
use uefi::boot;
use uefi::boot::AllocateType;
use uefi::mem::memory_map::MemoryMap;
use uefi::mem::memory_map::MemoryMapMut;
use uefi::mem::memory_map::MemoryType;
use uefi::proto::console::text::Input;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileInfo;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::runtime;
use uefi::runtime::ResetType;
use uefi::system;
use uefi::CStr16;
use uefi::Status;

const CORGOS_MAX_MEMORY_BYTES: usize = 64 << 30; // 64 GiB
const RESERVED_FOR_OS_LOADER_MEMORY_TYPE: u32 = 0x8000_0000;
const CORGOS_KERNEL_IMAGE_MEMORY_TYPE: u32 = RESERVED_FOR_OS_LOADER_MEMORY_TYPE;
const CORGOS_MEMORY_MAP_MEMORY_TYPE: u32 = RESERVED_FOR_OS_LOADER_MEMORY_TYPE + 1;
const CORGOS_PAGE_BITMAP_MEMORY_TYPE: u32 = RESERVED_FOR_OS_LOADER_MEMORY_TYPE + 2;

/// The name of the configuration file in the ESP partition alongside the loader.
#[cfg(target_arch = "x86_64")]
const CORGOS_INI: &CStr16 = uefi::cstr16!("corgos-boot-x86_64.ini");
#[cfg(target_arch = "aarch64")]
const CORGOS_INI: &CStr16 = uefi::cstr16!("corgos-boot-aarch64.ini");

/// The name of the CorgOS kernel binary image.
const CORGOS_KERNEL: &CStr16 = uefi::cstr16!("corgos");

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
            b"revision" => {
                let len = core::cmp::min(value.len(), config.revision.len());
                config.revision[..len].copy_from_slice(&value[..len])
            }
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

    let table_count = system::with_config_table(|tables| tables.len());
    log::info!(
        "Reported {table_count} configuration tables, known {}",
        uefi_guids::get_uefi_known_guids_count()
    );

    let rsdp = system::with_config_table(|tables| {
        let mut rsdp_addr: Option<*const core::ffi::c_void> = None;
        for table in tables {
            let name = uefi_guids::get_uefi_table_name(&table.guid);
            log::info!(
                "Table {} @ {:#016x}: {name}",
                table.guid,
                table.address as u64
            );
            if table.guid == uefi_guids::EFI_ACPI20_TABLE_GUID {
                rsdp_addr = Some(table.address);
            }
        }
        rsdp_addr
    })
    .expect("Must be able to locate ACPI 2.0 FADT");

    let rsdp: *const acpi::rsdp::Rsdp = rsdp.cast();
    let rsdp = unsafe {
        rsdp.as_ref()
            .expect("Must be a non-NULL point to ACPI 2.0 RSDP")
    };
    rsdp.validate().expect("Must have a valid ACPI 2.0 RSDP");
    assert!(rsdp.revision() == 2, "Expected ACPI 2.0 RSDP");

    log::info!("ACPI 2.0 RSDP {rsdp:x?}");
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

fn load_kernel_from_elf() {
    let sfs = boot::get_handle_for_protocol::<SimpleFileSystem>()
        .expect("SimpleFileSystem must be available");
    let mut sfs = boot::open_protocol_exclusive::<SimpleFileSystem>(sfs)
        .expect("SimpleFileSystem must be opened");
    let mut root = sfs.open_volume().expect("Failed to open root volume");

    let kernel_file = root
        .open(CORGOS_KERNEL, FileMode::Read, FileAttribute::empty())
        .expect("Failed to open kernel image");

    let mut kernel_file = kernel_file
        .into_regular_file()
        .expect("Failed to convert to a regular file");

    let elf_data_size = {
        let mut file_info_buf = [0u8; 512];
        let file_info = kernel_file
            .get_info::<FileInfo>(&mut file_info_buf)
            .expect("Failed to get file info");

        let file_size = file_info.file_size() as usize;
        (file_size as usize + 0xFFF) & !0xFFF
    };
    assert!(elf_data_size & 0xFFF == 0);

    log::info!("Kernel file size {elf_data_size} bytes, rounded up to 4KiB");

    let elf_data = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        elf_data_size / 0x1000,
    )
    .expect("Failed to allocate pages to read the kernel image")
    .as_ptr();
    let elf_data = unsafe { core::slice::from_raw_parts_mut(elf_data, elf_data_size) };

    kernel_file
        .read(elf_data)
        .expect("Cannot read the kernel image");
    // Downgrade to immutable.
    let elf_data = &elf_data[..elf_data_size];

    let elf = ElfBytes::<LittleEndian>::minimal_parse(elf_data)
        .expect("Cannot parse the kernel image as ELF");

    #[cfg(target_arch = "aarch64")]
    assert!(
        elf.ehdr.e_machine == elf::abi::EM_AARCH64,
        "Wrong kernel target arch, expected aarch64"
    );

    #[cfg(target_arch = "x86_64")]
    assert!(
        elf.ehdr.e_machine == elf::abi::EM_X86_64,
        "Wrong kernel target arch, expected x86_64"
    );

    let mut loaded_size = 0;
    let segments = elf
        .segments()
        .expect("Cannot find segments in the ELF file");

    // First pass: see how much data is going to be loaded.
    for ph in segments {
        log::info!(
            "Found segment of {} bytes ({} in the image), PA: {:#016x}, VA: {:#016x}",
            ph.p_memsz,
            ph.p_filesz,
            ph.p_paddr,
            ph.p_vaddr
        );

        if ph.p_type != elf::abi::PT_LOAD {
            continue;
        }
        loaded_size += (ph.p_memsz + 0xFFF) & !0xFFF;

        log::info!("Will load the segment");
    }

    assert!(loaded_size & 0xFFF == 0);

    log::info!("Loaded image size will be {loaded_size} bytes, rounded up to 4KiB");

    let loaded_data = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::custom(CORGOS_KERNEL_IMAGE_MEMORY_TYPE),
        (loaded_size / 0x1000) as usize,
    )
    .expect("Failed to allocate pages")
    .as_ptr();
    let _loaded_data =
        unsafe { core::slice::from_raw_parts_mut(loaded_data, loaded_size as usize) };

    // Second pass: load the code and data.
    let mut _bytes_loaded = 0;
    for ph in segments {
        if ph.p_type != elf::abi::PT_LOAD {
            continue;
        }
        log::info!(
            "Loading segment of {} bytes ({} in the image), PA: {:#016x}, VA: {:#016x}",
            ph.p_memsz,
            ph.p_filesz,
            ph.p_paddr,
            ph.p_vaddr
        );

        // TODO: copy, round up to a page.

        // if ph.p_filesz != 0 {
        //     // Copy segment data to the allocated memory
        //     let src_data = &elf_data[ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize];
        //     dst.copy_from_slice(src_data);
        // } else {
        //     // If memory size is greater than file size, zero out the rest (clean BSS)
        //     let zeroed_region = unsafe {
        //         core::slice::from_raw_parts_mut(segment_address as *mut u8, ph.p_memsz as usize)
        //     };
        //     zeroed_region.fill(0);
        // }
    }

    log::info!("Kernel entry point: {:#016x}", elf.ehdr.e_entry);
}

#[cfg_attr(target_os = "uefi", panic_handler)]
#[cfg_attr(not(target_os = "uefi"), allow(dead_code))]
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

    {
        // Needs `-semihosting` or `isa-debug-exit` on the qemu's command line.
        let smh = semihosting::Semihosting;

        // TODO: Might be divergent or cause a hardware failure.
        // TODO: detect if running under QEMU.
        smh.exit_host_failure();
        log::error!("Hit `Ctrl+A X` if running under QEMU, and it is not exiting");
    }

    #[cfg(target_arch = "x86_64")]
    #[allow(unreachable_code)]
    {
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

    log::info!(
        "Loading **CorgOS/{}**, \"{}\"",
        arch_name(),
        config.revision_str()
    );
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

    load_kernel_from_elf();

    // Allocate space for the page bitmap before exiting boot services
    let bitmap_size = page_bitmap::DefaultPageBitmap::bitmap_storage_size(CORGOS_MAX_MEMORY_BYTES);
    let alloc_bitmap = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::custom(CORGOS_PAGE_BITMAP_MEMORY_TYPE),
        (bitmap_size + 0xFFF) / 0x1000,
    )
    .expect("Failed to allocate pages for the page bitmap")
    .as_ptr() as *mut u64;
    let reserved_bitmap = boot::allocate_pages(
        AllocateType::AnyPages,
        MemoryType::RESERVED,
        (bitmap_size + 0xFFF) / 0x1000,
    )
    .expect("Failed to allocate pages for the page bitmap")
    .as_ptr() as *mut u64;

    let mut memory_map =
        unsafe { boot::exit_boot_services(MemoryType::custom(CORGOS_MEMORY_MAP_MEMORY_TYPE)) };
    memory_map.sort();
    log::info!("Memory map has {} entries", memory_map.entries().len());
    let mut total_memory = 0;
    let mut available_memory = 0;
    for entry in memory_map.entries() {
        log::info!("Memory map: {entry:x?}");
        total_memory += entry.page_count * 4096;
        if entry.ty == MemoryType::CONVENTIONAL {
            available_memory += entry.page_count * 4096;
        }
    }
    log::info!(
        "Total memory: {} bytes, available memory: {} bytes",
        total_memory,
        available_memory
    );

    log::info!(
        "Page bitmap size: {} bytes, {} pages",
        bitmap_size,
        bitmap_size / 4096
    );
    let mut memmap_iter = memory_map.entries().map(|entry| {
        PageRange::new(
            PageFrameNumber::new(entry.phys_start as usize / 4096),
            NonZero::new(entry.page_count as usize).unwrap(),
        )
    });
    let page_bitmap = DefaultPageBitmap::new(
        CORGOS_MAX_MEMORY_BYTES,
        [alloc_bitmap, reserved_bitmap],
        || memmap_iter.next(),
    );
    log::info!(
        "Page bitmap tracks {} available pages",
        page_bitmap.available_pages()
    );

    todo!("Map the kernel code and data approriately");
    // todo!("Transfer to the kernel");
}
