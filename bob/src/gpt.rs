use std::ffi::CString;
use std::fs::File;
use std::io::prelude::*;
use std::time::SystemTime;
use crate::err::BobErr;

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

	write_protective_mbr_header(&mut f, self.image_size.unwrap())?;
	write_partition_table(&mut f, &self.partitions)?;

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

fn write_protective_mbr_header(f: &mut File, size: usize) -> Result<(), BobErr> {
    let unused = [0;440 + 4 + 2];
    f.write_all(&unused).map_err(BobErr::IO)?;

    let zero = PartitionRecord::new();
    let mut first_record = PartitionRecord::new();
    first_record.starting_chs = [0x00, 0x02, 0x00];
    first_record.os_type = 0xEE;
    first_record.starting_lba = 0x00000001;
    first_record.size_in_lba = (size / 512) as u32;

    first_record.write(f)?;
    zero.write(f)?;
    zero.write(f)?;
    zero.write(f)?;

    Ok(())
}

fn write_partition_table(f: &mut File, partitions: &Vec<Partition>) -> Result<(), BobErr> {
    let gpt_header = GptHeader::new();
    // TODO: populate the header

    let partition_entries = partitions.iter().map(|p| GptPartitionEntry::from_partition(p)).collect::<Vec<_>>();
    gpt_header.write(f)?;

    for p in partition_entries {
	p.write(f)?;
    }

    Ok(())
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
	f.write_all(&self.boot_indicator.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.starting_chs).map_err(BobErr::IO)?;
	f.write_all(&self.os_type.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.ending_chs).map_err(BobErr::IO)?;
	f.write_all(&self.starting_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.size_in_lba.to_ne_bytes()).map_err(BobErr::IO)?;
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
    disk_guid: [u8;16],
    partition_entry_lba: u64,
    num_partition_entries: u32,
    partition_entry_sz: u32,
    partition_entry_array_crc32: u32,
}

impl GptHeader {

    fn write(&self, f: &mut File) -> Result<(), BobErr> {
	f.write_all(&self.signature.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.revision.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.header_sz.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.header_crc32.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.reserved.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.my_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.alt_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.first_usable_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.last_usable_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.disk_guid).map_err(BobErr::IO)?;
	f.write_all(&self.partition_entry_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.num_partition_entries.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.partition_entry_sz.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.partition_entry_array_crc32.to_ne_bytes()).map_err(BobErr::IO)?;
	Ok(())
    }

    fn bytes(&self) -> Vec<u8> {
	let mut bytes = vec![0;92];
	bytes[0..8].copy_from_slice(&self.signature.to_ne_bytes());
	bytes[8..12].copy_from_slice(&self.revision.to_ne_bytes());
	bytes[12..16].copy_from_slice(&self.header_sz.to_ne_bytes());
	bytes[16..20].copy_from_slice(&self.header_crc32.to_ne_bytes());
	bytes[20..24].copy_from_slice(&self.reserved.to_ne_bytes());
	bytes[24..32].copy_from_slice(&self.my_lba.to_ne_bytes());
	bytes[32..40].copy_from_slice(&self.alt_lba.to_ne_bytes());
	bytes[40..48].copy_from_slice(&self.first_usable_lba.to_ne_bytes());
	bytes[48..56].copy_from_slice(&self.last_usable_lba.to_ne_bytes());
	bytes[56..72].copy_from_slice(&self.disk_guid);
	bytes[72..80].copy_from_slice(&self.partition_entry_lba.to_ne_bytes());
	bytes[80..84].copy_from_slice(&self.num_partition_entries.to_ne_bytes());
	bytes[84..88].copy_from_slice(&self.partition_entry_sz.to_ne_bytes());
	bytes[88..92].copy_from_slice(&self.partition_entry_array_crc32.to_ne_bytes());

	bytes
    }

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
	    disk_guid: [0;16],
	    partition_entry_lba: 0,
	    num_partition_entries: 0,
	    partition_entry_sz: 0,
	    partition_entry_array_crc32: 0,
	}
    }
}

struct GptPartitionEntry {
    partition_type_guid: [u8;16],
    unique_partition_guid: [u8;16],
    starting_lba: u64,
    ending_lba: u64,
    attributes: u64,
    partition_name: CString,
}

impl GptPartitionEntry {

    fn from_partition(p: &Partition) -> Self {
	Self {
	    partition_type_guid: [0;16],
	    unique_partition_guid: [0;16],
	    starting_lba: 0,
	    ending_lba: 0,
	    attributes: 0,
	    partition_name: CString::new("").expect("Cannot happen, does not contain null byte."),
	}
    }

    fn write(&self, f: &mut File) -> Result<(), BobErr> {
	f.write_all(&self.partition_type_guid).map_err(BobErr::IO)?;
	f.write_all(&self.unique_partition_guid).map_err(BobErr::IO)?;
	f.write_all(&self.starting_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.ending_lba.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.attributes.to_ne_bytes()).map_err(BobErr::IO)?;
	f.write_all(&self.partition_name.as_bytes()).map_err(BobErr::IO)?;

	Ok(())
    }
}
