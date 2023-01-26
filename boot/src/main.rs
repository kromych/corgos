#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use conquer_once::spin::OnceCell;
use log::LevelFilter;
use qemu_exit::QEMUExit;
use raw_cpuid::CpuId;
use spinning_top::Spinlock;
use uart_16550::SerialPort;
use uefi::prelude::cstr16;
use uefi::prelude::Boot;
use uefi::prelude::SystemTable;
use uefi::proto::console::text::Output;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::runtime::ResetType;
use uefi::CStr16;
use uefi::Handle;
use uefi::Status;

enum BootLoggerOutput {
    None,
    Stdout,
    Serial,
}

struct SyncBootLogger {
    stdout: Option<Spinlock<*mut Output<'static>>>,
    serial: Option<Spinlock<SerialPort>>,
    output: BootLoggerOutput,
}

impl SyncBootLogger {
    fn new() -> Self {
        Self {
            stdout: None,
            serial: None,
            output: BootLoggerOutput::None,
        }
    }

    fn log_to_stdout(&mut self, boot_system_table: &mut SystemTable<Boot>) {
        // TODO: rework this barf
        boot_system_table.stdout().clear().ok();
        let stdout = boot_system_table.stdout() as *mut Output as u64;
        let stdout = stdout as *mut Output;

        self.stdout = Some(Spinlock::new(stdout));
        self.output = BootLoggerOutput::Stdout;
    }

    fn log_to_serial(&mut self, port: u16) {
        let mut serial_port = unsafe { SerialPort::new(port) };
        serial_port.init();

        self.serial = Some(Spinlock::new(unsafe { SerialPort::new(port) }));
        self.output = BootLoggerOutput::Serial;
    }
}

unsafe impl Send for SyncBootLogger {}
unsafe impl Sync for SyncBootLogger {}

impl log::Log for SyncBootLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        match self.output {
            BootLoggerOutput::None => {}
            BootLoggerOutput::Stdout => {
                if let Some(stdout) = &self.stdout {
                    let stdout = stdout.lock();
                    let stdout = unsafe { stdout.as_mut().unwrap() };
                    writeln!(
                        stdout,
                        "{:7} {}:{}@{}  {}",
                        record.level(),
                        record.module_path().unwrap_or_default(),
                        record.file().unwrap_or_default(),
                        record.line().unwrap_or_default(),
                        record.args()
                    )
                    .ok();
                }
            }
            BootLoggerOutput::Serial => {
                if let Some(serial_port) = &self.serial {
                    let mut serial_port = serial_port.lock();
                    writeln!(
                        serial_port,
                        "{:7} {}:{}@{}  {}",
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
    }

    fn flush(&self) {}
}

#[derive(Debug, Clone)]
enum LogDevice {
    StdOut,
    Com1,
    Com2,
}

#[derive(Debug, Clone)]
struct LoaderConfig {
    log_device: LogDevice,
    log_level: LevelFilter,
}

/// The name of the configuration file in the ESP partition alongside the loader.
const CORGOS_INI: &CStr16 = cstr16!("corgos-boot.ini");

/// Upon panic, b"CORGBARF" is loaded into R8. R9 contains the address of the file name,
/// R10 contains the line number in the least significant 32 bits, and the column number
/// in the most significant 32 bits.
/// The interrupts are disabled and the processor is halted.
const CORGOS_BARF: u64 = u64::from_le_bytes([0x46, 0x52, 0x41, 0x42, 0x47, 0x52, 0x4f, 0x43]);

fn parse_config(bytes: &[u8]) -> Option<LoaderConfig> {
    let mut config = LoaderConfig {
        log_device: LogDevice::StdOut,
        log_level: LevelFilter::Info,
    };
    let mut parser = corg_ini::Parser::new(bytes);

    while let Ok(Some(corg_ini::KeyValue { key, value })) = parser.parse() {
        match key {
            b"log_device" => match value {
                b"com1" => config.log_device = LogDevice::Com1,
                b"com2" => config.log_device = LogDevice::Com2,
                b"stdout" => config.log_device = LogDevice::StdOut,
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
            b"revision" => log::trace!("Revision '{}'", unsafe {
                core::str::from_utf8_unchecked(value)
            }),
            _ => continue,
        }
    }

    Some(config)
}

fn get_config(boot_system_table: &SystemTable<Boot>) -> LoaderConfig {
    let mut config = LoaderConfig {
        log_device: LogDevice::StdOut,
        log_level: LevelFilter::Trace,
    };

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

    log::trace!("{config:?}");

    config
}

static BOOT_LOGGER: OnceCell<SyncBootLogger> = OnceCell::uninit();

fn setup_logger(boot_system_table: &mut SystemTable<Boot>, config: LoaderConfig) {
    let logger = BOOT_LOGGER.get_or_init(move || {
        let mut logger = SyncBootLogger::new();
        match config.log_device {
            LogDevice::StdOut => logger.log_to_stdout(boot_system_table),
            LogDevice::Com1 => logger.log_to_serial(0x03f8),
            LogDevice::Com2 => logger.log_to_serial(0x02f8),
        };
        logger
    });

    log::set_logger(logger).unwrap();
    log::set_max_level(config.log_level);
}

#[no_mangle]
extern "efiapi" fn uefi_main(
    image_handle: Handle,
    mut boot_system_table: SystemTable<Boot>,
) -> Status {
    unsafe {
        boot_system_table
            .boot_services()
            .set_image_handle(image_handle);
    }

    //#[cfg(target_arch = "x86_64")]
    //wait_for_start();
    let config = get_config(&boot_system_table);
    setup_logger(&mut boot_system_table, config);

    let cpuid = CpuId::new();
    let cpu_vendor = cpuid
        .get_vendor_info()
        .expect("Must be able to get CPU vendor");
    let brand_str = cpuid.get_processor_brand_string();

    log::info!(
        "Loading **CorgOS** on {} {}",
        cpu_vendor.as_str(),
        if let Some(b) = &brand_str {
            b.as_str()
        } else {
            ""
        }
    );

    if let Some(hv_info) = cpuid.get_hypervisor_info() {
        log::info!("Hypervisor detected: {:?}", hv_info.identify());
    }

    let fw_vendor = boot_system_table.firmware_vendor();
    let fw_revision = boot_system_table.firmware_revision();
    let uefi_revision = boot_system_table.uefi_revision();
    log::info!(
        "Firmware {fw_vendor} {:x}.{:x}, UEFI revision {uefi_revision}",
        fw_revision >> 16,
        fw_revision as u16
    );

    log::info!("Exiting boot services; hit a key to reboot");
    boot_wait_for_key_press(&mut boot_system_table);

    let mut mmap_buf = [0_u8; 8192];
    let (runtime_system_table, _memory_map) = boot_system_table
        .exit_boot_services(image_handle, &mut mmap_buf)
        .unwrap();

    unsafe {
        runtime_system_table
            .runtime_services()
            .reset(ResetType::Warm, Status::ABORTED, None);
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

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
fn dead_loop() -> ! {
    loop {
        unsafe {
            asm!("cli", "hlt", options(nomem, nostack));
        }
    }
}

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
// Write 0 to R9 to break the loop.
fn wait_for_start() {
    unsafe {
        asm!(
            "1:     cmpq  %r9, 0",
            "       pause",
            "       jne 1b",
            in("r9") 1,
            options(att_syntax, nostack),
        );
    }
}

#[panic_handler]
#[cfg(target_arch = "x86_64")]
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

    let qemu_exit_handle = qemu_exit::X86::new(0xf4, 0xf);
    qemu_exit_handle.exit(_line_col as u32);

    // On the real hardware, the qmeu exit shouldn't work.
    // Prob no harm.
    #[allow(unreachable_code)]
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

#[no_mangle]
extern "efiapi" fn __chkstk() {}
