#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[allow(dead_code)]
fn kmain() -> !{
    loop {}
}

#[panic_handler]
fn panic_handler(_info: &PanicInfo) -> ! {
    loop {}
}

