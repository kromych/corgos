#![no_std]
#![no_main]

mod image_layout;

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    todo!("Kernel stub");
}

#[no_mangle]
extern "C" fn rust_eh_personality() {
    core::hint::spin_loop();
}

#[no_mangle]
extern "C" fn rust_eh_unwind_resume(_: &i8) {
    core::hint::spin_loop();
}

#[cfg_attr(feature = "kernel_build", panic_handler)]
#[cfg_attr(not(feature = "kernel_build"), allow(unused))]
fn panic_handler(_pi: &core::panic::PanicInfo<'_>) -> ! {
    core::hint::spin_loop();
    unsafe { core::hint::unreachable_unchecked() }
}

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(include_str!("start-aarch64.S"));

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(include_str!("start-x86_64.S"));
