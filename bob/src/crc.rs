/// Naive CRC 32 implementation.
/// I don't think this is correct, but I want to make progress elsewhere
pub fn crc32(bytes: &[u8]) -> u32 {
    // Extend the message by 32 bits
    let bytes = [bytes, &[0;4]].concat();
    let polynomial = 0x04C11DB7;
    let mut reg: u32 = 0;
    const TOP_BIT: u32 = 1 << 31;

    for i in 0..bytes.len() {
	reg ^= (bytes[i] as u32) << 24;

	for _ in 0..8 {
	    if reg & TOP_BIT > 0 {
		reg = (reg << 1) ^ polynomial;
	    } else {
		reg = reg << 1;
	    }
	}
    }

    reg
}
