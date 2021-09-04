//! Bindings for the Unicorn emulator.
//!

mod ffi;
pub mod unicorn_const;

mod arm;
mod arm64;
mod m68k;
mod mips;
mod ppc;
mod sparc;
mod x86;
pub use crate::{arm::*, arm64::*, m68k::*, mips::*, ppc::*, sparc::*, x86::*};

use ffi::uc_engine;
use ffi::uc_hook;
use libc::c_void;
use unicorn_const::*;

#[derive(Debug)]
pub struct Context {
    context: ffi::uc_context,
}

impl Context {
    pub fn new() -> Self {
        Context { context: 0 }
    }
    pub fn is_initialized(&self) -> bool {
        self.context != 0
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { ffi::uc_context_free(self.context) };
    }
}

#[derive(Clone, Copy)]
pub struct Handle {
    pub inner_handle: uc_engine
}

impl Handle {
    pub fn new(inner_handle: uc_engine) -> Self {
        Self {
            inner_handle: inner_handle
        }
    }

    /// Returns a vector with the memory regions that are mapped in the emulator.
    pub fn mem_regions(&self) -> Result<Vec<MemRegion>, uc_error> {
        let mut nb_regions: u32 = 0;
        let mut p_regions: *const MemRegion = std::ptr::null_mut();
        let err = unsafe { ffi::uc_mem_regions(self.inner_handle, &mut p_regions, &mut nb_regions) };
        if err == uc_error::OK {
            let mut regions = Vec::new();
            for i in 0..nb_regions {
                regions.push(unsafe { std::mem::transmute_copy(&*p_regions.offset(i as isize)) });
            }
            unsafe { libc::free(p_regions as _) };
            Ok(regions)
        } else {
            Err(err)
        }
    }

    /// Read a range of bytes from memory at the specified address.
    pub fn mem_read(&self, address: u64, buf: &mut [u8]) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_read(self.inner_handle, address, buf.as_mut_ptr(), buf.len()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    pub fn mem_read_val<T>(&self, address: u64) -> Result<T, uc_error> {
        let mut t: T = unsafe { core::mem::zeroed() };
        let err = unsafe { ffi::uc_mem_read(self.inner_handle, address, &mut t as *mut T as *mut u8, core::mem::size_of::<T>()) };
        if err == uc_error::OK {
            Ok(t)
        } else {
            Err(err)
        }
    }

    /// Return a range of bytes from memory at the specified address as vector.
    pub fn mem_read_as_vec(&self, address: u64, size: usize) -> Result<Vec<u8>, uc_error> {
        let mut buf = vec![0; size];
        let err = unsafe { ffi::uc_mem_read(self.inner_handle, address, buf.as_mut_ptr(), size) };
        if err == uc_error::OK {
            Ok(buf)
        } else {
            Err(err)
        }
    }

    pub fn mem_write(&mut self, address: u64, bytes: &[u8]) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_write(self.inner_handle, address, bytes.as_ptr(), bytes.len()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    pub fn mem_write_val<T>(&self, address: u64, val: T) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_write(self.inner_handle, address, &val as *const T as *const u8, core::mem::size_of::<T>()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Map an existing memory region in the emulator at the specified address.
    ///
    /// This function is marked unsafe because it is the responsibility of the caller to
    /// ensure that `size` matches the size of the passed buffer, an invalid `size` value will
    /// likely cause a crash in unicorn.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    ///
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    ///
    /// `ptr` is a pointer to the provided memory region that will be used by the emulator.
    pub fn mem_map_ptr(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
        ptr: *mut c_void,
    ) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_map_ptr(self.inner_handle, address, size, perms.bits(), ptr) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Map a memory region in the emulator at the specified address.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_map(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
    ) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_map(self.inner_handle, address, size, perms.bits()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Unmap a memory region.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_unmap(&mut self, address: u64, size: usize) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_unmap(self.inner_handle, address, size) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Set the memory permissions for an existing memory region.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_protect(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
    ) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_mem_protect(self.inner_handle, address, size, perms.bits()) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Write a value to a register.
    pub fn reg_write<U>(&mut self, regid: i32, value: U) -> Result<(), uc_error> {
        let err =
            unsafe { ffi::uc_reg_write(self.inner_handle, regid, &value as *const _ as *const c_void) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Read a value from a register.
    pub fn reg_read<U>(&self, regid: i32) -> Result<U, uc_error> {
        let mut value: U = unsafe { core::mem::zeroed() };
        let err =
            unsafe { ffi::uc_reg_read(self.inner_handle, regid, &mut value as *mut _ as *mut c_void) };
        if err == uc_error::OK {
            Ok(value)
        } else {
            Err(err)
        }
    }

    /// Allocate and return an empty Unicorn context.
    ///
    /// To be populated via context_save.
    pub fn context_alloc(&self) -> Result<Context, uc_error> {
        let mut empty_context: ffi::uc_context = Default::default();
        let err = unsafe { ffi::uc_context_alloc(self.inner_handle, &mut empty_context) };
        if err == uc_error::OK {
            Ok(Context {
                context: empty_context,
            })
        } else {
            Err(err)
        }
    }

    /// Save current Unicorn context to previously allocated Context struct.
    pub fn context_save(&self, context: &mut Context) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_context_save(self.inner_handle, context.context) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Allocate and return a Context struct initialized with the current CPU context.
    ///
    /// This can be used for fast rollbacks with context_restore.
    /// In case of many non-concurrent context saves, use context_alloc and *_save
    /// individually to avoid unnecessary allocations.
    pub fn context_init(&self) -> Result<Context, uc_error> {
        let mut new_context: ffi::uc_context = Default::default();
        let err = unsafe { ffi::uc_context_alloc(self.inner_handle, &mut new_context) };
        if err != uc_error::OK {
            return Err(err);
        }
        let err = unsafe { ffi::uc_context_save(self.inner_handle, new_context) };
        if err == uc_error::OK {
            Ok(Context {
                context: new_context,
            })
        } else {
            unsafe { ffi::uc_context_free(new_context) };
            Err(err)
        }
    }

    /// Restore a previously saved Unicorn context.
    ///
    /// Perform a quick rollback of the CPU context, including registers and some
    /// internal metadata. Contexts may not be shared across engine instances with
    /// differing arches or modes. Memory has to be restored manually, if needed.
    pub fn context_restore(&self, context: &Context) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_context_restore(self.inner_handle, context.context) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Emulate machine code for a specified duration.
    ///
    /// `begin` is the address where to start the emulation. The emulation stops if `until`
    /// is hit. `timeout` specifies a duration in microseconds after which the emulation is
    /// stopped (infinite execution if set to 0). `count` is the maximum number of instructions
    /// to emulate (emulate all the available instructions if set to 0).
    pub fn emu_start(
        &mut self,
        begin: u64,
        until: u64,
        timeout: u64,
        count: usize,
    ) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_emu_start(self.inner_handle, begin, until, timeout, count as _) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Stop the emulation.
    ///
    /// This is usually called from callback function in hooks.
    /// NOTE: For now, this will stop the execution only after the current block.
    pub fn emu_stop(&mut self) -> Result<(), uc_error> {
        let err = unsafe { ffi::uc_emu_stop(self.inner_handle) };
        if err == uc_error::OK {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Query the internal status of the engine.
    ///
    /// supported: MODE, PAGE_SIZE, ARCH
    pub fn query(&self, query: Query) -> Result<usize, uc_error> {
        let mut result: usize = Default::default();
        let err = unsafe { ffi::uc_query(self.inner_handle, query, &mut result) };
        if err == uc_error::OK {
            Ok(result)
        } else {
            Err(err)
        }
    }
}

pub struct Engine {
    pub handle: Handle,
    pub code_hooks: Vec<(Box<dyn Fn(Handle, u64, usize) + Send + Sync>, uc_hook)>,
    pub invalid_memory_access_hooks: Vec<(Box<dyn Fn(Handle, MemType, u64, usize, u64) + Send + Sync>, uc_hook)>,
    pub invalid_insn_hooks: Vec<(Box<dyn Fn(Handle) + Send + Sync>, uc_hook)>,
    pub intr_hooks: Vec<(Box<dyn Fn(Handle, u32) + Send + Sync>, uc_hook)>
}

unsafe extern "C" fn code_hook_impl(engine: uc_engine, address: u64, size: u32, user_data: *mut u8) {
    let handle = Handle::new(engine);
    let callback = &*(user_data as *mut Box<dyn Fn(Handle, u64, usize) + Send + Sync>);
    callback(handle, address, size as usize);
}

unsafe extern "C" fn invalid_memory_access_hook_impl(engine: uc_engine, mem_type: MemType, address: u64, size: u32, value: u64, user_data: *mut u8) {
    let handle = Handle::new(engine);
    let callback = &*(user_data as *mut Box<dyn Fn(Handle, MemType, u64, usize, u64) + Send + Sync>);
    callback(handle, mem_type, address, size as usize, value);
}

unsafe extern "C" fn invalid_insn_hook_impl(engine: uc_engine, user_data: *mut u8) {
    let handle = Handle::new(engine);
    let callback = &*(user_data as *mut Box<dyn Fn(Handle) + Send + Sync>);
    callback(handle);
}

unsafe extern "C" fn intr_hook_impl(engine: uc_engine, intr_no: u32, user_data: *mut u8) {
    let handle = Handle::new(engine);
    let callback = &*(user_data as *mut Box<dyn Fn(Handle, u32) + Send + Sync>);
    callback(handle, intr_no);
}

impl Engine {
    /// Create a new instance of the unicorn engine for the specified architecture
    /// and hardware mode.
    pub fn new(arch: Arch, mode: Mode) -> Result<Self, uc_error> {
        let mut handle: uc_engine = std::ptr::null_mut();
        let err = unsafe { ffi::uc_open(arch, mode, &mut handle) };
        if err == uc_error::OK {
            Ok(Self {
                handle: Handle::new(handle),
                code_hooks: Vec::new(),
                invalid_memory_access_hooks: Vec::new(),
                invalid_insn_hooks: Vec::new(),
                intr_hooks: Vec::new()
            })
        } else {
            Err(err)
        }
    }

    pub fn add_code_hook<F: Fn(Handle, u64, usize) + Send + Sync + 'static>(&mut self, f: F, begin: u64, end: u64) -> Result<uc_hook, uc_error> {
        unsafe {
            let mut hook: uc_hook = core::ptr::null_mut();
            let index = self.code_hooks.len();
            self.code_hooks.push((Box::new(f), hook));
            let (callback_ref, _) = &mut self.code_hooks[index];
            let err = ffi::uc_hook_add(self.handle.inner_handle, &mut hook as *mut _, HookType::CODE, code_hook_impl as *mut c_void, callback_ref as *mut _ as *mut c_void, begin, end);
            if err == uc_error::OK {
                Ok(hook)
            }
            else {
                let _ = self.code_hooks.remove(index);
                Err(err)
            }
        }
    }

    pub fn add_invalid_memory_access_hook<F: Fn(Handle, MemType, u64, usize, u64) + Send + Sync + 'static>(&mut self, f: F, begin: u64, end: u64) -> Result<uc_hook, uc_error> {
        unsafe {
            let mut hook: uc_hook = core::ptr::null_mut();
            let index = self.invalid_memory_access_hooks.len();
            self.invalid_memory_access_hooks.push((Box::new(f), hook));
            let (callback_ref, _) = &mut self.invalid_memory_access_hooks[index];
            let err = ffi::uc_hook_add(self.handle.inner_handle, &mut hook as *mut _, HookType::MEM_INVALID, invalid_memory_access_hook_impl as *mut c_void, callback_ref as *mut _ as *mut c_void, begin, end);
            if err == uc_error::OK {
                Ok(hook)
            }
            else {
                let _ = self.invalid_memory_access_hooks.remove(index);
                Err(err)
            }
        }
    }

    pub fn add_invalid_insn_hook<F: Fn(Handle) + Send + Sync + 'static>(&mut self, f: F, begin: u64, end: u64) -> Result<uc_hook, uc_error> {
        unsafe {
            let mut hook: uc_hook = core::ptr::null_mut();
            let index = self.invalid_insn_hooks.len();
            self.invalid_insn_hooks.push((Box::new(f), hook));
            let (callback_ref, _) = &mut self.invalid_insn_hooks[index];
            let err = ffi::uc_hook_add(self.handle.inner_handle, &mut hook as *mut _, HookType::INSN_INVALID, invalid_insn_hook_impl as *mut c_void, callback_ref as *mut _ as *mut c_void, begin, end);
            if err == uc_error::OK {
                Ok(hook)
            }
            else {
                let _ = self.invalid_insn_hooks.remove(index);
                Err(err)
            }
        }
    }

    pub fn add_intr_hook<F: Fn(Handle, u32) + Send + Sync + 'static>(&mut self, f: F, begin: u64, end: u64) -> Result<uc_hook, uc_error> {
        unsafe {
            let mut hook: uc_hook = core::ptr::null_mut();
            let index = self.intr_hooks.len();
            self.intr_hooks.push((Box::new(f), hook));
            let (callback_ref, _) = &mut self.intr_hooks[index];
            let err = ffi::uc_hook_add(self.handle.inner_handle, &mut hook as *mut _, HookType::INTR, intr_hook_impl as *mut c_void, callback_ref as *mut _ as *mut c_void, begin, end);
            if err == uc_error::OK {
                Ok(hook)
            }
            else {
                let _ = self.intr_hooks.remove(index);
                Err(err)
            }
        }
    }

    /// Remove a hook.
    ///
    /// `hook` is the value returned by `add_*_hook` functions.
    pub fn remove_hook(&mut self, hook: uc_hook) -> Result<(), uc_error> {
        let err: uc_error;
        let mut found = false;

        for i in 0..self.code_hooks.len() {
            let (_, c_hook) = self.code_hooks[i];
            if hook == c_hook {
                found = true;
                let _ = self.code_hooks.remove(i);
                break;
            }
        }
        for i in 0..self.invalid_memory_access_hooks.len() {
            let (_, c_hook) = self.invalid_memory_access_hooks[i];
            if hook == c_hook {
                found = true;
                let _ = self.invalid_memory_access_hooks.remove(i);
                break;
            }
        }
        for i in 0..self.invalid_insn_hooks.len() {
            let (_, c_hook) = self.invalid_insn_hooks[i];
            if hook == c_hook {
                found = true;
                let _ = self.invalid_insn_hooks.remove(i);
                break;
            }
        }
        for i in 0..self.intr_hooks.len() {
            let (_, c_hook) = self.intr_hooks[i];
            if hook == c_hook {
                found = true;
                let _ = self.intr_hooks.remove(i);
                break;
            }
        }

        if found {
            err = unsafe { ffi::uc_hook_del(self.handle.inner_handle, hook) };
        }
        else {
            err = uc_error::HOOK;
        }

        if err == uc_error::OK {
            Ok(())
        }
        else {
            Err(err)
        }
    }

    /// Returns a vector with the memory regions that are mapped in the emulator.
    pub fn mem_regions(&self) -> Result<Vec<MemRegion>, uc_error> {
        self.handle.mem_regions()
    }

    /// Read a range of bytes from memory at the specified address.
    pub fn mem_read(&self, address: u64, buf: &mut [u8]) -> Result<(), uc_error> {
        self.handle.mem_read(address, buf)
    }

    pub fn mem_read_val<T>(&self, address: u64) -> Result<T, uc_error> {
        self.handle.mem_read_val(address)
    }

    /// Return a range of bytes from memory at the specified address as vector.
    pub fn mem_read_as_vec(&self, address: u64, size: usize) -> Result<Vec<u8>, uc_error> {
        self.handle.mem_read_as_vec(address, size)
    }

    pub fn mem_write(&mut self, address: u64, bytes: &[u8]) -> Result<(), uc_error> {
        self.handle.mem_write(address, bytes)
    }

    pub fn mem_write_val<T>(&self, address: u64, val: T) -> Result<(), uc_error> {
        self.handle.mem_write_val(address, val)
    }

    /// Map an existing memory region in the emulator at the specified address.
    ///
    /// This function is marked unsafe because it is the responsibility of the caller to
    /// ensure that `size` matches the size of the passed buffer, an invalid `size` value will
    /// likely cause a crash in unicorn.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    ///
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    ///
    /// `ptr` is a pointer to the provided memory region that will be used by the emulator.
    pub fn mem_map_ptr(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
        ptr: *mut c_void,
    ) -> Result<(), uc_error> {
        self.handle.mem_map_ptr(address, size, perms, ptr)
    }

    /// Map a memory region in the emulator at the specified address.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_map(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
    ) -> Result<(), uc_error> {
        self.handle.mem_map(address, size, perms)
    }

    /// Unmap a memory region.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_unmap(&mut self, address: u64, size: usize) -> Result<(), uc_error> {
        self.handle.mem_unmap(address, size)
    }

    /// Set the memory permissions for an existing memory region.
    ///
    /// `address` must be aligned to 4kb or this will return `Error::ARG`.
    /// `size` must be a multiple of 4kb or this will return `Error::ARG`.
    pub fn mem_protect(
        &mut self,
        address: u64,
        size: usize,
        perms: Permission,
    ) -> Result<(), uc_error> {
        self.handle.mem_protect(address, size, perms)
    }

    /// Write a value to a register.
    pub fn reg_write<U>(&mut self, regid: i32, value: U) -> Result<(), uc_error> {
        self.handle.reg_write(regid, value)
    }

    /// Read a value from a register.
    pub fn reg_read<U>(&self, regid: i32) -> Result<U, uc_error> {
        self.handle.reg_read(regid)
    }

    /// Allocate and return an empty Unicorn context.
    ///
    /// To be populated via context_save.
    pub fn context_alloc(&self) -> Result<Context, uc_error> {
        self.handle.context_alloc()
    }

    /// Save current Unicorn context to previously allocated Context struct.
    pub fn context_save(&self, context: &mut Context) -> Result<(), uc_error> {
        self.handle.context_save(context)
    }

    /// Allocate and return a Context struct initialized with the current CPU context.
    ///
    /// This can be used for fast rollbacks with context_restore.
    /// In case of many non-concurrent context saves, use context_alloc and *_save
    /// individually to avoid unnecessary allocations.
    pub fn context_init(&self) -> Result<Context, uc_error> {
        self.handle.context_init()
    }

    /// Restore a previously saved Unicorn context.
    ///
    /// Perform a quick rollback of the CPU context, including registers and some
    /// internal metadata. Contexts may not be shared across engine instances with
    /// differing arches or modes. Memory has to be restored manually, if needed.
    pub fn context_restore(&self, context: &Context) -> Result<(), uc_error> {
        self.handle.context_restore(context)
    }

    /// Emulate machine code for a specified duration.
    ///
    /// `begin` is the address where to start the emulation. The emulation stops if `until`
    /// is hit. `timeout` specifies a duration in microseconds after which the emulation is
    /// stopped (infinite execution if set to 0). `count` is the maximum number of instructions
    /// to emulate (emulate all the available instructions if set to 0).
    pub fn emu_start(
        &mut self,
        begin: u64,
        until: u64,
        timeout: u64,
        count: usize,
    ) -> Result<(), uc_error> {
        self.handle.emu_start(begin, until, timeout, count)
    }

    /// Stop the emulation.
    ///
    /// This is usually called from callback function in hooks.
    /// NOTE: For now, this will stop the execution only after the current block.
    pub fn emu_stop(&mut self) -> Result<(), uc_error> {
        self.handle.emu_stop()
    }

    /// Query the internal status of the engine.
    ///
    /// supported: MODE, PAGE_SIZE, ARCH
    pub fn query(&self, query: Query) -> Result<usize, uc_error> {
        self.handle.query(query)
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { ffi::uc_close(self.handle.inner_handle) };
    }
}