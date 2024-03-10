use uefi::table::boot::MemoryMap;

pub struct FrameAllocator {
    
}

impl FrameAllocator {

    pub fn new(mm: MemoryMap) -> Self {
	Self {}
    }
}

