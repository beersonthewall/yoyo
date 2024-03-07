pub fn load_elf<'a>(bytes: &'a [u8]) -> Result<Elf<'a>, ParseErr> {
    Elf::<'a>::parse(bytes)
}

// ELF Header Constants

const EI_MAG0: usize = 0;
const EI_MAG1: usize = 1;
const EI_MAG2: usize = 2;
const EI_MAG3: usize = 3;
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const EI_OSABI: usize = 7;
const EI_ABIVERSION: usize = 8;
const EI_PAD: usize = 9;
const E_IDENT_SZ: usize = 9;

/// An ELF Binary File.
pub struct Elf<'a> {
    bytes: &'a [u8],
    header: Header,
}

/// ELF 64 Header
pub struct Header {
    pub e_ident: [u8;E_IDENT_SZ],
    // pub e_typ: u16,
    // pub e_machine: u16,
    // pub e_version: u32,
    // pub e_entry: u64,
    // pub e_phoff: u64,
    // pub e_shoff: u64,
    // pub e_flags: u32,
    // pub e_ehsize: u16,
    // pub e_phentsize: u16,
}

#[derive(Debug)]
pub enum ParseErr {
    MagicNumber,
    EIClass,
    InputBounds,
}

pub enum Endianness {
    Big,
    Little,
}

impl<'a> Elf<'a> {

    fn parse(bytes: &'a [u8]) -> Result<Elf<'a>, ParseErr> {
	let header = Header::parse(bytes)?;

	Ok(Elf {
	    bytes,
	    header,
	})
    }
}

impl Header {
    fn parse(bytes: &[u8]) -> Result<Header, ParseErr> {
	let mut e_ident = [0;E_IDENT_SZ];
	if let Some(s) = bytes.get(EI_MAG0..E_IDENT_SZ) {
	    e_ident.copy_from_slice(&s);
	} else {
	    return Err(ParseErr::InputBounds);
	}

	// Check ELF Magic Number
	if &e_ident[EI_MAG0..=EI_MAG3] != [0x7F, 0x45, 0x4C, 0x46] {
	    return Err(ParseErr::MagicNumber);
	}

	// Only support 64bit ELF
	let e_typ = e_ident[EI_CLASS];
	if e_typ != 2 {
	    return Err(ParseErr::EIClass);
	}

	Ok(Header {
	    e_ident,
	})
    }
}

mod tests {

    // It's being used, but rust analyzer / flycheck / _something_ complains.
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn parse_header() {
	let mut header = [0;20];

	header[EI_MAG0] = 0x7F;
	header[EI_MAG1] = 0x45;
	header[EI_MAG2] = 0x4c;
	header[EI_MAG3] = 0x46;
	header[EI_CLASS] = 2;
	header[EI_DATA] = 1;

	let _ = Elf::parse(&header).expect("things");
    }

    #[test]
    fn bad_magic() {
	let mut header = [0;20];

	header[EI_MAG0] = 0x7F;
	header[EI_MAG1] = 0x20; // <- Bad!
	header[EI_MAG2] = 0x4c;
	header[EI_MAG3] = 0x46;
	header[EI_CLASS] = 2;
	header[EI_DATA] = 1;

	let result = Elf::parse(&header);
	assert!(result.is_err());
	assert!(matches!(result, Err(ParseErr::MagicNumber)));
    }

    #[test]
    fn bad_endianness() {
	let mut header = [0;20];

	header[EI_MAG0] = 0x7F;
	header[EI_MAG1] = 0x45;
	header[EI_MAG2] = 0x4c;
	header[EI_MAG3] = 0x46;
	header[EI_CLASS] = 42; // <- Bad!
	header[EI_DATA] = 1;

	let result = Elf::parse(&header);
	assert!(matches!(result, Err(ParseErr::EIClass)));
    }

    #[test]
    fn e_ident_bounds() {
	let header = [];
	let result = Elf::parse(&header);
	assert!(matches!(result, Err(ParseErr::InputBounds)));
    }
}
