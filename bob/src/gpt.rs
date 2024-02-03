use std::ffi::CString;
use std::fs::File;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::time::SystemTime;
use crate::err::BobErr;
use crate::crc::crc32;
use crate::guid::Guid;

const LOGICAL_BLOCK_SZ: usize = 512;
const PARTITION_NAME_MAX_BYTES: usize = 72;

pub struct DiskImgBuilder {
    image_size: Option<usize>,
    output: Option<String>,
    partitions: Vec<Partition>,
}

impl DiskImgBuilder {
    pub fn new() -> Self {
        Self {
            image_size: None,
            output: None,
            partitions: Vec::new(),
        }
    }

    /// Total size of the output disk image
    pub fn total_size(mut self, s: usize) -> Self {
        self.image_size = Some(s);
        self
    }

    /// Filename to use for the created disk image.
    pub fn output_file(mut self, o: &str) -> Self {
        // TODO: not this.
        self.output = Some(String::from(o));
        self
    }

    /// Adds a partition to create in the generated disk image.
    pub fn partition(mut self, p: Partition) -> Self {
	self.partitions.push(p);
        self
    }


    /// Build the disk image file.
    pub fn build(self) -> Result<(), BobErr> {
	// Default to append the current time since UNIX EPOCH to avoid overwriting any old
	// images using the default filename by accident.
	let filename = if let Some(f) = self.output {
	    f
	} else {
	    let suffix = SystemTime::now()
		.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap()
		.as_secs().to_string();
	    format!("disk_image_{suffix}.img")
	};
	
	let mut f = File::options()
	    .write(true)
	    .create(true)
	    .open(filename).map_err(BobErr::IO)?;

	if let Some(image_size) = self.image_size {
	    f.set_len(image_size as u64).map_err(BobErr::IO)?;
	} else {
	    // This is already enforced by clap, just being careful.
	    return Err(BobErr::MissingArgument);
	}

	let image_size = self.image_size.expect("To have an image size provided");
	Self::write_protective_mbr_header(&mut f, image_size)?;	
	// TODO: validate the partiton offsets given make any sense
	Self::write_gpt_partition_table(&mut f, image_size, &self.partitions)?;
	// TODO: Push file cursor past partition space in the image
	// TODO: write the alternate partition table

	f.flush().map_err(BobErr::IO)?;

	Ok(())
    }

    /// Write the Protective MBR Header.
    /// Ref: https://uefi.org/specs/UEFI/2.10/05_GUID_Partition_Table_Format.html#protective-mbr
    fn write_protective_mbr_header(f: &mut File, size: usize) -> Result<(), BobErr> {
	// First 440 bytes are unused by UEFI systems
	f.write_all(&[0;440]).map_err(BobErr::IO)?;

	// Unique MBR Disk Signature, unused.
	f.write_all(&[0;4]).map_err(BobErr::IO)?;

	// Uknown
	f.write_all(&[0;2]).map_err(BobErr::IO)?;


	let mut first_record = PartitionRecord::new();
	first_record.starting_chs = [0x00, 0x02, 0x00];
	first_record.os_type = 0xEE;
	first_record.starting_lba = 0x00000001;
	first_record.size_in_lba = (size / LOGICAL_BLOCK_SZ) as u32;
	first_record.write(f)?;

	let zero = PartitionRecord::new();
	zero.write(f)?;
	zero.write(f)?;
	zero.write(f)?;

	// signature, set to 0xAA55.
	f.write_all(&[0x55, 0xAA]).map_err(BobErr::IO)?;

	// May need to pad out to the logical block size
	let pos = f.stream_position().map_err(BobErr::IO)?;
	if pos < (LOGICAL_BLOCK_SZ - 1) as u64 {
	    let zeros: Vec<u8> = [0].repeat(LOGICAL_BLOCK_SZ - 1 - pos as usize);
	    f.write_all(&zeros).map_err(BobErr::IO)?;
	}

	Ok(())
    }

    /// Write the partition table
    /// Header reference: https://uefi.org/specs/UEFI/2.10/05_GUID_Partition_Table_Format.html#gpt-header
    /// Entry reference: https://uefi.org/specs/UEFI/2.10/05_GUID_Partition_Table_Format.html#gpt-partition-entry-array
    fn write_gpt_partition_table(f: &mut File, image_size: usize, partitions: &[Partition]) -> Result<(), BobErr> {
	let partition_entries = &partitions.iter().map(|p| GptPartitionEntry::from_partition(p)).collect::<Vec<_>>();

	let mut header = GptHeader::new();

	// We don't extend the Protective MBR beyond 1 logical block in size
	// so this header is the second (or index 1).
	header.my_lba = 1;
	// Alternate (backup) header is located in the last logical block
	header.alt_lba = (image_size / LOGICAL_BLOCK_SZ) as u64 - 1;
	// Starts after minimum amount to reserve for protective MBR, GPT Header, and partition entry array.
	header.first_usable_lba = 34;
	let size_in_blocks = (image_size / LOGICAL_BLOCK_SZ) as u64;
	if size_in_blocks < 34 + 33 + 1 {
	    return Err(BobErr::ImageTooSmall);
	}

	// Subtracts 33 to reserve enough logical blocks for the backup partition table header (1)
	// and partiton entry array (32).
	header.last_usable_lba =  size_in_blocks - 33;

	// Partiton table information
	header.partition_entry_lba = 2;
	header.num_partition_entries = 128;
	header.partition_entry_sz = 128;
	let crc: u32 = partition_entries.iter().map(|p| p.crc()).sum();
	header.partition_entry_array_crc32 = crc;

	header.crc();
	header.write(f)?;

	for p in partition_entries {
	    p.write(f)?;
	}

	Ok(())
    }
}

/// A GPT Partition
#[derive(Clone, Copy, Debug)]
pub struct Partition {
    pt: PartitionType,
    start_offset: usize,
    end_offset: usize,
}

pub struct PartitionBuilder {
    pt: Option<PartitionType>,
    start_offset: Option<usize>,
    end_offset: Option<usize>,
}

impl PartitionBuilder {
    pub fn new() -> Self {
	Self {
	    pt: None,
	    start_offset: None,
	    end_offset: None,
	}
    }

    pub fn partition_type(mut self, pt: PartitionType) -> Self {
	self.pt = Some(pt);
	self
    }

    pub fn start_offset(mut self, start_offset: usize) -> Self {
	self.start_offset = Some(start_offset);
	self
    }

    pub fn end_offset(mut self, end_offset: usize) -> Self {
	self.end_offset = Some(end_offset);
	self
    }

    pub fn build(self) -> Result<Partition, BobErr> {
	if self.pt.is_none() || self.start_offset.is_none() || self.end_offset.is_none() {
	    return Err(BobErr::PartitionParse);
	}

	Ok(Partition {
	    pt: self.pt.unwrap(),
	    start_offset: self.start_offset.unwrap(),
	    end_offset: self.end_offset.unwrap()
	})
    }
}

/// Partition Type GUID
/// https://en.wikipedia.org/wiki/GUID_Partition_Table#Partition_type_GUIDs
#[derive(Clone, Copy, Debug)]
pub enum PartitionType {
    EFISystem,
}

impl PartitionType {
    fn uuid(&self) -> Guid {
	let b =[0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B];
	match self {
	    Self::EFISystem => Guid::from_bytes(b),
	}
    }

    fn name(&self) -> CString {
	match self {
	    Self::EFISystem => CString::new("EFI System Partition").expect("No null byte, not possible"),
	}
    }
}

struct PartitionRecord {
    boot_indicator: u8,
    starting_chs: [u8;3],
    os_type: u8,
    ending_chs: [u8;3],
    starting_lba: u32,
    size_in_lba: u32,
}

impl PartitionRecord {
    fn new() -> Self {
	Self {
	    boot_indicator: 0,
	    starting_chs: [0;3],
	    os_type: 0,
	    ending_chs: [0;3],
	    starting_lba: 0,
	    size_in_lba: 0,
	}
    }

    fn write(&self, f: &mut File) -> Result<(), BobErr> {
	#[cfg(target_endian = "little")]
	{
	    f.write_all(&self.boot_indicator.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.starting_chs).map_err(BobErr::IO)?;
	    f.write_all(&self.os_type.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.ending_chs).map_err(BobErr::IO)?;
	    f.write_all(&self.starting_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.size_in_lba.to_le_bytes()).map_err(BobErr::IO)?;
	}
	#[cfg(target_endian = "big")]
	{
	    f.write_all(&self.boot_indicator.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.starting_chs).map_err(BobErr::IO)?;
	    f.write_all(&self.os_type.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.ending_chs).map_err(BobErr::IO)?;
	    f.write_all(&self.starting_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.size_in_lba.to_be_bytes()).map_err(BobErr::IO)?;
	}	
	Ok(())
    }
}

struct GptHeader {
    signature: u64,
    revision: u32,
    header_sz: u32,
    header_crc32: u32,
    reserved: u32,
    my_lba: u64,
    alt_lba: u64,
    first_usable_lba: u64,
    last_usable_lba: u64,
    disk_guid: Guid,
    partition_entry_lba: u64,
    num_partition_entries: u32,
    partition_entry_sz: u32,
    partition_entry_array_crc32: u32,
}

impl GptHeader {

    fn new() -> Self {
	Self {
	    signature: 0x5452415020494645, // ASCII string “EFI PART”
	    revision: 0x00010000,
	    header_sz: 92,
	    header_crc32: 0,
	    reserved: 0,
	    my_lba: 0,
	    alt_lba: 0,
	    first_usable_lba: 0,
	    last_usable_lba: 0,
	    disk_guid: Guid::new_v4(),
	    partition_entry_lba: 0,
	    num_partition_entries: 0,
	    partition_entry_sz: 0,
	    partition_entry_array_crc32: 0,
	}
    }

    fn write(&self, f: &mut File) -> Result<(), BobErr> {
	#[cfg(target_endian = "little")]
	{
	    f.write_all(&self.signature.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.revision.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.header_sz.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.header_crc32.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.reserved.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.my_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.alt_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.first_usable_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.last_usable_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.disk_guid.to_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.partition_entry_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.num_partition_entries.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.partition_entry_sz.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.partition_entry_array_crc32.to_le_bytes()).map_err(BobErr::IO)?;
	}
	#[cfg(target_endian = "big")]
	{
	    f.write_all(&self.signature.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.revision.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.header_sz.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.header_crc32.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.reserved.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.my_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.alt_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.first_usable_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.last_usable_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(self.disk_guid.as_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.partition_entry_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.num_partition_entries.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.partition_entry_sz.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.partition_entry_array_crc32.to_be_bytes()).map_err(BobErr::IO)?;
	}

	f.seek(SeekFrom::Current((LOGICAL_BLOCK_SZ as i64) - 92)).map_err(BobErr::IO)?;

	Ok(())
    }

    #[cfg(target_endian = "little")]
    fn crc(&mut self) {
	self.header_crc32 = 0;
	self.header_crc32 = crc32(&self.signature.to_le_bytes());
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.revision.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.header_sz.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.header_crc32.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.reserved.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.my_lba.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.alt_lba.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.first_usable_lba.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.last_usable_lba.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.disk_guid.to_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.partition_entry_lba.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.num_partition_entries.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.partition_entry_sz.to_le_bytes()));
	self.header_crc32 = self.header_crc32.wrapping_add(crc32(&self.partition_entry_array_crc32.to_le_bytes()));
    }
}

struct GptPartitionEntry {
    partition_type_guid: Guid,
    unique_partition_guid: Guid,
    starting_lba: u64,
    ending_lba: u64,
    attributes: u64,
    partition_name: CString,
}

impl GptPartitionEntry {

    fn from_partition(p: &Partition) -> Self {
	let partition_type_guid = p.pt.uuid();
	let starting_lba = (p.start_offset / LOGICAL_BLOCK_SZ) as u64;
	let ending_lba = (p.end_offset / LOGICAL_BLOCK_SZ) as u64;
	let unique_partition_guid = Guid::new_v4();
	let partition_name = p.pt.name();

	Self {
	    partition_type_guid,
	    unique_partition_guid,
	    starting_lba,
	    ending_lba,
	    // TODO: bit 1 may need to be set for EFI System partitions
	    attributes: 0,
	    partition_name,
	}
    }

    fn crc(&self) -> u32 {
	let mut crc = crc32(&self.partition_type_guid.to_bytes());
	crc = crc.wrapping_add(crc32(&self.unique_partition_guid.to_bytes()));
	#[cfg(target_endian = "little")]
	{
	    crc = crc.wrapping_add(crc32(&self.starting_lba.to_le_bytes()));
	    crc = crc.wrapping_add(crc32(&self.ending_lba.to_le_bytes()));
	    crc = crc.wrapping_add(crc32(&self.attributes.to_le_bytes()));
	    crc = crc.wrapping_add(crc32(&self.partition_name.as_bytes()));
	    crc
	}
	#[cfg(target_endian = "big")]
	{
	    crc = crc.wrapping_add(crc32(&self.starting_lba.to_be_bytes()));
	    crc = crc.wrapping_add(crc32(&self.ending_lba.to_be_bytes()));
	    crc = crc.wrapping_add(crc32(&self.attributes.to_be_bytes()));
	    crc = crc.wrapping_add(crc32(&self.partition_name.as_bytes()));
	    crc
	}
    }

    fn write(&self, f: &mut File) -> Result<(), BobErr> {
	f.write_all(&self.partition_type_guid.to_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.unique_partition_guid.to_bytes()).map_err(BobErr::IO)?;
	#[cfg(target_endian = "little")]
	{
	    f.write_all(&self.starting_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.ending_lba.to_le_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.attributes.to_le_bytes()).map_err(BobErr::IO)?;
	}
	#[cfg(target_endian = "big")]
	{
	    f.write_all(&self.starting_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.ending_lba.to_be_bytes()).map_err(BobErr::IO)?;
	    f.write_all(&self.attributes.to_be_bytes()).map_err(BobErr::IO)?;
	}
	let name_bytes = self.partition_name.as_bytes();
	f.write_all(&name_bytes).map_err(BobErr::IO)?;

	// TODO: check can be pushed up to the arg parsing
	if name_bytes.len() > PARTITION_NAME_MAX_BYTES {
	    return Err(BobErr::PartitionNameTooLong);
	}

	let remaining: i64 = (PARTITION_NAME_MAX_BYTES - name_bytes.len()) as i64;
	f.seek(SeekFrom::Current(remaining)).map_err(BobErr::IO)?;

	Ok(())
    }
}
