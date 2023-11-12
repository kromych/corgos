# CorgOS

Currently, this is a UEFI bootloader that logs to the serial port. It supports a configuration file
to specify where to log to, and other parameters like so:

```ini
revision = "088bf38 Update reg definitions from the aarch64-lab, refactor"
log_device = com2
log_level = trace
wait_for_start = false
```

To build for `x86_64` and `aarch64`, run

```sh
./build.sh
```

To boot with `qemu`, use ` ./qemu-run-x86_64.sh` or ` ./qemu-run-aarch64.sh`
