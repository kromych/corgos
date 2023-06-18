#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use conquer_once::spin::OnceCell;
use corg_uart::BaudDivisor;
use corg_uart::ComPort;
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

enum LogOutput {
    Stdout(*mut Output),
    Com(ComPort),
    Pl(Pl011),
}

/// Single-thread logger
struct BootLogger(Option<LogOutput>);

impl BootLogger {
    fn new() -> Self {
        Self(None)
    }

    fn log_to_stdout(&mut self, boot_system_table: &mut SystemTable<Boot>) {
        // TODO: rework this barf
        boot_system_table.stdout().clear().ok();
        let stdout = boot_system_table.stdout() as *mut Output as u64;
        self.0 = Some(stdout as *mut Output);
    }

    fn log_to_com_port(&mut self, port: ComPort, baud: BaudDivisor) {
        self.0 = Some(ComPort::new(port, baud));
    }

    fn log_to_pl011(&mut self, base_addr: u64) {
        self.0 = Some(Pl011::new(base_addr));
    }
}

unsafe impl Send for BootLogger {}
unsafe impl Sync for BootLogger {}

impl log::Log for BootLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        match self {
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
            Some(LogOutput::Com(serial_port)) => {
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
            Some(LogOutput::Pl(pl011_dev)) => {
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
}

impl Default for BootLoaderConfig {
    fn default() -> Self {
        Self {
            log_device: LogDevice::StdOut,
            log_level: LevelFilter::Trace,
            wait_for_start: false,
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
const WATCHDOG_TIMEOUT_SECONDS: usize = 15;
const WATCHDOG_TIMEOUT_CODE: u64 = CORGOS_BARF;

fn parse_config(bytes: &[u8]) -> Option<BootLoaderConfig> {
    let mut config = BootLoaderConfig::default();
    let mut parser = corg_ini::Parser::new(bytes);

    while let Ok(Some(corg_ini::KeyValue { key, value })) = parser.parse() {
        match key {
            b"log_device" => match value {
                b"com1" => config.log_device = LogDevice::Com1,
                b"com2" => config.log_device = LogDevice::Com2,
                b"stdout" => config.log_device = LogDevice::StdOut,
                b"pl011" => config.log_device = LogDevice::Pl011(0x9000000), // TODO: parse base addr
                _ => continue,
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
    let logger = BOOT_LOGGER.get_or_init(move || {
        let mut logger = BootLogger::new();
        match config.log_device {
            LogDevice::StdOut => logger.log_to_stdout(boot_system_table),
            LogDevice::Com1 => {
                #[cfg(target_arch = "x86_64")]
                {
                    logger.log_to_com_port(ComPort::Com1, BaudDivisor::Baud115200)
                }

                #[cfg(target_arch = "aarch64")]
                {
                    logger.log_to_stdout(boot_system_table)
                }
            }

            LogDevice::Com2 => {
                #[cfg(target_arch = "x86_64")]
                {
                    logger.log_to_com_port(ComPort::Com2, BaudDivisor::Baud115200)
                }

                #[cfg(target_arch = "aarch64")]
                {
                    logger.log_to_stdout(boot_system_table)
                }
            }

            LogDevice::Pl011(base_addr) => {
                #[cfg(target_arch = "x86_64")]
                {
                    logger.log_to_stdout(boot_system_table)
                }

                #[cfg(target_arch = "aarch64")]
                {
                    logger.log_to_pl011(base_addr)
                }
            }
        };

        logger
    });

    log::set_logger(logger).unwrap();
    log::set_max_level(config.log_level);

    log::trace!("{config:?}");
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

    boot_system_table
        .boot_services()
        .set_watchdog_timer(WATCHDOG_TIMEOUT_SECONDS, WATCHDOG_TIMEOUT_CODE, None)
        .unwrap();
    log::info!(
        "Exiting boot services; hit a key to reboot. Timeout {WATCHDOG_TIMEOUT_SECONDS} seconds"
    );
    boot_wait_for_key_press(&mut boot_system_table);

    let (runtime_system_table, _memory_map) = boot_system_table.exit_boot_services();

    reset(runtime_system_table);

    Status::LOAD_ERROR
}
