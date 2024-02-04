/// A module for implementing GUIDs
/// https://datatracker.ietf.org/doc/html/rfc4122
/// GUID's mostly follow this RFC, EXCEPT for the fact that time_low, time_mid, and time_high are little
/// endian. In the RFC these fields are stored in big-endian format, so this code would not be compatible
/// for any other uses besides in GPT disks.
/// Apple also notes this: https://developer.apple.com/library/archive/technotes/tn2166/_index.html#//apple_ref/doc/uid/DTS10003927-CH1-SUBSECTION11

#[derive(Debug)]
pub struct Guid {
    time_low: u32,
    time_mid: [u8;2],
    time_high_and_version: [u8;2],
    clock_seq_hi_and_reserved: u8,
    clock_seq_low: u8,
    node: [u8;6],
}

impl Guid {

    /// Generate a new Guid from random bytes
    /// https://datatracker.ietf.org/doc/html/rfc4122#section-4.4
    pub fn new_v4() -> Self {
	let rb: [u8;16] = rand::random();
	let mut s = Self::from_bytes(rb);
	s.clock_seq_hi_and_reserved |= (1 << 6) | (1 << 7);
	s.time_high_and_version[1] = (s.time_high_and_version[1] & 0b11110000) | 0b0100;
	s
    }

    pub fn new(time_low: u32,
	       time_mid: [u8;2],
	       time_high_and_version: [u8;2],
	       clock_seq_hi_and_reserved: u8,
	       clock_seq_low: u8,
	       node: [u8;6]) -> Self {
	Self {
	    time_low,
	    time_mid,
	    time_high_and_version,
	    clock_seq_hi_and_reserved,
	    clock_seq_low,
	    node,
	}
    }

    pub fn from_bytes(bytes: [u8;16]) -> Self {
	Self {
	    time_low: u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
	    time_mid: [bytes[4], bytes[5]],
	    time_high_and_version: [bytes[6], bytes[7]],
	    clock_seq_hi_and_reserved: bytes[8],
	    clock_seq_low: bytes[9],
	    node: [ bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],]
	}
    }

    pub fn to_bytes(&self) -> [u8;16] {
	let time_low = self.time_low.to_le_bytes();
	[
	    time_low[0],
	    time_low[1],
	    time_low[2],
	    time_low[3],
	    self.time_mid[0],
	    self.time_mid[1],
	    self.time_high_and_version[0],
	    self.time_high_and_version[1],
	    self.clock_seq_hi_and_reserved,
	    self.clock_seq_low,
	    self.node[0],
	    self.node[1],
	    self.node[2],
	    self.node[3],
	    self.node[4],
	    self.node[5],
	]
    }
}

