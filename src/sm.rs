#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(C)]
pub struct ServiceName {
    pub value: u64,
}

impl ServiceName {
    pub const fn from(value: u64) -> Self {
        Self { value: value }
    }
    
    pub const fn new(name: &str) -> Self {
        // Note: for the name to be valid, it should end with at least a NUL terminator (use the nul!("name") macro present in this crate for that)
        let value = unsafe { *(name.as_ptr() as *const u64) };
        Self::from(value)
    }

    pub const fn is_empty(&self) -> bool {
        self.value == 0
    }

    pub const fn empty() -> Self {
        Self::from(0)
    }
}