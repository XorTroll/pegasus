use std::fmt;
use std::num::NonZeroUsize;
use std::ptr;
use std::any::Any;
use std::sync::Arc;
use std::thread;
use parking_lot::lock_api::{GetThreadId, RawReentrantMutex, RawMutex as RawMutexTrait};
use parking_lot::{RawMutex, Mutex, MutexGuard};
use crate::kern::proc::{get_current_process, has_current_process};
use crate::kern::thread::{get_current_thread, has_current_thread};
use crate::result;
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

pub struct ThreadIdStub {}

unsafe impl GetThreadId for ThreadIdStub {
    const INIT: Self = ThreadIdStub {};

    fn nonzero_thread_id(&self) -> NonZeroUsize {
        // Note: would be cool to use KThread's ID, but this might be accessed from host threads without a KThread object, like the main thread of this project
        NonZeroUsize::new(thread::current().id().as_u64().get() as usize).unwrap()
    }
}

pub type Lock = RawMutex;
pub type RecursiveLock = RawReentrantMutex<RawMutex, ThreadIdStub>;

pub struct LockGuard<'a> {
    lock: &'a mut Lock
}

impl<'a> LockGuard<'a> {
    pub fn new(lock: &'a mut Lock) -> Self {
        lock.lock();

        Self {
            lock: lock
        }
    }
}

impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            self.lock.unlock();
        }
    }
}

pub struct RecursiveLockGuard<'a> {
    lock: &'a mut RecursiveLock
}

impl<'a> RecursiveLockGuard<'a> {
    pub fn new(lock: &'a mut RecursiveLock) -> Self {
        lock.lock();

        Self {
            lock: lock
        }
    }
}

impl<'a> Drop for RecursiveLockGuard<'a> {
    fn drop(&mut self) {
        unsafe {
            self.lock.unlock();
        }
    }
}

pub const fn new_lock() -> Lock {
    Lock::INIT
}

pub const fn new_recursive_lock() -> RecursiveLock {
    RecursiveLock::INIT
}

static mut G_LOG_LOCK: RecursiveLock = new_recursive_lock();

pub fn make_log_guard<'a>() -> RecursiveLockGuard<'a> {
    unsafe {
        RecursiveLockGuard::new(&mut G_LOG_LOCK)
    }
}

pub fn log_line_msg(msg: String) {
    let _guard = make_log_guard();

    let process_name = match has_current_process() {
        true => String::from(get_current_process().get().npdm.meta.name.get_str().unwrap()),
        false => String::from("Host~pegasus")
    };
    let thread_name = match has_current_thread() {
        true => String::from(get_current_thread().get().host_thread_handle.as_ref().unwrap().thread().name().unwrap()),
        false => format!("Host~{}", std::thread::current().name().unwrap())
    };

    println!("[{} -> {}] {}", process_name, thread_name, msg);
}

#[macro_export]
macro_rules! log_line {
    ($($arg:tt)*) => {{
        let log_msg = format!($($arg)*);
        $crate::util::log_line_msg(log_msg);
    }};
}

pub fn align_up<V: Into<usize> + From<usize>>(value: V, size: usize) -> V {
    // TODO: make const?
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
    result_return_unless!((offset_val + len) <= slice.len(), 0xBEEF);
    
    Ok(slice[offset_val..offset_val + len].to_vec())
}

pub fn slice_read_val<T: Copy>(slice: &[u8], offset: Option<usize>) -> Result<T> {
    let offset_val = offset.unwrap_or(0);

    // TODO
    result_return_unless!((offset_val + core::mem::size_of::<T>()) <= slice.len(), 0xBEEF);
    
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

pub fn trailing_zero_count(val: u64) -> u64 {
    for i in 0..64 {
        if (val & bit!(i)) != 0 {
            return i;
        }
    }

    return 64;
}

pub type Shared<T> = Arc<Mutex<T>>;
pub type SharedAny = Arc<dyn Any + Send + Sync>;

#[inline]
pub fn make_shared<T: Sized>(t: T) -> Shared<T> {
    Arc::new(Mutex::new(t))
}

pub trait SharedObject<T: ?Sized> {
    fn get(&self) -> MutexGuard<'_, T>;
    fn ptr_eq(&self, other: &Shared<T>) -> bool;
}

pub trait SharedCast {
    fn as_any(&self) -> SharedAny;
    fn cast<U: Any + Send + Sync>(&self) -> Result<Shared<U>> where Self: Any + Send + Sync + Sized;
}

impl<T: ?Sized> SharedObject<T> for Shared<T> {
    #[inline]
    fn get(&self) -> MutexGuard<'_, T> {
        if self.is_locked() {
            panic!("Attempted to access an already locked Shared<{}>", std::any::type_name::<T>());
        }

        self.lock()
    }

    #[inline]
    fn ptr_eq(&self, other: &Shared<T>) -> bool {
        Arc::ptr_eq(self, other)
    }
}

impl<T: Any + Send + Sync> SharedObject<T> for SharedAny {
    fn get(&self) -> MutexGuard<'_, T> {
        panic!("Attempted to get() from a SharedAny object");
    }

    fn ptr_eq(&self, other: &Shared<T>) -> bool {
        Arc::ptr_eq(self, &other.as_any())
    }
}

impl SharedCast for SharedAny {
    fn as_any(&self) -> SharedAny {
        self.clone()
    }

    fn cast<U: Any + Send + Sync>(&self) -> Result<Shared<U>> where Self: Any + Send + Sync + Sized {
        match self.clone().downcast::<Mutex<U>>() {
            Ok(shared) => Ok(shared),
            Err(_) => result::ResultInvalidCast::make_err()
        }
    }
}

impl<T: Any + Send + Sync> SharedCast for Shared<T> {
    fn as_any(&self) -> SharedAny {
        self.clone()
    }

    fn cast<U: Any + Send + Sync>(&self) -> Result<Shared<U>> where Self: Any + Send + Sync + Sized {
        self.as_any().cast()
    }
}