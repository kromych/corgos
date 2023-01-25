#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use log::LevelFilter;
use raw_cpuid::CpuId;
use spinning_top::Spinlock;
use uefi::prelude::cstr16;
use uefi::prelude::Boot;
use uefi::prelude::BootServices;
use uefi::prelude::SystemTable;
use uefi::proto::console::serial::Serial;
use uefi::proto::console::text::Output;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::ScopedProtocol;
use uefi::CStr16;
use uefi::Handle;
use uefi::Status;

struct SyncSerialLogger<'a> {
    boot_system_table: SystemTable<Boot>,
    boot_services: &'a BootServices,
    serial_proto: Spinlock<ScopedProtocol<'a, Serial<'a>>>,
}

impl<'a> SyncSerialLogger<'a> {
    fn new(boot_system_table: &SystemTable<Boot>) -> Option<Self> {
        let mut boot_system_table = unsafe { boot_system_table.unsafe_clone() };
        let boot_services = boot_system_table.boot_services();
        if let Ok(serial_proto_handle) = boot_services.get_handle_for_protocol::<Serial>() {
            if let Ok(mut serial_proto) =
                boot_services.open_protocol_exclusive::<Serial>(serial_proto_handle)
            {
                serial_proto.reset();
                return Some(Self {
                    boot_system_table,
                    boot_services,
                    serial_proto: Spinlock::new(serial_proto),
                });
            }
        }
        None
    }
}

unsafe impl<'a> Send for SyncSerialLogger<'a> {}
unsafe impl<'a> Sync for SyncSerialLogger<'a> {}

impl<'a> log::Log for SyncSerialLogger<'a> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let serial_proto = self.serial_proto.lock();
        writeln!(
            serial_proto,
            "{:5}:{} {}",
            record.level(),
            record.target(),
            record.args()
        )
        .unwrap();
    }

    fn flush(&self) {}
}

struct SyncStdOut<'a> {
    boot_system_table: SystemTable<Boot>,
    boot_services: &'a BootServices,
    stdout: Spinlock<Output<'a>>,
}

impl<'a> SyncStdOut<'a> {
    fn new(boot_system_table: &SystemTable<Boot>) -> Self {
        let mut boot_system_table = unsafe { boot_system_table.unsafe_clone() };
        let boot_services = boot_system_table.boot_services();
        let stdout = boot_system_table.stdout();
        stdout.clear().ok();

        Self {
            boot_system_table,
            boot_services,
            stdout: Spinlock::new(*stdout),
        }
    }
}

unsafe impl<'a> Send for SyncStdOut<'a> {}
unsafe impl<'a> Sync for SyncStdOut<'a> {}

impl<'a> log::Log for SyncStdOut<'a> {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let stdout = self.stdout.lock();
        writeln!(stdout, "{:5}: {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}

enum LogDevice {
    StdOut,
    Serial,
}

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
                b"serial" => config.log_device = LogDevice::Serial,
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
            _ => continue,
        }
    }

    Some(config)
}

fn get_config(boot_system_table: &SystemTable<Boot>) -> LoaderConfig {
    let mut config = LoaderConfig {
        log_device: LogDevice::StdOut,
        log_level: LevelFilter::Info,
    };

    let mut boot_system_table_unsafe_clone = unsafe { boot_system_table.unsafe_clone() };
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

fn setup_logger(boot_system_table: &SystemTable<Boot>, config: LoaderConfig) {
    match config.log_device {
        LogDevice::StdOut => {
            log::set_logger(&SyncStdOut::new(boot_system_table));
        }
        LogDevice::Serial => {
            if let Some(serial) = SyncSerialLogger::new(boot_system_table) {
                log::set_logger(&serial);
            }
        }
    }

    log::set_max_level(config.log_level);
}

#[no_mangle]
extern "efiapi" fn uefi_main(image_handle: Handle, boot_system_table: SystemTable<Boot>) -> Status {
    //#[cfg(target_arch = "x86_64")]
    //wait_for_start();

    let config = get_config(&boot_system_table);
    setup_logger(&boot_system_table, config);

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

    let mut mmap_buf = [0_u8; 4096];
    let (_runtime_system_table, _memory_map) = boot_system_table
        .exit_boot_services(image_handle, &mut mmap_buf)
        .unwrap();

    panic!();
}

#[cfg(target_arch = "x86_64")]
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
    let (file_name_addr, line_col) = if let Some(location) = panic.location() {
        (
            location.file().as_ptr() as u64,
            (location.line() as u64) | (location.column() as u64) << 32_u64,
        )
    } else {
        (0, 0)
    };

    loop {
        unsafe {
            asm!("cli", options(nomem, nostack));
            asm!(
                "hlt",
                in("r8") CORGOS_BARF,
                in("r9") file_name_addr,
                in("r10") line_col,
                options(att_syntax, nomem, nostack),
            );
        }
    }
}

#[no_mangle]
extern "efiapi" fn __chkstk() {}
