use std::fmt;
use std::marker::Unsize;
use std::ops::CoerceUnsized;
use std::ptr;
use std::sync::Arc;
use std::cell::RefCell;
use std::ops::Deref;
use std::ops::DerefMut;
use parking_lot::{Mutex, MutexGuard};

use crate::result::*;

macro_rules! bit_enum {
    ($name:ident ($base:ty) { $( $entry_name:ident = $entry_value:expr ),* }) => {
        #[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
        #[repr(C)]
        pub struct $name($base);
        
        impl $name {
            pub const fn from(val: $base) -> Self {
                Self(val)
            }
            
            pub const fn contains(self, other: Self) -> bool {
                (self.0 & other.0) != 0
            }

            pub const fn get(self) -> $base {
                self.0
            }
        
            $(
                pub const fn $entry_name() -> Self {
                    Self($entry_value)
                }
            )*
        }
        
        impl const core::ops::BitOr for $name {
            type Output = Self;
        
            #[inline]
            fn bitor(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }

        impl const core::ops::BitAnd for $name {
            type Output = Self;
        
            #[inline]
            fn bitand(self, other: Self) -> Self {
                Self(self.0 & other.0)
            }
        }

        impl core::ops::BitOrAssign for $name {
            #[inline]
            fn bitor_assign(&mut self, other: Self) {
                self.0 |= other.0
            }
        }
        
        impl core::ops::BitAndAssign for $name {
            #[inline]
            fn bitand_assign(&mut self, other: Self) {
                self.0 &= other.0
            }
        }
    };
}

macro_rules! bit {
    ($val:expr) => {
        (1 << ($val))
    };
}

#[macro_export]
macro_rules! write_bits {
    ($start:expr, $end:expr, $value:expr, $data:expr) => {
        $value = ($value & (!( ((1 << ($end - $start + 1)) - 1) << $start ))) | ($data << $start);
    };
}

#[macro_export]
macro_rules! read_bits {
    ($start:expr, $end:expr, $value:expr) => {
        ($value & (((1 << ($end - $start + 1)) - 1) << $start)) >> $start
    };
}

pub const fn align_up<V: Into<usize> + From<usize>>(value: V, size: usize) -> V {
    let mask = size - 1;
    V::from((value.into() + mask) & !mask)
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CString<const S: usize> {
    pub c_str: [u8; S]
}

impl<const S: usize> fmt::Display for CString<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_data = match self.get_str() {
            Ok(got_str) => got_str,
            Err(_) => ""
        };
        write!(f, "{}", str_data)
    }
}

impl<const S: usize> fmt::Debug for CString<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_data = match self.get_str() {
            Ok(got_str) => got_str,
            Err(_) => ""
        };
        write!(f, "\"{}\"", str_data)
    }
}

impl<const S: usize> PartialEq for CString<S> {
    fn eq(&self, other: &Self) -> bool {
        if let Ok(self_str) = self.get_str() {
            if let Ok(other_str) = other.get_str() {
                return self_str == other_str;
            }
        }
        false
    }
}

impl<const S: usize> Eq for CString<S> {}

impl<const S: usize> Default for CString<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const S: usize> CString<S> {
    pub const fn new() -> Self {
        Self { c_str: [0; S] }
    }

    pub fn from_str(string: &str) -> Result<Self> {
        let mut cstr = Self::new();
        cstr.set_str(string)?;
        Ok(cstr)
    }

    pub fn from_string(string: String) -> Result<Self> {
        let mut cstr = Self::new();
        cstr.set_string(string)?;
        Ok(cstr)
    }

    fn copy_str_to(string: &str, ptr: *mut u8, ptr_len: usize) -> Result<()> {
        unsafe {
            ptr::write_bytes(ptr, 0, ptr_len);
            if !string.is_empty() {
                ptr::copy(string.as_ptr(), ptr, core::cmp::min(ptr_len - 1, string.len()));
            }
        }
        Ok(())
    }
    
    fn copy_string_to(string: String, ptr: *mut u8, ptr_len: usize) -> Result<()> {
        unsafe {
            ptr::write_bytes(ptr, 0, ptr_len);
            if !string.is_empty() {
                ptr::copy(string.as_ptr(), ptr, core::cmp::min(ptr_len - 1, string.len()));
            }
        }
        Ok(())
    }
    
    fn read_str_from(ptr: *const u8, ptr_len: usize) -> Result<&'static str> {
        unsafe {
            match core::str::from_utf8(core::slice::from_raw_parts(ptr, ptr_len)) {
                Ok(name) => Ok(name.trim_matches('\0')),
                // TODO
                Err(_) => Err(ResultCode::new(0xBABA))
            }
        }
    }
    
    fn read_string_from(ptr: *const u8, ptr_len: usize) -> Result<String> {
        Ok(String::from(Self::read_str_from(ptr, ptr_len)?))
    }

    pub fn set_str(&mut self, string: &str) -> Result<()> {
        Self::copy_str_to(string, &mut self.c_str as *mut _ as *mut u8, S)
    }

    pub fn set_string(&mut self, string: String) -> Result<()> {
        Self::copy_string_to(string, &mut self.c_str as *mut _ as *mut u8, S)
    }

    pub fn get_str(&self) -> Result<&'static str> {
        Self::read_str_from(&self.c_str as *const _ as *const u8, S)
    }

    pub fn get_string(&self) -> Result<String> {
        Self::read_string_from(&self.c_str as *const _ as *const u8, S)
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct CString16<const S: usize> {
    pub c_str: [u16; S]
}

impl<const S: usize> fmt::Display for CString16<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_data = match self.get_string() {
            Ok(got_str) => got_str,
            Err(_) => String::new()
        };
        write!(f, "{}", str_data)
    }
}

impl<const S: usize> fmt::Debug for CString16<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str_data = match self.get_string() {
            Ok(got_str) => got_str,
            Err(_) => String::new()
        };
        write!(f, "\"{}\"", str_data)
    }
}

impl<const S: usize> PartialEq for CString16<S> {
    fn eq(&self, other: &Self) -> bool {
        if let Ok(self_str) = self.get_string() {
            if let Ok(other_str) = other.get_string() {
                return self_str == other_str;
            }
        }
        false
    }
}

impl<const S: usize> Eq for CString16<S> {}

impl<const S: usize> Default for CString16<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const S: usize> CString16<S> {
    pub const fn new() -> Self {
        Self { c_str: [0; S] }
    }

    pub fn from_str(string: &str) -> Result<Self> {
        let mut cstr = Self::new();
        cstr.set_str(string)?;
        Ok(cstr)
    }

    pub fn from_string(string: String) -> Result<Self> {
        let mut cstr = Self::new();
        cstr.set_string(string)?;
        Ok(cstr)
    }

    fn copy_str_to(string: &str, ptr: *mut u16, ptr_len: usize) -> Result<()> {
        let mut encode_buf: [u16; 2] = [0; 2];
        let mut i: isize = 0;
        unsafe {
            ptr::write_bytes(ptr, 0, ptr_len);
            for ch in string.chars() {
                let enc = ch.encode_utf16(&mut encode_buf);
                *ptr.offset(i) = enc[0];

                i += 1;
                if i as usize > (ptr_len - 1) {
                    break;
                }
            }
        }
        Ok(())
    }
    
    fn read_string_from(ptr: *const u16, ptr_len: usize) -> Result<String> {
        let mut string = String::new();
        unsafe {
            let tmp_slice = core::slice::from_raw_parts(ptr, ptr_len);
            for ch_v in core::char::decode_utf16(tmp_slice.iter().cloned()) {
                if let Ok(ch) = ch_v {
                    if ch == '\0' {
                        break;
                    }
                    string.push(ch);
                }
                else {
                    break;
                }
            }
        }
        Ok(string)
    }

    pub fn set_str(&mut self, string: &str) -> Result<()> {
        Self::copy_str_to(string, &mut self.c_str as *mut _ as *mut u16, S)
    }

    pub fn set_string(&mut self, string: String) -> Result<()> {
        self.set_str(string.as_str())
    }

    pub fn get_string(&self) -> Result<String> {
        Self::read_string_from(&self.c_str as *const _ as *const u16, S)
    }
}

pub fn slice_read_data(slice: &[u8], offset: Option<usize>, len: usize) -> Result<Vec<u8>> {
    let offset_val = offset.unwrap_or(0);

    // TODO
    result_return_unless!((offset_val + len) <= slice.len(), 0xB);
    
    Ok(slice[offset_val..offset_val + len].to_vec())
}

pub fn slice_read_val<T: Copy>(slice: &[u8], offset: Option<usize>) -> Result<T> {
    let offset_val = offset.unwrap_or(0);

    // TODO
    result_return_unless!((offset_val + core::mem::size_of::<T>()) <= slice.len(), 0xB);
    
    unsafe {
        let ptr = slice.as_ptr().offset(offset_val as isize) as *const T;
        Ok(*ptr)
    }
}

pub fn slice_read_val_advance<T: Copy>(slice: &[u8], offset: &mut usize) -> Result<T> {
    let t: T = slice_read_val(slice, Some(*offset))?;
    *offset += core::mem::size_of::<T>();
    Ok(t)
}

pub fn slice_read_data_advance(slice: &[u8], offset: &mut usize, len: usize) -> Result<Vec<u8>> {
    let data = slice_read_data(slice, Some(*offset), len)?;
    *offset += len;
    Ok(data)
}

pub struct Shared<T: ?Sized>(Arc<Mutex<T>>);

impl<T: Sized> Shared<T> {
    pub fn new(t: T) -> Self {
        Self(Arc::new(Mutex::new(t)))
    }
}

impl<T: ?Sized> Shared<T> {
    #[inline]
    pub fn get(&self) -> MutexGuard<'_, T> {
        assert!(!self.0.deref().is_locked());
        self.0.deref().lock()
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Shared(self.0.clone())
    }
}

impl<T: ?Sized> Shared<T> {
    pub fn ptr_eq(&self, other: &Shared<T>) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Shared<U>> for Shared<T> {}