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
	FileInfo,
    },
    
    data_types::CStr16,
    table::boot::{
	MemoryType,
	MemoryMap,
    },
};
use common::elf::{load_elf, Elf};

const KERNEL_PATH: &'static str = "\\efi\\boot\\kernel";

fn load_kernel(image_handle: Handle, boot_services: &BootServices) -> Result<&'static mut [u8]> {
    let mut simple_fs_proto = boot_services.get_image_file_system(image_handle)?;
    let mut root_dir = simple_fs_proto.open_volume()?;
    let mut buf = [0;64];
    let kernel = root_dir.open(CStr16::from_str_with_buf(KERNEL_PATH, &mut buf).unwrap(), FileMode::Read, FileAttribute::empty())?;

    info!("Hello, uefi!");
    info!("Parsing kernel elf binary...");

    let mut kernel = kernel.into_regular_file().expect("The kernel to be a regular file");
    let mut buf = [0;512];
    let file_info: &mut FileInfo = kernel.get_info(&mut buf).expect("file info");
    let kernel_sz = usize::try_from(file_info.file_size()).unwrap();

    let kbuf = boot_services.allocate_pool(MemoryType::RESERVED, kernel_sz).expect("kernel buf alloc");
    unsafe { core::ptr::write_bytes(kbuf, 0, kernel_sz) }
    let kbuf = unsafe { core::slice::from_raw_parts_mut(kbuf, kernel_sz) };

    let bytes_read = kernel.read(kbuf).expect("read kernel");
    assert!(bytes_read == kernel_sz);
    Ok(kbuf)
}

fn switch_to_kernel() -> ! {
    loop {}
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
    let kernel = load_kernel(image_handle, boot_services).expect("Kernel bytes from disk");
    let _kernel_elf = load_elf(kernel).expect("Kernel is a valid ELF binary");

    info!("exit boot services");
    let (_system_table, mut _memory_map) = system_table.exit_boot_services(MemoryType::RESERVED);

    switch_to_kernel();
}
