#![no_main]
#![no_std]

use log::info;
use uefi::{prelude::*, table::boot::OpenProtocolParams, proto::device_path::DevicePath, Result};

fn load_kernel(image_handle: Handle, boot_services: &BootServices) -> Result<Status> {
    let device_path_proto_handle = boot_services.get_handle_for_protocol::<DevicePath>()
	.expect("to have the device path protocol handle");
    let protocol = boot_services.open_protocol_exclusive::<DevicePath>(device_path_proto_handle)?;
    Ok(Status::SUCCESS)
}

#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // At this point uefi has done all the usual platform init, plus it has landed us
    // in long-mode with identity mapped pages.

    /*
    What actually needs to happen?
    - Setup page tables
    - load kernel binary into memory
    - call exit boot services
    - start executing the kernel
     */

    uefi_services::init(&mut system_table).unwrap();
    info!("Hello, uefi!");
    let boot_services = system_table.boot_services();
    boot_services.stall(10_000_000);
    load_kernel(image_handle, boot_services).status()
}
