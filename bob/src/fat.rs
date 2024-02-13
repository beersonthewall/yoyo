use crate::err::BobErr;
use crate::gpt::{Partition};

struct FatMeta {
    bpb: BPB,
    ebr: ExtendedBootRecord,
    fs_info: FSInfo,
}

/// BIOS Paramter Block
struct BPB {
    jmp_short: [u8; 3],
    oem_identifier: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats: u8,
    max_dir_entries: u16,
    total_sectors_short: u16,
    media_descriptor: u8,
    sectors_per_fat_short: u16,
    sectors_per_track: u16,
    num_heads: u16,
    hidden_sectors: u32,
    total_sectors_long: u32,
}

struct ExtendedBootRecord {
    sectors_per_fat_long: u32,
    flags: u16,
    fat_version: u16,
    root_cluster: u32,
    fsinfo_sector: u16,
    backup_boot_sector: u16,
    reserved: [u8; 12],
    drive_number: u8,
    windows_nt_flags: u8,
    signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_type_label: [u8; 8],
}

struct FSInfo {
    lead_signature: u32,
    reserved1: [u8; 480],
    struct_signature: u32,
    free_count: u32,
    next_free: u32,
    reserved2: [u8; 12],
    trail_signature: u32,
}

struct FileAllocationTable {
    
}

/// TODO: document
pub fn format_as_fat(p: &mut Partition) -> Result<(), BobErr> {
//    let meta = FatMeta::new(p.size());
    Ok(())
}

/*impl FatMeta {
    fn new(sz: usize) -> Self {
	Self {
	    
	}
    }
}*/
