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
	write_partition_table(&mut f);

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

    let zero = PartitionRecord::new().bytes();
    let mut first_record = PartitionRecord::new();
    first_record.starting_chs = [0x00, 0x02, 0x00];
    first_record.os_type = 0xEE;
    first_record.starting_lba = 0x00000001;
    first_record.size_in_lba = (size / 512) as u32;

    f.write_all(&first_record.bytes()).map_err(BobErr::IO)?;
    f.write_all(&zero).map_err(BobErr::IO)?;
    f.write_all(&zero).map_err(BobErr::IO)?;
    f.write_all(&zero).map_err(BobErr::IO)?;

    Ok(())
}

fn write_partition_table(f: &mut File) -> Result<(), BobErr> {
    let gpt_header = GptHeader::new();
    // TODO: populate the header

    f.write_all(&gpt_header.bytes()).map_err(BobErr::IO)?;

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

    fn bytes(&self) -> Vec<u8> {
	// TODO we can make this so we don't copy so much.
	let mut bytes = vec![0;16];
	bytes[0] = self.boot_indicator;
	bytes[1..4].copy_from_slice(&self.starting_chs);
	bytes[4] = self.os_type;
	bytes[5..8].copy_from_slice(&self.ending_chs);

	bytes[8] = (self.starting_lba & 0xFF) as u8;
	bytes[9] = (self.starting_lba & (0xFF << 8)) as u8;
	bytes[10] = (self.starting_lba & (0xFF << 16)) as u8;	
	bytes[11] = (self.starting_lba & (0xFF << 24)) as u8;

	bytes[12] = (self.size_in_lba & 0xFF) as u8;
	bytes[13] = (self.size_in_lba & (0xFF << 8)) as u8;
	bytes[14] = (self.size_in_lba & (0xFF << 16)) as u8;	
	bytes[15] = (self.size_in_lba & (0xFF << 24)) as u8;

	bytes
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

    fn bytes(&self) -> Vec<u8> {
	let mut bytes = vec![0;92];
	bytes[0..8].copy_from_slice(&u64_to_slice(self.signature));
	bytes[8..12].copy_from_slice(&u32_to_slice(self.revision));
	bytes[12..16].copy_from_slice(&u32_to_slice(self.header_sz));
	bytes[16..20].copy_from_slice(&u32_to_slice(self.header_crc32));
	bytes[20..24].copy_from_slice(&u32_to_slice(self.reserved));
	bytes[24..32].copy_from_slice(&u64_to_slice(self.my_lba));
	bytes[32..40].copy_from_slice(&u64_to_slice(self.alt_lba));
	bytes[40..48].copy_from_slice(&u64_to_slice(self.first_usable_lba));
	bytes[48..56].copy_from_slice(&u64_to_slice(self.last_usable_lba));
	bytes[56..72].copy_from_slice(&self.disk_guid);
	bytes[72..80].copy_from_slice(&u64_to_slice(self.partition_entry_lba));
	bytes[80..84].copy_from_slice(&u32_to_slice(self.num_partition_entries));
	bytes[84..88].copy_from_slice(&u32_to_slice(self.partition_entry_sz));
	bytes[88..92].copy_from_slice(&u32_to_slice(self.partition_entry_array_crc32));

	bytes
    }

    fn new() -> Self {
	Self {
	    signature: 0,
	    revision: 0,
	    header_sz: 0,
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

fn u32_to_slice(i: u32) -> [u8;4] {
    let mut result = [0;4];
    let mut i = i;
    for ind in 0..4 {
	result[ind] = (i & 0xFF) as u8;
	i >>= 8;
    }
    result    
}

fn u64_to_slice(i: u64) -> [u8;8] {
    let mut result = [0;8];

    let mut i = i;
    for ind in 0..8 {
	result[ind] = (i & 0xFF) as u8;
	i >>= 8;
    }

    result
}
