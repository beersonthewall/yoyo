#[derive(Debug)]
pub enum BobErr {
    PartitionParse,
    MissingArgument,
    IO(std::io::Error),
    ImageTooSmall,
}
