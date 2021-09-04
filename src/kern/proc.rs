use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::collections::BTreeMap;
use std::path::Prefix;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::thread::Builder;
use std::thread::JoinHandle;
use std::thread::Thread;
use std::time;
use parking_lot::Mutex;
use rsevents::AutoResetEvent;
use rsevents::Awaitable;
use rsevents::ManualResetEvent;
use rsevents::State;
use parking_lot::lock_api::RawMutex as RawMutexTrait;
use parking_lot::RawMutex;
use crate::emu::cpu;
use crate::ldr::npdm::NpdmData;
use crate::util::Shared;
use crate::result::*;

use super::KAutoObject;
use super::KSynchronizationObject;
use super::thread::KThread;
use super::thread::get_current_thread;
use super::result;

// KProcess

pub struct KProcess {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    pub cpu_ctx: cpu::Context,
    pub npdm: NpdmData
}

impl KAutoObject for KProcess {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }

    fn destroy(&mut self) {
    }
}

impl KSynchronizationObject for KProcess {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }

    fn is_signaled(&self) -> bool {
        false
    }
}

impl KProcess {
    pub fn new(cpu_ctx: cpu::Context, npdm: NpdmData) -> Shared<Self> {
        Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            cpu_ctx: cpu_ctx,
            npdm: npdm
        })
    }

    pub fn create_main_thread(proc: &Shared<KProcess>, host_thread_name: String, entry_addr: u64) -> Result<Shared<KThread>> {
        let priority = proc.get().npdm.meta.main_thread_priority as i32;
        let cpu_core = proc.get().npdm.meta.main_thread_cpu_core as i32;
        let stack_size = proc.get().npdm.meta.main_thread_stack_size as usize;
        KThread::new(Some(proc.clone()), host_thread_name, priority, cpu_core, Some((entry_addr, stack_size)))
    }
}

pub fn has_current_process() -> bool {
    get_current_thread().get().owner_process.is_some()
}

pub fn get_current_process() -> Shared<KProcess> {
    assert!(has_current_process());

    get_current_thread().get().owner_process.as_ref().unwrap().clone()
}