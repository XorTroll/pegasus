use std::sync::atomic::AtomicI32;
use parking_lot::Mutex;
use crate::emu::cpu;
use crate::ldr::npdm::NpdmData;
use crate::util::{Shared, SharedAny};
use crate::result::*;
use crate::result as lib_result;
use super::KAutoObject;
use super::KResourceLimit;
use super::KSynchronizationObject;
use super::ipc::{KClientPort, KClientSession, KServerPort, KServerSession};
use super::thread::{KThread, try_get_current_thread};
use super::thread::get_current_thread;
use super::svc::LimitableResource;
use super::svc::Handle;
use super::svc::CURRENT_PROCESS_PSEUDO_HANDLE;
use super::svc::CURRENT_THREAD_PSEUDO_HANDLE;
use super::result;

// KHandleTableEntry

pub struct KHandleTableEntry {
    pub linear_id: u16,
    pub obj: Option<SharedAny>
}

impl KHandleTableEntry {
    pub const MAX_LINEAR_ID: u16 = 0x8000; // Max value for a 15-bit unsigned integer, 2^15
    pub const MIN_LINEAR_ID: u16 = 1;
    pub const INVALID_LINEAR_ID: u16 = 0;

    pub const fn new() -> Self {
        Self {
            linear_id: Self::INVALID_LINEAR_ID,
            obj: None
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.linear_id == Self::INVALID_LINEAR_ID
    }
}

// ---

// KHandleTable

pub struct KHandleTable {
    entry_table: Mutex<Vec<KHandleTableEntry>>,
    used_entry_count: u32,
    linear_id_counter: u16
}

impl KHandleTable {
    pub const MAX_SIZE: usize = 0x400;

    pub const fn encode_handle(idx: u32, linear_id: u16) -> Handle {
        ((linear_id as u32) << 15) | idx
    }

    pub const fn decode_handle(handle: Handle) -> (u32, u16) {
        (handle & 0x7FFF, (handle >> 15) as u16)
    }

    pub fn new(size: usize) -> Result<Self> {
        result_return_unless!((size > 0) && (size <= Self::MAX_SIZE), result::ResultOutOfMemory);

        let mut entry_table: Vec<KHandleTableEntry> = Vec::new();
        for _ in 0..size {
            entry_table.push(KHandleTableEntry::new());
        }

        Ok(Self {
            entry_table: Mutex::new(entry_table),
            used_entry_count: 0,
            linear_id_counter: KHandleTableEntry::MIN_LINEAR_ID
        })
    }

    pub fn allocate_handle_set<K: KAutoObject + 'static>(&mut self, obj: Shared<K>) -> Result<Handle> {
        let mut entry_table = self.entry_table.lock();

        result_return_unless!(self.used_entry_count < entry_table.len() as u32, result::ResultOutOfHandles);

        for i in 0..entry_table.len() {
            let entry = &mut entry_table[i];

            if entry.is_empty() {
                entry.linear_id = self.linear_id_counter;
                self.linear_id_counter += 1;
                if self.linear_id_counter > KHandleTableEntry::MAX_LINEAR_ID {
                    self.linear_id_counter = KHandleTableEntry::MIN_LINEAR_ID;
                }

                let handle = Self::encode_handle(i as u32, entry.linear_id);
                obj.get().increment_refcount();
                entry.obj = Some(obj.as_any());
                self.used_entry_count += 1;

                return Ok(handle);
            }
        }

        result::ResultOutOfHandles::make_err()
    }

    pub fn allocate_handle(&mut self) -> Result<Handle> {
        let mut entry_table = self.entry_table.lock();

        result_return_unless!(self.used_entry_count < entry_table.len() as u32, result::ResultOutOfHandles);

        for i in 0..entry_table.len() {
            let entry = &mut entry_table[i];

            if entry.is_empty() {
                entry.linear_id = self.linear_id_counter;
                self.linear_id_counter += 1;
                if self.linear_id_counter > KHandleTableEntry::MAX_LINEAR_ID {
                    self.linear_id_counter = KHandleTableEntry::MIN_LINEAR_ID;
                }

                let handle = Self::encode_handle(i as u32, entry.linear_id);
                entry.obj = None;
                self.used_entry_count += 1;

                return Ok(handle);
            }
        }

        result::ResultOutOfHandles::make_err()
    }

    pub fn set_allocated_handle<K: KAutoObject + 'static>(&mut self, handle: Handle, obj: Shared<K>) -> Result<()> {
        let (idx, linear_id) = Self::decode_handle(handle);
        let mut entry_table = self.entry_table.lock();

        let entry = &mut entry_table[idx as usize];
        result_return_unless!(entry.linear_id == linear_id, result::ResultInvalidHandle);

        obj.get().increment_refcount();
        entry.obj = Some(obj.as_any());
        Ok(())
    }

    pub fn deallocate_handle(&mut self, handle: Handle) -> Result<()> {
        result_return_if!((handle == CURRENT_PROCESS_PSEUDO_HANDLE) || (handle == CURRENT_THREAD_PSEUDO_HANDLE), result::ResultInvalidHandle);

        let (idx, linear_id) = Self::decode_handle(handle);
        let mut entry_table = self.entry_table.lock();

        let entry = &mut entry_table[idx as usize];
        result_return_unless!(entry.linear_id == linear_id, result::ResultInvalidHandle);

        *entry = KHandleTableEntry::new();
        Ok(())
    }

    pub fn close_handle(&mut self, handle: Handle) -> Result<()> {
        result_return_if!((handle == CURRENT_PROCESS_PSEUDO_HANDLE) || (handle == CURRENT_THREAD_PSEUDO_HANDLE), result::ResultInvalidHandle);

        let (idx, linear_id) = Self::decode_handle(handle);
        let mut entry_table = self.entry_table.lock();

        let entry = &mut entry_table[idx as usize];
        result_return_unless!(entry.linear_id == linear_id, result::ResultInvalidHandle);
        result_return_unless!(entry.obj.is_some(), result::ResultInvalidHandle);

        // TODO: should decrement refcount here...?
        // entry.obj.as_ref().unwrap().cast::<dyn KAutoObject>().get().decrement_refcount();
        *entry = KHandleTableEntry::new();
        self.used_entry_count -= 1;
        Ok(())
    }

    pub fn get_handle_obj_any(&self, handle: Handle) -> Result<SharedAny> {
        let (idx, linear_id) = Self::decode_handle(handle);
        let entry_table = self.entry_table.lock();

        let entry = &entry_table[idx as usize];
        result_return_unless!(entry.linear_id == linear_id, result::ResultInvalidHandle);
        result_return_unless!(entry.obj.is_some(), result::ResultInvalidHandle);

        Ok(entry.obj.as_ref().unwrap().clone())
    }
    
    #[inline]
    pub fn get_handle_obj<K: KAutoObject + 'static>(&self, handle: Handle) -> Result<Shared<K>> {
        self.get_handle_obj_any(handle)?.cast::<K>()
    }

    pub fn get_handle_sync_obj(&self, handle: Handle) -> Result<Shared<dyn KSynchronizationObject>> {
        // Due to how great Rust is with downcasting, we have to do this with all KSynchronizationObject types. Luckily there aren't that many of them...
        let obj = self.get_handle_obj_any(handle)?;

        if let Ok(thread) = obj.cast::<KThread>() {
            return Ok(thread);
        }

        if let Ok(process) = obj.cast::<KProcess>() {
            return Ok(process);
        }

        if let Ok(server_port) = obj.cast::<KServerPort>() {
            return Ok(server_port);
        }
        if let Ok(client_port) = obj.cast::<KClientPort>() {
            return Ok(client_port);
        }

        if let Ok(server_session) = obj.cast::<KServerSession>() {
            return Ok(server_session);
        }
        if let Ok(client_session) = obj.cast::<KClientSession>() {
            return Ok(client_session);
        }

        lib_result::ResultInvalidCast::make_err()
    }
}

// ---

// KProcess

pub struct KProcess {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    pub cpu_ctx: Option<cpu::Context>,
    pub npdm: NpdmData,
    pub handle_table: KHandleTable,
    pub resource_limit: Shared<KResourceLimit>
}

impl KAutoObject for KProcess {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

impl KSynchronizationObject for KProcess {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }
}

impl KProcess {
    pub fn new(cpu_ctx: Option<cpu::Context>, npdm: NpdmData) -> Result<Shared<Self>> {
        let handle_table_size = npdm.aci0_kernel_capabilities.handle_table_size.unwrap() as usize;

        // TODO: memory?
        // TODO: make this a bit more realistic for processes, applets, applications, etc. ?
        // Note: curremntly using Ryujinx's values
        let resource_limit = KResourceLimit::new();
        resource_limit.get().set_limit_value(LimitableResource::PhysicalMemory, 0)?;
        resource_limit.get().set_limit_value(LimitableResource::Thread, 608)?;
        resource_limit.get().set_limit_value(LimitableResource::Event, 700)?;
        resource_limit.get().set_limit_value(LimitableResource::TransferMemory, 128)?;
        resource_limit.get().set_limit_value(LimitableResource::Session, 894)?;

        Ok(Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            cpu_ctx: cpu_ctx,
            npdm: npdm,
            handle_table: KHandleTable::new(handle_table_size)?,
            resource_limit: resource_limit
        }))
    }

    pub fn create_main_thread(proc: &mut Shared<KProcess>, host_thread_name: String, entry_addr: u64) -> Result<(Shared<KThread>, Handle)> {
        let priority = proc.get().npdm.meta.main_thread_priority as i32;
        let cpu_core = proc.get().npdm.meta.main_thread_cpu_core as i32;
        let stack_size = proc.get().npdm.meta.main_thread_stack_size as usize;

        let thread = KThread::new(Some(proc.clone()), host_thread_name, priority, cpu_core, Some((entry_addr, stack_size)))?;
        let thread_handle = proc.get().handle_table.allocate_handle_set(thread.clone())?;
        Ok((thread, thread_handle))
    }

    pub fn create_main_thread_host(proc: &Shared<KProcess>, host_thread_name: String) -> Result<Shared<KThread>> {
        let priority = proc.get().npdm.meta.main_thread_priority as i32;
        let cpu_core = proc.get().npdm.meta.main_thread_cpu_core as i32;

        KThread::new_host(Some(proc.clone()), host_thread_name, priority, cpu_core)
    }
}

#[inline]
pub fn has_current_process() -> bool {
    if let Some(thread) = try_get_current_thread() {
        thread.get().owner_process.is_some()
    }
    else {
        false
    }
}

#[inline]
pub fn try_get_current_process() -> Option<Shared<KProcess>> {
    if let Some(thread) = try_get_current_thread() {
        thread.get().owner_process.clone()
    }
    else {
        None
    }
}

#[inline]
pub fn get_current_process() -> Shared<KProcess> {
    assert!(has_current_process());

    get_current_thread().get().owner_process.as_ref().unwrap().clone()
}