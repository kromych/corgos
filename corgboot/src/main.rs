#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(target_arch = "aarch64")]
mod aarch64_regs;

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use conquer_once::spin::OnceCell;
use corg_uart::BaudDivisor;
use corg_uart::ComPort;
use corg_uart::ComPortIo;
use corg_uart::Pl011;
use log::LevelFilter;
use uefi::entry;
use uefi::prelude::*;
use uefi::proto::console::text::Output;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::runtime::ResetType;
use uefi::table::Runtime;
use uefi::CStr16;
use uefi::Handle;
use uefi::Status;

#[allow(dead_code)]
#[derive(Debug)]
enum LogOutput {
    Stdout(*mut Output),
    Com(ComPort),
    Pl(Pl011),
}

/// Single-thread logger
#[derive(Debug)]
struct BootLogger(Option<LogOutput>);

unsafe impl Send for BootLogger {}
unsafe impl Sync for BootLogger {}

impl log::Log for BootLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        match &self.0 {
            None => {}
            Some(LogOutput::Stdout(stdout)) => {
                let stdout = unsafe { stdout.as_mut().unwrap() };
                writeln!(
                    stdout,
                    "[{:7}][{}:{}@{}]  {}",
                    record.level(),
                    record.module_path().unwrap_or_default(),
                    record.file().unwrap_or_default(),
                    record.line().unwrap_or_default(),
                    record.args()
                )
                .ok();
            }
            Some(LogOutput::Com(mut serial_port)) => {
                write!(
                    serial_port,
                    "[{:7}][{}:{}@{}]  {}\r\n",
                    record.level(),
                    record.module_path().unwrap_or_default(),
                    record.file().unwrap_or_default(),
                    record.line().unwrap_or_default(),
                    record.args()
                )
                .ok();
            }
            Some(LogOutput::Pl(mut pl011_dev)) => {
                write!(
                    pl011_dev,
                    "[{:7}][{}:{}@{}]  {}\r\n",
                    record.level(),
                    record.module_path().unwrap_or_default(),
                    record.file().unwrap_or_default(),
                    record.line().unwrap_or_default(),
                    record.args()
                )
                .ok();
            }
        }
    }

    fn flush(&self) {}
}

#[derive(Debug, Clone)]
enum LogDevice {
    Null,
    StdOut,
    Com1,
    Com2,
    Pl011(u64),
}

#[derive(Debug, Clone)]
struct BootLoaderConfig {
    log_device: LogDevice,
    log_level: LevelFilter,
    wait_for_start: bool,
    watchdog_seconds: Option<usize>,
}

impl Default for BootLoaderConfig {
    fn default() -> Self {
        Self {
            log_device: LogDevice::StdOut,
            log_level: LevelFilter::Trace,
            wait_for_start: false,
            watchdog_seconds: None,
        }
    }
}

/// The name of the configuration file in the ESP partition alongside the loader.
#[cfg(target_arch = "x86_64")]
const CORGOS_INI: &CStr16 = cstr16!("corgos-boot-x86_64.ini");
#[cfg(target_arch = "aarch64")]
const CORGOS_INI: &CStr16 = cstr16!("corgos-boot-aarch64.ini");

/// Upon panic, b"CORGBARF" is loaded into R8. R9 contains the address of the file name,
/// R10 contains the line number in the least significant 32 bits, and the column number
/// in the most significant 32 bits.
/// The interrupts are disabled and the processor is halted.
const CORGOS_BARF: u64 = u64::from_le_bytes([0x46, 0x52, 0x41, 0x42, 0x47, 0x52, 0x4f, 0x43]);

/// Timeout for the boot services.
const WATCHDOG_TIMEOUT_CODE: u64 = CORGOS_BARF;

fn parse_config(bytes: &[u8]) -> Option<BootLoaderConfig> {
    let mut config = BootLoaderConfig::default();
    let mut parser = corg_ini::Parser::new(bytes);

    while let Ok(Some(corg_ini::KeyValue { key, value })) = parser.parse() {
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
            b"wait_for_start" => {
                config.wait_for_start = value == b"yes" || value == b"on" || value == b"1"
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

fn get_config(boot_system_table: &SystemTable<Boot>) -> BootLoaderConfig {
    let mut config = BootLoaderConfig::default();

    let boot_system_table_unsafe_clone = unsafe { boot_system_table.unsafe_clone() };
    let boot_services = boot_system_table_unsafe_clone.boot_services();
    if let Ok(fs_handle) = boot_services.get_handle_for_protocol::<SimpleFileSystem>() {
        if let Ok(mut fs) = boot_services.open_protocol_exclusive::<SimpleFileSystem>(fs_handle) {
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

static BOOT_LOGGER: OnceCell<BootLogger> = OnceCell::uninit();

fn setup_logger(boot_system_table: &mut SystemTable<Boot>, config: &BootLoaderConfig) {
    let mut stdout_logger = || {
        // TODO: rework this barf
        boot_system_table.stdout().clear().ok();
        let stdout = boot_system_table.stdout() as *mut Output as u64;
        Some(LogOutput::Stdout(stdout as *mut Output))
    };

    let logger = BOOT_LOGGER.get_or_init(move || {
        let output = match config.log_device {
            LogDevice::StdOut => stdout_logger(),
            LogDevice::Com1 => {
                if cfg!(target_arch = "x86_64") {
                    Some(LogOutput::Com(ComPort::new(
                        ComPortIo::Com1,
                        BaudDivisor::Baud115200,
                    )))
                } else {
                    stdout_logger()
                }
            }
            LogDevice::Com2 => {
                if cfg!(target_arch = "x86_64") {
                    Some(LogOutput::Com(ComPort::new(
                        ComPortIo::Com2,
                        BaudDivisor::Baud115200,
                    )))
                } else {
                    stdout_logger()
                }
            }
            LogDevice::Pl011(base_addr) => {
                if cfg!(target_arch = "aarch64") {
                    Some(LogOutput::Pl(Pl011::new(base_addr)))
                } else {
                    stdout_logger()
                }
            }
            LogDevice::Null => None,
        };

        BootLogger(output)
    });

    log::set_logger(logger).unwrap();
    log::set_max_level(config.log_level);

    log::trace!("{config:x?}, {logger:x?}");
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
        use crate::aarch64_regs::*;
        use aarch64_cpu::registers::*;
        use tock_registers::interfaces::Readable;

        let current_el_raw = CurrentEL.get();
        let sctlr_el1_raw = SCTLR_EL1.get();
        let vbar_el1_raw = VBAR_EL1.get();
        let mair_el1_raw = MAIR_EL1.get();
        let tcr_el1_raw = TCR_EL1.get();
        let ttbr0_el1_raw = TTBR0_EL1.get();
        let ttbr1_el1_raw = TTBR1_EL1.get();
        let id_aa64mmfr0_el1_raw = ID_AA64MMFR0_EL1.get();
        let elr_el1_raw = ELR_EL1.get();
        let esr_el1_raw = ESR_EL1.get();
        let spsr_el1_raw = SPSR_EL1.get();

        let current_el = CurrentElVal::from(current_el_raw).el();
        let sctlr_el1 = SystemControlEl1Val::from(sctlr_el1_raw);
        let vbar_el1 = VectorBaseEl1Val::from(vbar_el1_raw).vbar();
        let mair_el1 = MemoryAttributeIndirectionEl1Val::from(mair_el1_raw);
        let tcr_el1 = TranslationControlEl1Val::from(tcr_el1_raw);
        let ttbr0_el1 = TranslationBaseEl1Val::from(ttbr0_el1_raw);
        let ttbr1_el1 = TranslationBaseEl1Val::from(ttbr1_el1_raw);
        let id_aa64mmfr0_el1 = MmuFeatures0El1Val::from(id_aa64mmfr0_el1_raw);
        let spsr_el1 = SavedProgramState::from(spsr_el1_raw);

        log::info!("CurrentEL\t{current_el_raw:#016x?}: {current_el:?}");
        log::info!("SCTLR_EL1\t{sctlr_el1_raw:#016x?}: {sctlr_el1:?}");
        log::info!("VBAR_EL1\t{vbar_el1_raw:#016x?}: {vbar_el1:#x?}");
        log::info!("MAIR_EL1\t{mair_el1_raw:#016x?}: {mair_el1:x?}");
        log::info!("TCR_EL1\t{tcr_el1_raw:#016x?}: {tcr_el1:?}");
        log::info!("TTBR0_EL1\t{ttbr0_el1_raw:#016x?}: {ttbr0_el1:x?}");
        log::info!("TTBR1_EL1\t{ttbr1_el1_raw:#016x?}: {ttbr1_el1:x?}");
        log::info!("AA64MMFR0_EL1\t{id_aa64mmfr0_el1_raw:#016x?}: {id_aa64mmfr0_el1:?}");
        log::info!("ELR_EL1\t{elr_el1_raw:#016x?}");
        log::info!("ESR_EL1\t{esr_el1_raw:#016x?}");
        log::info!("SPSR_EL1\t{spsr_el1_raw:#016x?}: {spsr_el1:?}");

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
            log::info!("{entry:x?}");

            // Assuming 4K pages (check TCR!)
            let next_table_entries = unsafe {
                core::slice::from_raw_parts((entry.next_table_pfn() << 12) as *const u64, 512)
            };

            for &entry in next_table_entries.iter().rev() {
                if level >= 3 {
                    // This is a block pointer (a leaf).
                    let entry = PageBlockEntry::from(entry);
                    log::info!("{entry:x?}");
                    continue;
                }

                if entry & 0b11 == 0b11 {
                    dfs_stack[dfs_stack_top] = (level + 1, entry);
                    dfs_stack_top += 1;
                } else if entry & 1 == 1 {
                    // This is a block pointer (a leaf).
                    let entry = PageBlockEntry::from(entry);
                    log::info!("{entry:x?}");
                }
            }
        }
    }
}

fn report_uefi_info(boot_system_table: &SystemTable<Boot>) {
    let fw_vendor = boot_system_table.firmware_vendor();
    let fw_revision = boot_system_table.firmware_revision();
    let uefi_revision = boot_system_table.uefi_revision();
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

fn boot_wait_for_key_press(boot_system_table: &mut SystemTable<Boot>) {
    unsafe {
        let mut boot_system_table_key_event = boot_system_table.unsafe_clone();
        let key_event = boot_system_table_key_event.stdin().wait_for_key_event();
        {
            let boot_system_table_wait_event = boot_system_table.unsafe_clone();
            boot_system_table_wait_event
                .boot_services()
                .wait_for_event(&mut [key_event.unsafe_clone()])
                .ok();
        }
    }
}

#[allow(dead_code)]
fn reset(runtime_system_table: SystemTable<Runtime>) {
    unsafe {
        runtime_system_table
            .runtime_services()
            .reset(ResetType::WARM, Status::ABORTED, None);
    }
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

#[panic_handler]
fn panic(panic: &PanicInfo<'_>) -> ! {
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
        qemu_exit_handle.exit(_line_col as u32);

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
        qemu_exit_handle.exit(_line_col as u32);

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

#[no_mangle]
extern "efiapi" fn __chkstk() {}

#[entry]
fn main(image_handle: Handle, mut boot_system_table: SystemTable<Boot>) -> Status {
    let config = get_config(&boot_system_table);
    if config.wait_for_start {
        wait_for_start();
    }
    setup_logger(&mut boot_system_table, &config);

    log::info!("Loading **CorgOS/{}**", arch_name());
    report_boot_processor_info();
    report_uefi_info(&boot_system_table);

    if let Some(watchdog_seconds) = config.watchdog_seconds {
        boot_system_table
            .boot_services()
            .set_watchdog_timer(watchdog_seconds, WATCHDOG_TIMEOUT_CODE, None)
            .unwrap();
        log::info!(
            "Hit a key to exit loader, otherwise the system will reboot. Timeout {watchdog_seconds} seconds"
        );

        boot_wait_for_key_press(&mut boot_system_table);
        return Status::ABORTED;
    }

    let (_runtime_system_table, mut memory_map) = boot_system_table.exit_boot_services();

    memory_map.sort();
    log::info!("Memory map has {} entries", memory_map.entries().len());
    for entry in memory_map.entries() {
        log::info!("Memory map: {entry:x?}")
    }

    panic!("Could not load the system");
}
