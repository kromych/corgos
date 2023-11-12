# CorgOS

Currently, this is a UEFI bootloader that logs to the serial port. It supports a configuration file
to specify where to log to, and other parameters like so for `x86_64`:

```ini
revision = "088bf38 Update reg definitions from the aarch64-lab, refactor"
log_device = com2
log_level = trace
wait_for_start = false
```

or `aarch64`:

```ini
revision = "87b08ae Add readme"
log_device = "pl011@9000000"
log_level = trace
wait_for_start = false
walk_page_tables = false
```

The names of the keys come from this definition (this will be a rustdoc one day):

```rust
struct BootLoaderConfig {
    /// The target device for boot logging.
    log_device: LogDevice,
    /// Verbosity for logging
    log_level: LevelFilter,
    /// Wait at the entry point until `x9` or `r9` are set to `0`.
    wait_for_start: bool,
    /// Walk the page tables, and dump the page table entries.
    walk_page_tables: bool,
    /// TImeout in seconds for the UEFI watchdog.
    watchdog_seconds: Option<usize>,
}
```

To build for `x86_64` and `aarch64`, run

```sh
./build.sh
```

To boot with `qemu`, use ` ./qemu-run-x86_64.sh` or ` ./qemu-run-aarch64.sh`. The UEFI log
is written to `fw.log`, the serial logs go to `serial*.log` files.
