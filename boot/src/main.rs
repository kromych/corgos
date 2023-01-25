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
use uefi::prelude::cstr16;
use uefi::prelude::Boot;
use uefi::prelude::SystemTable;
use uefi::proto::console::text::Output;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::CStr16;
use uefi::Handle;
use uefi::Status;

struct SyncBootLogger {
    stdout: Spinlock<*mut Output<'static>>,
}

impl SyncBootLogger {
    fn new(boot_system_table: &mut SystemTable<Boot>) -> Self {
        // TODO: rework this barf
        let stdout = boot_system_table.stdout() as *mut Output as u64;
        let stdout = stdout as *mut Output;

        Self {
            stdout: Spinlock::new(stdout),
        }
    }
}

unsafe impl Send for SyncBootLogger {}
unsafe impl Sync for SyncBootLogger {}

impl log::Log for SyncBootLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let stdout = self.stdout.lock();
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

    fn flush(&self) {}
}

// struct SyncSerialLogger<'a> {
//     boot_system_table: SystemTable<Boot>,
//     serial_proto: Spinlock<ScopedProtocol<'a, Serial<'a>>>,
// }

// impl<'a> SyncSerialLogger<'a> {
//     fn new(boot_system_table: &SystemTable<Boot>) -> Option<Self> {
//         let boot_system_table = unsafe { boot_system_table.unsafe_clone() };
//         let boot_services = boot_system_table.boot_services();
//         if let Ok(serial_proto_handle) = boot_services.get_handle_for_protocol::<Serial>() {
//             if let Ok(mut serial_proto) =
//                 boot_services.open_protocol_exclusive::<Serial>(serial_proto_handle)
//             {
//                 serial_proto.reset();
//                 return Some(Self {
//                     boot_system_table,
//                     serial_proto: Spinlock::new(serial_proto),
//                 });
//             }
//         }
//         None
//     }
// }

// unsafe impl<'a> Send for SyncSerialLogger<'a> {}
// unsafe impl<'a> Sync for SyncSerialLogger<'a> {}

// impl<'a> log::Log for SyncSerialLogger<'a> {
//     fn enabled(&self, _metadata: &log::Metadata) -> bool {
//         true
//     }

//     fn log(&self, record: &log::Record) {
//         let mut serial_proto = self.serial_proto.lock();
//         writeln!(
//             serial_proto,
//             "{:5}:{} {}",
//             record.level(),
//             record.target(),
//             record.args()
//         )
//         .unwrap();
//     }

//     fn flush(&self) {}
// }

#[derive(Debug, Clone)]
enum LogDevice {
    StdOut,
    Serial,
}

#[derive(Debug, Clone)]
struct LoaderConfig {
    log_device: LogDevice,
    log_level: LevelFilter,
}

/// The name of the configuration file in the ESP partition alongside the loader.
const CORGOS_INI: &CStr16 = cstr16!("efi\\boot\\corgos-boot.ini");

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
        log_device: LogDevice::Serial,
        log_level: LevelFilter::Info,
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

fn setup_logger(boot_system_table: &mut SystemTable<Boot>, level: LevelFilter) {
    let logger = BOOT_LOGGER.get_or_init(move || SyncBootLogger::new(boot_system_table));
    log::set_logger(logger).unwrap();
    log::set_max_level(level);
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
    setup_logger(&mut boot_system_table, LevelFilter::Trace);

    let _config = get_config(&boot_system_table);

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

    let mut mmap_buf = [0_u8; 8192];
    let (_runtime_system_table, _memory_map) = boot_system_table
        .exit_boot_services(image_handle, &mut mmap_buf)
        .unwrap();

    loop {}
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
