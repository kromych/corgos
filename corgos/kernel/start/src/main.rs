#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn kernel_start() -> ! {
    loop {}
}

#[panic_handler]
fn panic_handler(_pi: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
