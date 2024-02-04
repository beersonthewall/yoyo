mod cmd;
mod err;
mod fat;
mod gpt;
mod guid;

use clap::{
    arg, command, Arg, Command, value_parser,
    error::ErrorKind,
};
use cmd::{create_disk_image, write_fat_fs};
use err::BobErr;
use gpt::{Partition, PartitionBuilder, PartitionType};

#[derive(Clone)]
struct PartitionParser;

impl clap::builder::TypedValueParser for PartitionParser {
    type Value = Partition;

    fn parse_ref(
	&self,
	cmd: &Command,
	arg: Option<&Arg>,
	value: &std::ffi::OsStr)
	-> Result<Self::Value, clap::Error> {

	if value.is_empty() {
	    return Err(clap::Error::new(ErrorKind::InvalidValue));
	}

	if let Some(val) = value.to_str() {
	    let mut partition_builder = PartitionBuilder::new();
	    for field in val.split(',') {
		// fields "should be" one of:
		// - t=<value>
		// - so=<value>
		// - eo=<value>
		// where t, so, and eo stand for type, start offset, and end offset respectively
		if let Some((key, value)) = field.split_once('=') {
		    if key == "t" {
			partition_builder = partition_builder.partition_type(PartitionType::EFISystem);
		    } else if key == "so" {
			let so = value.trim().parse::<usize>().map_err(|_| clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd))?;
			partition_builder = partition_builder.start_offset(so);
		    } else if key == "eo" {
			let eo = value.parse::<usize>().map_err(|_| clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd))?;
			partition_builder = partition_builder.end_offset(eo);
		    }
		}
	    }
	    return partition_builder.build().map_err(|_| clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd));
	} else {
	    return Err(clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd));
	}
    }
}

fn main() -> Result<(), BobErr> {
    let matches = command!()
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
	    Command::new("create")
		.about("Create a new disk image")
		.args(&[
		    arg!(-o --output <FILE> "Output filename"),
		    arg!(-s --size <SIZE> "Total size of the desired disk image")
			.required(true)
			.value_parser(value_parser!(usize)),
		    Arg::new("partition").short('p').required(false)
			.action(clap::ArgAction::Append)
			.value_parser(PartitionParser {})
			.value_name("t=<type>,so=<offset>,eo=<offset>")
			.help("A GPT partition specification. t=<val> specifies the parition type, so=<val> is the start offset, eo=<val> is the end offset.")
		])
	)
	.subcommand(
	    Command::new("update")
		.about("Update a disk image")
		.arg(arg!(-i --image <FILE> "Disk image file to update"))
	)
	.get_matches();

    if let Some(sub_matches) = matches.subcommand_matches("create") {
        return write_fat_fs(create_disk_image(sub_matches)?);
    }

    if let Some(_sub_matches) = matches.subcommand_matches("update") {
        todo!("Updating GPT disk images is not yet implemented :(");
    }

    return Ok(());
}
