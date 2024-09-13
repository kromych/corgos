#![no_std]

use core::fmt::Write;

use boot_uart::BaudDivisor;
use boot_uart::ComPort;
use boot_uart::ComPortIo;
use boot_uart::Pl011;
use conquer_once::spin::OnceCell;
use log::LevelFilter;
use uefi::boot;
use uefi::proto::console::text::Output;

#[derive(Debug, Clone)]
pub struct BootLoaderConfig {
    pub log_device: LogDevice,
    pub log_level: LevelFilter,
    pub wait_for_start: bool,
    pub walk_page_tables: bool,
    pub watchdog_seconds: Option<usize>,
}

impl Default for BootLoaderConfig {
    fn default() -> Self {
        Self {
            log_device: LogDevice::StdOut,
            log_level: LevelFilter::Trace,
            wait_for_start: false,
            walk_page_tables: false,
            watchdog_seconds: None,
        }
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
pub struct BootLogger(Option<LogOutput>);

unsafe impl Send for BootLogger {}
unsafe impl Sync for BootLogger {}

impl log::Log for BootLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        match &self.0 {
            None => {}
            Some(LogOutput::Stdout) => {
                let stdout =
                    boot::get_handle_for_protocol::<Output>().expect("can get stdout handle");
                let mut stdout =
                    boot::open_protocol_exclusive::<Output>(stdout).expect("can open stdout");
                stdout
                    .write_fmt(format_args!(
                        "[{:7}][{}:{}@{}]  {}",
                        record.level(),
                        record.module_path().unwrap_or_default(),
                        record.file().unwrap_or_default(),
                        record.line().unwrap_or_default(),
                        record.args(),
                    ))
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

        BootLogger(output)
    });

    log::set_logger(logger).unwrap();
    log::set_max_level(config.log_level);

    log::trace!("{config:x?}, {logger:x?}");
}
