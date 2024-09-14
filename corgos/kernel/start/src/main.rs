#![no_std]
#![no_main]

mod image_layout;

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    loop {}
}

#[panic_handler]
fn panic_handler(_pi: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(include_str!("start-aarch64.S"));

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(include_str!("start-x86_64.S"));
