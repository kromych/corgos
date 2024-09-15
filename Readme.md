# CorgOS

## Setting up the build host

### Ubuntu

```bash
sudo apt-get install clang llvm clang-tools lld qemu-system
```

### Fedora

```bash
sudo dnf install clang llvm clang-tools lld qemu-system
```

### macOS

```bash
brew install llvm qemu
```

## Cloning the repo

> This repo uses submodules. Please clone with
> `git clone --recurse https://github.com/kromych/corgos`
> or do
>
> ```sh
> git submodule init
> git submodule update
> ```

## Building and running

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
pub struct BootLoaderConfig {
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
```

To build the project and boot with `qemu`, use `./run.py -a x86_64` or `./run.py -a aarch64`.
The UEFI log is written to `fw.log`, the serial logs go to `serial*.log` files.

To only build the project, use

```sh
# Add '-r' for the release build
./run.py -b
```

## Look also

Many bits and pieces like the code for the serial port support,
self-relocation and page table manipulation have come from my
[aarch64-lab](https://github.com/kromych/aarch64-lab) project
and my C++ OS roject whose name was ToyOS.
