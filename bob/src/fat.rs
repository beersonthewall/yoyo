use crate::err::BobErr;
use crate::gpt::{GptImage, Partition};

const SECTOR_SIZE: u16 = 512;
const SECTORS_PER_CLUSTER: u8 = 2;
const NUM_FATS: u8 = 2;

struct FatMeta {
    bpb: Bpb,
    ebr: ExtendedBootRecord,
    fs_info: FSInfo,
    fat: FileAllocationTable,
}

/// BIOS Paramter Block
struct Bpb {
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

/// Formats the given partition as a FAT32 filesystem.
pub fn format_as_fat<T: Partition>(p: &mut T) -> Result<(), BobErr> {
    let meta = FatMeta::new(p);
    p.write(&meta.bytes()).map_err(BobErr::IO)?;
    Ok(())
}

impl FatMeta {
    fn new<T: Partition>(p: &T) -> Self {
	let bpb = Bpb::new(p);
	let ebr = ExtendedBootRecord::new();
	let fs_info = FSInfo::new();
	let fat = FileAllocationTable::new();

	Self {
	    bpb,
	    ebr,
	    fs_info,
	    fat,
	}
    }

    fn bytes(&self) -> Vec<u8> {
	let mut b = Vec::new();
	b.extend(self.bpb.bytes());
	b.extend(self.ebr.bytes());
	b.extend(self.fs_info.bytes());
	b
    }
}

impl Bpb {
    fn new<T: Partition>(p: &T) -> Self {
	let total_sectors_long = (p.size() / SECTOR_SIZE as usize) as u32;

	Self {
	    jmp_short: [0xEB, 0x3C, 0x90],
	    oem_identifier: [0;8],
	    bytes_per_sector: SECTOR_SIZE,
	    sectors_per_cluster: SECTORS_PER_CLUSTER,
	    reserved_sectors: 0,
	    num_fats: NUM_FATS,
	    max_dir_entries: 0,
	    total_sectors_short: 0,
	    media_descriptor: 0,
	    sectors_per_fat_short: 0,
	    sectors_per_track: 0,
	    num_heads: 0,
	    hidden_sectors: 0,
	    total_sectors_long,
	}
    }

    fn bytes(&self) -> Vec<u8> {
	let mut b = Vec::new();
	b.extend(self.jmp_short);
	b.extend(self.oem_identifier);
	b.extend(self.bytes_per_sector.to_le_bytes());
	b.push(self.sectors_per_cluster);
	b.extend(self.reserved_sectors.to_le_bytes());
	b.push(self.num_fats);
	b.extend(self.max_dir_entries.to_le_bytes());
	b.extend(self.total_sectors_short.to_le_bytes());
	b.push(self.media_descriptor);
	b.extend(self.sectors_per_fat_short.to_le_bytes());
	b
    }
}

impl ExtendedBootRecord {
    fn new() -> Self {
	Self {
	    sectors_per_fat_long: 0,
	    flags: 0,
	    fat_version: 0,
	    root_cluster: 0,
	    fsinfo_sector: 0,
	    backup_boot_sector: 0,
	    reserved: [0; 12],
	    drive_number: 0,
	    windows_nt_flags: 0,
	    signature: 0,
	    volume_id: 0,
	    volume_label: [0; 11],
	    fs_type_label: [0; 8],
	}
    }

    fn bytes(&self) -> Vec<u8> {
	let mut b = Vec::new();
	b.extend(self.sectors_per_fat_long.to_le_bytes());
	b.extend(self.flags.to_le_bytes());
	b.extend(self.fat_version.to_le_bytes());
	b.extend(self.root_cluster.to_le_bytes());
	b.extend(self.fsinfo_sector.to_le_bytes());
	b.extend(self.reserved);
	b.push(self.drive_number);
	b.push(self.windows_nt_flags);
	b.push(self.signature);
	b.extend(self.volume_id.to_le_bytes());
	b.extend(self.volume_label);
	b.extend(self.fs_type_label);
	b
    }
}

impl FSInfo {
    fn new() -> Self {
	Self {
	    lead_signature: 0x41615252,
	    reserved1: [0;480],
	    struct_signature: 0x61417272,
	    free_count: 0,
	    next_free: 0,
	    reserved2: [0;12],
	    trail_signature: 0xAA550000,
	}
    }

    fn bytes(&self) -> Vec<u8> {
	let mut b = Vec::new();
	b.extend(self.lead_signature.to_le_bytes());
	b.extend(self.reserved1);
	b.extend(self.struct_signature.to_le_bytes());
	b.extend(self.free_count.to_le_bytes());
	b.extend(self.next_free.to_le_bytes());
	b.extend(self.reserved2);
	b.extend(self.trail_signature.to_le_bytes());
	b
    }
}

impl FileAllocationTable {
    fn new() -> Self {
	Self {}
    }
}
