pub fn load_elf<'a>(bytes: &'a [u8]) -> Result<Elf<'a>, ParseErr> {
    Elf::<'a>::parse(bytes)
}

// ELF Header Constants

const EI_MAG0: usize = 0;
const EI_MAG1: usize = 1;
const EI_MAG2: usize = 2;
const EI_MAG3: usize = 3;
const EI_CLASS: usize = 4;
const EI_VERSION: usize = 4;
const EI_OSABI: usize = 4;
const EI_ABIVERSION: usize = 4;
const EI_PAD: usize = 4;

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
}

impl<'a> Elf<'a> {

    fn parse(bytes: &'a [u8]) -> Result<Elf<'a>, ParseErr> {
	let mut e_ident = [0;E_IDENT_SZ];
	e_ident.copy_from_slice(&bytes[EI_MAG0..E_IDENT_SZ]);

	// TODO: from_ne_bytes is almost certainly not what I want

	// Check ELF Magic Number
	if &e_ident[EI_MAG0..(EI_MAG3+1)] != [0x7F, 0x45, 0x4C, 0x46] {
	    return Err(ParseErr::MagicNumber);
	}

	let header = Header {
	    e_ident,
	};
	Ok(Elf {
	    bytes,
	    header,
	})
    }
}

mod tests {

    #[test]
    fn parse_elf() {
	
    }
}
