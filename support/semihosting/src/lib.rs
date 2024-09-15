#![no_std]

pub struct Semihosting;

/// Semihosting for Aarch64
///
/// See [Reference](https://github.com/ARM-software/abi-aa/blob/main/semihosting/semihosting.rst)
/// for futher details.
#[cfg(target_arch = "aarch64")]
pub mod aarch64 {
    use crate::Semihosting;
    use core::arch::asm;

    macro_rules! host_trap {
        () => {
            "hlt #0xF000"
        };
    }

    macro_rules! semi_call {
    // Base case for no additional parameters (just the number).
    ($number:expr) => {{
        let r: u64;
        unsafe {
            asm!(
                host_trap!(),
                in("w0") $number,
                lateout("x0") r,
                options(nostack, preserves_flags),
            );
        }
        r
    }};

    // For 1 parameter.
    ($number:expr, $p1:expr) => {{
        let r: u64;
        unsafe {
            asm!(
                host_trap!(),
                in("w0") $number,
                in("x1") $p1 as u64,
                lateout("x0") r,
                options(nostack, preserves_flags),
            );
        }
        r
    }};

    // For 2 parameters.
    ($number:expr, $p1:expr, $p2:expr) => {{
        let r: u64;
        unsafe {
            asm!(
                host_trap!(),
                in("w0") $number,
                in("x1") $p1 as u64,
                in("x2") $p2 as u64,
                lateout("x0") r,
                options(nostack, preserves_flags),
            );
        }
        r
    }};

    // For 3 parameters.
    ($number:expr, $p1:expr, $p2:expr, $p3:expr) => {{
        let r: u64;
        unsafe {
            asm!(
                host_trap!(),
                in("w0") $number,
                in("x1") $p1 as u64,
                in("x2") $p2 as u64,
                in("x3") $p3 as u64,
                lateout("x0") r,
                options(nostack, preserves_flags),
            );
        }
        r
    }};

    // For 4 parameters.
    ($number:expr, $p1:expr, $p2:expr, $p3:expr, $p4:expr) => {{
        let r: u64;
        unsafe {
            asm!(
                host_trap!(),
                in("w0") $number,
                in("x1") $p1 as u64,
                in("x2") $p2 as u64,
                in("x3") $p3 as u64,
                in("x4") $p4 as u64,
                lateout("x0") r,
                options(nostack, preserves_flags),
            );
        }
        r
    }};
}

    const SYS_OPEN: u32 = 0x01;
    const SYS_CLOSE: u32 = 0x02;
    const SYS_WRITEC: u32 = 0x03;
    const SYS_WRITE0: u32 = 0x04;
    const SYS_WRITE: u32 = 0x05;
    const SYS_READ: u32 = 0x06;
    const SYS_READC: u32 = 0x07;
    const SYS_FLEN: u32 = 0x0c;
    const SYS_EXIT: u32 = 0x18;
    const SYS_ERRNO: u32 = 0x13;

    impl Semihosting {
        /// Might be divergent if semihosting is present,
        /// or cause a hardware fault. Neither of that impacts
        /// memory-safety, hence not marking as unsafe.
        pub fn exit_host(&self, code: u64) {
            const APPLICATION_EXIT: u64 = 0x20026;

            let data = [APPLICATION_EXIT, code];
            semi_call!(SYS_EXIT, data.as_ptr());
        }

        pub fn exit_host_success(&self) {
            self.exit_host(0)
        }

        pub fn exit_host_failure(&self) {
            self.exit_host(1)
        }

        pub fn write_dbg_char(&self, c: char) {
            let data = [c as u64];
            semi_call!(SYS_WRITEC, data.as_ptr());
        }

        pub fn write_dbg_str0(&self, s: &[u8]) {
            semi_call!(SYS_WRITE0, s.as_ptr());
        }

        pub fn write_dbg_hex(&self, h: u64) {
            let mut hs = [0_u16; 11];
            hs[0] = u16::from_le_bytes([b'0', b'x']);

            let hexn = |nibble| match nibble {
                0..=9 => nibble + b'0',
                10..=15 => nibble - 10 + b'a',
                _ => panic!("Nibble out of range"),
            };
            for (n, &b) in h.to_be_bytes().iter().enumerate() {
                hs[n + 1] = ((hexn(b & 0xf) as u16) << 8) | (hexn(b >> 4) as u16);
            }
            let hs = unsafe { core::slice::from_raw_parts(hs.as_ptr() as *const u8, hs.len() * 2) };
            self.write_dbg_str0(hs);
        }
    }

    impl core::fmt::Write for Semihosting {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let buf = core::mem::MaybeUninit::<[u8; 512]>::uninit();

            let bytes = s.as_bytes();
            let mut buf = unsafe { buf.assume_init() };

            let mut printed = 0;
            while printed < s.len() {
                let available = core::cmp::min(buf.len() - 2, s.len() - printed);
                buf[0..available].copy_from_slice(&bytes[..available]);
                buf[available] = 0;

                self.write_dbg_str0(&buf);

                printed += available;
            }

            Ok(())
        }
    }
}

/// Semihosting for x86_64 supported by qemu.
///
/// Only exits are supported. The device specification might specify a custom I/O port,
/// and I/O size, e.g.: `-device isa-debug-exit,iobase=0xf4,iosize=0x04`. Here, the
/// defaults are used.
#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use crate::Semihosting;

    pub const IO_BASE: u16 = 0x501;
    pub const SUCCESS_CODE: u8 = 0xf;
    const _CHECK_SUCCESS_CODE_ODD: () = assert!((SUCCESS_CODE & 1) == 1);

    macro_rules! host_trap {
        ($code:expr) => {
            unsafe {
                core::arch::asm!(
                    "out dx, al",
                    in("dx") IO_BASE,
                    in("al") $code,
                    options(nomem, nostack)
                );
            }
        };
    }

    impl Semihosting {
        pub fn exit_host(&self, code: u8) {
            // qemu exits with `(code << 1) | 1`.
            host_trap!(code);
        }

        pub fn exit_host_success(&self) {
            // Shift as qemu does `(code << 1) | 1`.
            self.exit_host(SUCCESS_CODE >> 1)
        }

        pub fn exit_host_failure(&self) {
            self.exit_host(0)
        }
    }
}
