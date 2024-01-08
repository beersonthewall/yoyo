#![no_main]
#![no_std]

use log::info;
use uefi::prelude::*;

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // At this point uefi has done all the usual platform init, plus it has landed us
    // in long-mode with identity mapped pages.
    uefi_services::init(&mut system_table).unwrap();
    info!("Hello, UEFI!");
    system_table.boot_services().stall(10_000_000);
    Status::SUCCESS
}
