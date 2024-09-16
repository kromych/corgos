#![no_std]

//! Logging facilities for the bootloader.
//!
//! While UEFI offers the serial output, that works only when
//! the boot services are still active. Due to this, a UART with
//! polling is used.

use core::fmt::Write;

use conquer_once::spin::OnceCell;
use log::LevelFilter;
use poll_uart::BaudDivisor;
use poll_uart::ComPort;
use poll_uart::ComPortIo;
use poll_uart::Pl011;
use uefi::boot;
use uefi::proto::console::text::Output;
use uefi::table;

pub const MAX_REVISION_SIZE: usize = 64;

#[derive(Debug, Clone)]
pub struct BootLoaderConfig {
    /// Git revision and some data about the latest change.
    pub revision: [u8; MAX_REVISION_SIZE],
    /// The target device for boot logging.
    pub log_device: LogDevice,
    /// Verbosity for logging.
    pub log_level: LevelFilter,
    /// Log source line and path.
    pub log_source_path: bool,
    /// Wait at the entry point until `x9` or `r9` are set to `0`.
    pub wait_for_start: bool,
    /// Walk the page tables, and dump the page table entries.
    pub walk_page_tables: bool,
    /// TImeout in seconds for the UEFI watchdog.
    pub watchdog_seconds: Option<usize>,
}

impl Default for BootLoaderConfig {
    fn default() -> Self {
        Self {
            revision: [0; MAX_REVISION_SIZE],
            log_device: LogDevice::StdOut,
            log_level: LevelFilter::Trace,
            log_source_path: false,
            wait_for_start: false,
            walk_page_tables: false,
            watchdog_seconds: None,
        }
    }
}

impl BootLoaderConfig {
    pub fn revision_str(&self) -> &str {
        let len = self
            .revision
            .iter()
            .position(|&x| x == 0)
            .unwrap_or(self.revision.len());
        unsafe { core::str::from_utf8_unchecked(&self.revision[..len]) }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum LogOutput {
    Stdout,
    Com(ComPort),
    Pl(Pl011),
}

/// Single-thread logger
#[derive(Debug)]
pub struct BootLogger {
    output: Option<LogOutput>,
    log_source_path: bool,
}

impl BootLogger {
    fn write(&self, output: &mut dyn Write, record: &log::Record) {
        output
            .write_fmt(format_args!(
                "[{:7}][{}",
                record.level(),
                record.module_path().unwrap_or_default(),
            ))
            .ok();
        if self.log_source_path {
            output
                .write_fmt(format_args!(
                    "{}@{}",
                    record.file().unwrap_or_default(),
                    record.line().unwrap_or_default(),
                ))
                .ok();
        }
        output.write_fmt(format_args!("] {}", record.args())).ok();
        if matches!(
            self.output,
            Some(LogOutput::Com(_)) | Some(LogOutput::Pl(_))
        ) {
            output.write_str("\r\n").ok();
        }
    }
}

unsafe impl Send for BootLogger {}
unsafe impl Sync for BootLogger {}

impl log::Log for BootLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        match &self.output {
            None => {}
            Some(LogOutput::Stdout) => {
                if table::system_table_raw().is_some() {
                    // Boot services are still acive.
                    let stdout =
                        boot::get_handle_for_protocol::<Output>().expect("can get stdout handle");
                    let mut stdout =
                        boot::open_protocol_exclusive::<Output>(stdout).expect("can open stdout");
                    self.write(&mut *stdout, record);
                }
            }
            Some(LogOutput::Com(mut serial_port)) => {
                self.write(&mut serial_port, record);
            }
            Some(LogOutput::Pl(mut pl011_dev)) => {
                self.write(&mut pl011_dev, record);
            }
        }
    }

    fn flush(&self) {}
}

#[derive(Debug, Clone)]
pub enum LogDevice {
    Null,
    StdOut,
    Com1,
    Com2,
    Pl011(u64),
}

static BOOT_LOGGER: OnceCell<BootLogger> = OnceCell::uninit();

pub fn setup_logger(config: &BootLoaderConfig) {
    let stdout_logger = || {
        let stdout = boot::get_handle_for_protocol::<Output>().expect("can get stdout handle");
        let mut stdout = boot::open_protocol_exclusive::<Output>(stdout).expect("can open stdout");
        stdout.clear().ok();
        Some(LogOutput::Stdout)
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

        BootLogger {
            output,
            log_source_path: config.log_source_path,
        }
    });

    log::set_logger(logger).unwrap();
    log::set_max_level(config.log_level);
}
