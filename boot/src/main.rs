#![no_std]
#![no_main]
#![deny(unsafe_op_in_unsafe_fn)]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;

use log::LevelFilter;
use raw_cpuid::CpuId;
use uefi::prelude::cstr16;
use uefi::prelude::Boot;
use uefi::prelude::BootServices;
use uefi::prelude::SystemTable;
use uefi::proto::console::serial::Serial;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::CStr16;

struct SerialLogger {}

impl log::Log for SerialLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, _record: &log::Record) {
        //writeln!(serial, "{:5}: {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}

struct StdOutLogger {}

impl log::Log for StdOutLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, _record: &log::Record) {
        //writeln!(stdout, "{:5}: {}", record.level(), record.args()).unwrap();
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

fn parse_config(_bytes: &[u8]) -> Option<LoaderConfig> {
    Some(LoaderConfig {
        log_device: LogDevice::StdOut,
        log_level: LevelFilter::Info,
    })
}

fn get_config(boot_services: &BootServices) -> LoaderConfig {
    let mut config = LoaderConfig {
        log_device: LogDevice::StdOut,
        log_level: LevelFilter::Info,
    };

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

fn setup_logger(boot_services: &BootServices, config: LoaderConfig) {
    match config.log_device {
        LogDevice::StdOut => {}
        LogDevice::Serial => {
            if let Ok(serial_proto_handle) = boot_services.get_handle_for_protocol::<Serial>() {
                if let Ok(mut serial_proto) =
                    boot_services.open_protocol_exclusive::<Serial>(serial_proto_handle)
                {
                    // serial_proto.reset();
                    // log::set_logger(logger).expect("must be able to set the logger");
                }
            }
        }
    }

    log::set_max_level(config.log_level);
}

#[no_mangle]
pub extern "efiapi" fn efi_main(
    image_handle: uefi::Handle,
    mut system_table: SystemTable<Boot>,
) -> ! {
    system_table.stdout().clear().unwrap();

    let cpuid = CpuId::new();
    let cpu_vendor = cpuid
        .get_vendor_info()
        .expect("Must be able to get CPU vendor");
    let brand_str = cpuid.get_processor_brand_string();

    writeln!(
        system_table.stdout(),
        "Loading **CorgOS** on {} {}",
        cpu_vendor.as_str(),
        if let Some(b) = &brand_str {
            b.as_str()
        } else {
            ""
        }
    )
    .unwrap();

    let boot_services = system_table.boot_services();
    let config = get_config(boot_services);

    setup_logger(boot_services, config);

    let _ebs = system_table
        .exit_boot_services(image_handle, &mut [])
        .unwrap();

    panic!();
}

#[panic_handler]
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
