mod cmd;
mod err;
mod gpt;

use clap::{arg, command, Arg, Command};
use cmd::create_disk_image;
use err::BobErr;

fn main() -> Result<(), BobErr> {
    let matches = command!()
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
	    Command::new("create")
		.about("Create a new disk image")
		.args(&[
		    arg!(-o --output <FILE> "Output filename"),
		    arg!(-s --size <SIZE> "Total size of the desired disk image"),
		    Arg::new("partition").short('p').required(false)
			.action(clap::ArgAction::Append)
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
        return create_disk_image(sub_matches);
    }

    if let Some(_sub_matches) = matches.subcommand_matches("update") {
        todo!("Updating GPT disk images is not yet implemented :(");
    }

    return Ok(());
}
