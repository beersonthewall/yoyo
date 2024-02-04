use clap::ArgMatches;

use crate::err::BobErr;
use crate::gpt::{DiskImgBuilder, Partition, GptImage};

/// Creates a disk image from the provided argument matches.
pub fn create_disk_image(create_matches: &ArgMatches) -> Result<GptImage, BobErr> {
    let mut img_builder = DiskImgBuilder::new();

    if let Some(output_filename) = create_matches.get_one::<String>("output") {
        img_builder = img_builder.output_file(output_filename);
    }

    if let Some(size) = create_matches.get_one::<usize>("size") {
        img_builder = img_builder.total_size(*size);
    }

    if let Some(partitions) = create_matches.get_many::<Partition>("partition") {
	for p in partitions {
	    img_builder = img_builder.partition(*p);
	}
    }

    img_builder.build()
}

/// Writes FAT filesystem to the EFI system partition on the GPT disc image.
pub fn write_fat_fs(_gpt: GptImage) -> Result<(), BobErr> {
    Ok(())
}
