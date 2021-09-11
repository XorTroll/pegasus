pub mod result;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(C)]
pub struct ServiceName {
    pub name: [u8; 0x8]
}

impl ServiceName {
    const fn default_get(name_u8: &[u8], idx: usize) -> u8 {
        if idx < name_u8.len() {
            name_u8[idx]
        }
        else {
            0
        }
    }

    pub const fn new(name: &str) -> Self {
        let name_u8 = name.as_bytes();

        Self {
            name: [
                Self::default_get(name_u8, 0), Self::default_get(name_u8, 1),
                Self::default_get(name_u8, 2), Self::default_get(name_u8, 3),
                Self::default_get(name_u8, 4), Self::default_get(name_u8, 5),
                Self::default_get(name_u8, 6), Self::default_get(name_u8, 7)
            ]
        }
    }

    pub const fn to_str(&self) -> &str {
        unsafe {
            std::str::from_utf8_unchecked(&self.name)
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.name[0] == 0
    }

    pub const fn empty() -> Self {
        Self {
            name: [0; 0x8]
        }
    }
}