use crate::util::CString;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(C)]
pub struct FirmwareVersion {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
    pub pad_1: u8,
    pub revision_major: u8,
    pub revision_minor: u8,
    pub pad_2: u8,
    pub pad_3: u8,
    pub platform: CString<0x20>,
    pub version_hash: CString<0x40>,
    pub display_version: CString<0x18>,
    pub display_title: CString<0x80>
}