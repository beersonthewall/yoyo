use std::fs::File;
use std::io::prelude::*;
use std::{
    io,
    io::{
	SeekFrom,
	Write,
	Seek,
	ErrorKind
    }
};
use std::time::SystemTime;
use crc32fast::Hasher;
use crate::err::BobErr;
use crate::guid::Guid;

const LOGICAL_BLOCK_SZ: usize = 512;
const PARTITION_NAME_MAX_BYTES: usize = 72;

pub struct GptImage {
    hdr: GptHeader,
    bkp_hdr: GptHeader,
    pentry: Vec<GptPartitionEntry>,
    fd: File,
}

pub trait Partition: Write + Seek {
    fn ptype(&self) -> PartitionType;
    fn name(&self) -> &str;
    fn size(&self) -> usize;
}

/// A 'view' into a partition. Allows for reading and writing to
/// only the given partition specified by the GptPartitonEntry
/// without having access to other parts of the GptImage.
pub struct PartitionView<'a> {
    meta: &'a GptPartitionEntry,
    offset: u64,
    fd: &'a mut File,
}

// Builders and input strutures

/// Partitoion input data collected from the cmd line
#[derive(Clone, Copy, Debug)]
pub struct PartitionInput {
    pt: PartitionType,
    start_offset: usize,
    end_offset: usize,
}

pub struct DiskImgBuilder {
    image_size: Option<usize>,
    output: Option<String>,
    partitions: Vec<PartitionInput>,
}

pub struct PartitionBuilder {
    pt: Option<PartitionType>,
    start_offset: Option<usize>,
    end_offset: Option<usize>,
}

// GPT Metadata structures

#[derive(Clone, Copy)]
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

#[derive(Debug)]
struct GptPartitionEntry {
    partition_type_guid: Guid,
    unique_partition_guid: Guid,
    starting_lba: u64,
    ending_lba: u64,
    attributes: u64,
    partition_name: String,
}

struct PartitionRecord {
    boot_indicator: u8,
    starting_chs: [u8;3],
    os_type: u8,
    ending_chs: [u8;3],
    starting_lba: u32,
    size_in_lba: u32,
}

/// Partition Type GUID
/// https://en.wikipedia.org/wiki/GUID_Partition_Table#Partition_type_GUIDs
#[derive(Clone, Copy, Debug)]
pub enum PartitionType {
    EFISystem,
}

impl GptImage {
    /// Returns a reference to the first partition
    pub fn get_partition_view(&mut self, name: &str) -> Option<PartitionView> {
	let matches: Vec<_> = self.pentry.iter().filter(|p| &p.partition_name == name).collect();
	if let Some(meta) = matches.into_iter().next() {
	    Some(PartitionView::new(&mut self.fd, meta))
	} else {
	    None
	}
    }
}

impl<'a> PartitionView<'a> {
    fn new(fd: &'a mut File, meta: &'a GptPartitionEntry) -> Self {
	Self {
	    fd,
	    meta,
	    offset: 0,
	}
    }
}

impl<'a> Partition for PartitionView<'a> {
    fn name(&self) -> &str {
	&self.meta.partition_name
    }

    fn size(&self) -> usize {
	(self.meta.ending_lba - self.meta.starting_lba) as usize * LOGICAL_BLOCK_SZ
    }

    fn ptype(&self) -> PartitionType {
	// TODO: not this
	PartitionType::EFISystem
    }
}

impl<'a> Write for PartitionView<'a> {

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
	let base = self.meta.starting_lba * LOGICAL_BLOCK_SZ as u64;
	let size = (self.meta.ending_lba * LOGICAL_BLOCK_SZ as u64) - base;

	let current_pos = self.fd.stream_position()?;

	if current_pos < base || current_pos >= base + size {
	    self.fd.seek(SeekFrom::Start(base + self.offset))?;
	}

	let space_remaining = size - self.offset;
	if buf.len() as u64 > space_remaining {
	    return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "Cursor would pass partition end writing this buffer."));
	}

	let written = self.fd.write(buf)?;
	self.offset += written as u64;

	Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
	self.fd.flush()
    }
}

impl<'a> Seek for PartitionView<'a> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
	match pos {
	    SeekFrom::Current(offset) => {},
	    SeekFrom::Start(offset) => {},
	    SeekFrom::End(offset) => {}
	}
	Ok(1)
    }
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
    pub fn partition(mut self, p: PartitionInput) -> Self {
	self.partitions.push(p);
        self
    }


    /// Build the disk image file.
    pub fn build(self) -> Result<GptImage, BobErr> {
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
	
	let f = File::options()
	    .write(true)
	    .create(true)
	    .open(filename).map_err(BobErr::IO)?;

	let mut gpt = GptImage {
	    hdr: GptHeader::new(),
	    bkp_hdr: GptHeader::new(),
	    pentry: Vec::new(),
	    fd: f
	};

	if let Some(image_size) = self.image_size {
	    gpt.fd.set_len(image_size as u64).map_err(BobErr::IO)?;
	} else {
	    // This is already enforced by clap, just being careful.
	    return Err(BobErr::MissingArgument);
	}

	let image_size = self.image_size.expect("To have an image size provided");
	Self::write_protective_mbr_header(&mut gpt.fd, image_size)?;	
	// TODO: validate the partiton offsets given make any sense
	Self::write_gpt_partition_table(&mut gpt, image_size, &self.partitions)?;

	Ok(gpt)
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

	let ending_chs = size / LOGICAL_BLOCK_SZ;
	if ending_chs >= 0xFF_FF_FF {
	    first_record.ending_chs = [0xFF, 0xFF, 0xFF];
	} else {
	    first_record.ending_chs = [(ending_chs | 0xFF) as u8, ((ending_chs >> 8) | 0xFF) as u8, ((ending_chs >> 16) | 0xFF) as u8];
	}

	first_record.os_type = 0xEE;
	first_record.starting_lba = 0x00000001;
	first_record.size_in_lba = (size / LOGICAL_BLOCK_SZ) as u32;
	first_record.write(f)?;

	// TODO: don't need to do this work, can just seek past it since by creating
	// the file to a specific size it'll be zero-initialized.
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
    fn write_gpt_partition_table(gpt: &mut GptImage, image_size: usize, partitions: &[PartitionInput]) -> Result<(), BobErr> {
	let partition_entries = partitions.iter().map(|p| GptPartitionEntry::from_partition(p)).collect::<Vec<_>>();

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

	header.last_usable_lba =  size_in_blocks - 34;

	// Partiton table information
	header.partition_entry_lba = 2;
	header.num_partition_entries = 128;
	header.partition_entry_sz = 128;
	let crc: u32 = partition_entries.iter().map(|p| p.crc()).sum();
	header.partition_entry_array_crc32 = crc;

	header.crc();
	header.write(&mut gpt.fd)?;

	gpt.hdr = header;
	gpt.pentry = partition_entries;

	for p in &gpt.pentry {
	    p.write(&mut gpt.fd)?;
	}

	let backup_table_lba = size_in_blocks - 33;
	gpt.fd.seek(SeekFrom::Start(backup_table_lba * LOGICAL_BLOCK_SZ as u64)).map_err(BobErr::IO)?;

	for p in &gpt.pentry {
	    p.write(&mut gpt.fd)?;
	}

	// subtract 2, 1 for the last bock, 1 to adjust for 0-based indexing
	let last_block_number = size_in_blocks - 1;
	gpt.fd.seek(SeekFrom::Start(last_block_number * LOGICAL_BLOCK_SZ as u64)).map_err(BobErr::IO)?;

	gpt.bkp_hdr = gpt.hdr.clone();
	gpt.bkp_hdr.partition_entry_lba = backup_table_lba;
	gpt.bkp_hdr.write(&mut gpt.fd)?;

	Ok(())
    }
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

    pub fn build(self) -> Result<PartitionInput, BobErr> {
	if self.pt.is_none() || self.start_offset.is_none() || self.end_offset.is_none() {
	    return Err(BobErr::PartitionParse);
	}

	Ok(PartitionInput {
	    pt: self.pt.unwrap(),
	    start_offset: self.start_offset.unwrap(),
	    end_offset: self.end_offset.unwrap()
	})
    }
}

impl PartitionType {
    fn uuid(&self) -> Guid {
	let b =[0x28, 0x73, 0x2A, 0xC1, 0x1F, 0xF8, 0xD2, 0x11, 0xBA, 0x4B, 0x00, 0xA0, 0xC9, 0x3E, 0xC9, 0x3B];
	match self {
	    Self::EFISystem => Guid::from_bytes(b),
	}
    }

    pub fn name(&self) -> String {
	match self {
	    Self::EFISystem => String::from("EFI system partition"),
	}
    }
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
	f.write_all(&self.boot_indicator.to_le_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.starting_chs).map_err(BobErr::IO)?;
	f.write_all(&self.os_type.to_le_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.ending_chs).map_err(BobErr::IO)?;
	f.write_all(&self.starting_lba.to_le_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.size_in_lba.to_le_bytes()).map_err(BobErr::IO)?;
	Ok(())
    }
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
	f.seek(SeekFrom::Current((LOGICAL_BLOCK_SZ as i64) - 92)).map_err(BobErr::IO)?;

	Ok(())
    }

    fn crc(&mut self) {
	let mut h = Hasher::new();
	self.header_crc32 = 0;

	h.update(&self.signature.to_le_bytes());
	h.update(&self.revision.to_le_bytes());
	h.update(&self.header_sz.to_le_bytes());
	h.update(&self.header_crc32.to_le_bytes());
	h.update(&self.reserved.to_le_bytes());
	h.update(&self.my_lba.to_le_bytes());
	h.update(&self.alt_lba.to_le_bytes());
	h.update(&self.first_usable_lba.to_le_bytes());
	h.update(&self.last_usable_lba.to_le_bytes());
	h.update(&self.disk_guid.to_bytes());
	h.update(&self.partition_entry_lba.to_le_bytes());
	h.update(&self.num_partition_entries.to_le_bytes());
	h.update(&self.partition_entry_sz.to_le_bytes());
	h.update(&self.partition_entry_array_crc32.to_le_bytes());

	self.header_crc32 = h.finalize();
    }
}

impl GptPartitionEntry {

    fn from_partition(p: &PartitionInput) -> Self {
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
	let mut h = Hasher::new();
	h.update(&self.partition_type_guid.to_bytes());
	h.update(&self.unique_partition_guid.to_bytes());
	h.update(&self.starting_lba.to_le_bytes());
	h.update(&self.ending_lba.to_le_bytes());
	h.update(&self.attributes.to_le_bytes());
	h.update(&self.partition_name.as_bytes());
	h.finalize()
    }

    fn write(&self, f: &mut File) -> Result<(), BobErr> {
	f.write_all(&self.partition_type_guid.to_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.unique_partition_guid.to_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.starting_lba.to_le_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.ending_lba.to_le_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.attributes.to_le_bytes()).map_err(BobErr::IO)?;
	let name_bytes: Vec<u8> = str::encode_utf16(&self.partition_name).map(|c| c.to_le_bytes()).flatten().collect();
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
