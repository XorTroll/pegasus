pub mod npdm;

pub mod result;

bit_enum! {
    NsoFlags (u32) {
        TextCompressed = bit!(0),
        RodataCompressed = bit!(1),
        DataCompressed = bit!(2),
        TextCheckHash = bit!(3),
        RodataCheckHash = bit!(4),
        DataCheckHash = bit!(5)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct NsoSegmentHeader {
    pub file_offset: u32,
    pub memory_offset: u32,
    pub section_size: u32
}
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct NsoRodataRelativeSegmentHeader {
    pub offset: u32,
    pub size: u32
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct NsoHeader {
    pub magic: u32,
    pub version: u32,
    pub reserved_1: [u8; 4],
    pub flags: NsoFlags,
    pub text_segment: NsoSegmentHeader,
    pub module_name_offset: u32,
    pub rodata_segment: NsoSegmentHeader,
    pub module_name_size: u32,
    pub data_segment: NsoSegmentHeader,
    pub bss_size: u32,
    pub module_id: [u8; 0x20],
    pub text_file_size: u32,
    pub rodata_file_size: u32,
    pub data_file_size: u32,
    pub reserved_2: [u8; 0x1C],
    pub rodata_api_info_segment: NsoRodataRelativeSegmentHeader,
    pub rodata_dynstr_segment: NsoRodataRelativeSegmentHeader,
    pub rodata_dynsym_segment: NsoRodataRelativeSegmentHeader,
    pub text_hash: [u8; 0x20],
    pub rodata_hash: [u8; 0x20],
    pub data_hash: [u8; 0x20]
}

impl NsoHeader {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"NSO0");
}