#![no_main]
#![no_std]

use log::info;
use uefi::{
    Result,
    prelude::*,
    proto::media::file::{
	FileAttribute,
	FileMode,
	File,
    },
    data_types::CStr16,
    table::boot::{
	MemoryType,
	MemoryMap,
    },
};

const KERNEL_PATH: &'static str = "\\efi\\boot\\kernel";

fn load_kernel(image_handle: Handle, boot_services: &BootServices) -> Result<Status> {
    let mut simple_fs_proto = boot_services.get_image_file_system(image_handle)?;
    let mut root_dir = simple_fs_proto.open_volume()?;
    let mut buf = [0;64];
    let kernel = root_dir.open(CStr16::from_str_with_buf(KERNEL_PATH, &mut buf).unwrap(), FileMode::Read, FileAttribute::empty())?;
    info!("Hello, uefi!");
    boot_services.stall(10_000_000);
    Ok(Status::SUCCESS)
}

/// UEFI Entrypoint.
///
/// Assumes the kernel binary is a sibiling file to the uefi os loader. The EFI boot partition
/// filesystem should look like this:
///
/// efi
/// |- boot/
///    |- BOOTx64.efi
///    |- kernel
#[entry]
fn main(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();
    let boot_services = system_table.boot_services();

    let map_sz = boot_services.memory_map_size();
    let buf_sz = map_sz.entry_size *  map_sz.map_size;

    // TODO: What memory type?
    let raw_buf = boot_services.allocate_pool(MemoryType::RESERVED, buf_sz).unwrap();
    let mut buf: &mut [u8] = unsafe { core::slice::from_raw_parts_mut(raw_buf, buf_sz) };
    let _mmap = boot_services.memory_map(&mut buf).unwrap();

    load_kernel(image_handle, boot_services).status()
}
