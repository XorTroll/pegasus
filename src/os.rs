use crate::util::CString;

#[repr(C)]
pub struct ThreadType {
    pub other_fields: [u8; 0x180],
    pub thread_name: CString<0x20>,
    pub thread_name_pointer: *mut u8
}

// Note: https://switchbrew.org/wiki/Thread_Local_Region

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ThreadLocalRegion {
    pub msg_buffer: [u8; 0x100],
    pub disable_counter: u16,
    pub interrupt_flag: u16,
    pub reserved_1: [u8; 0x4],
    pub reserved_2: [u8; 0x78],
    pub tls: [u8; 0x50],
    pub locale_ptr: *mut u8,
    pub errno_val: i64,
    pub thread_data: [u8; 0x8],
    pub eh_globals: [u8; 0x8],
    pub thread_ptr: *mut u8,
    pub thread_ref: *mut ThreadType,
}