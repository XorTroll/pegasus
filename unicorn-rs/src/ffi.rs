#![allow(non_camel_case_types)]
#![allow(dead_code)]

use super::unicorn_const::*;
use libc::{c_char, c_int};
use std::ffi::c_void;

pub type uc_engine = *mut c_void;
pub type uc_hook = *mut c_void;
pub type uc_context = libc::size_t;

extern "C" {
    pub fn uc_version(major: *mut u32, minor: *mut u32) -> u32;
    pub fn uc_arch_supported(arch: Arch) -> bool;
    pub fn uc_open(arch: Arch, mode: Mode, engine: *mut uc_engine) -> uc_error;
    pub fn uc_close(engine: uc_engine) -> uc_error;
    pub fn uc_context_free(mem: uc_context) -> uc_error;
    pub fn uc_errno(engine: uc_engine) -> uc_error;
    pub fn uc_strerror(error_code: uc_error) -> *const c_char;
    pub fn uc_reg_write(engine: uc_engine, regid: c_int, value: *const c_void) -> uc_error;
    pub fn uc_reg_read(engine: uc_engine, regid: c_int, value: *mut c_void) -> uc_error;
    pub fn uc_mem_write(
        engine: uc_engine,
        address: u64,
        bytes: *const u8,
        size: usize,
    ) -> uc_error;
    pub fn uc_mem_read(
        engine: uc_engine,
        address: u64,
        bytes: *mut u8,
        size: usize,
    ) -> uc_error;
    pub fn uc_mem_map(engine: uc_engine, address: u64, size: usize, perms: u32) -> uc_error;
    pub fn uc_mem_map_ptr(
        engine: uc_engine,
        address: u64,
        size: usize,
        perms: u32,
        ptr: *mut c_void,
    ) -> uc_error;
    pub fn uc_mem_unmap(engine: uc_engine, address: u64, size: usize) -> uc_error;
    pub fn uc_mem_protect(
        engine: uc_engine,
        address: u64,
        size: usize,
        perms: u32,
    ) -> uc_error;
    pub fn uc_mem_regions(
        engine: uc_engine,
        regions: *const *const MemRegion,
        count: *mut u32,
    ) -> uc_error;
    pub fn uc_emu_start(
        engine: uc_engine,
        begin: u64,
        until: u64,
        timeout: u64,
        count: usize,
    ) -> uc_error;
    pub fn uc_emu_stop(engine: uc_engine) -> uc_error;
    pub fn uc_hook_add(
        engine: uc_engine,
        hook: *mut uc_hook,
        hook_type: HookType,
        callback: *mut c_void,
        user_data: *mut c_void,
        begin: u64,
        end: u64,
        ...
    ) -> uc_error;
    pub fn uc_hook_del(engine: uc_engine, hook: uc_hook) -> uc_error;
    pub fn uc_query(engine: uc_engine, query_type: Query, result: *mut usize) -> uc_error;
    pub fn uc_context_alloc(engine: uc_engine, context: *mut uc_context) -> uc_error;
    pub fn uc_context_save(engine: uc_engine, context: uc_context) -> uc_error;
    pub fn uc_context_restore(engine: uc_engine, context: uc_context) -> uc_error;
}