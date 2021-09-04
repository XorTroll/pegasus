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
use crate::util::Shared;
use crate::result::*;

use super::KAutoObject;
use super::KSynchronizationObject;
use super::proc::KProcess;
use super::proc::has_current_process;
use super::proc::get_current_process;
use super::result;

// KCriticalSection
// Note: thanks Rust for only supporting Mutex functionality through guards/wrapping objects, luckily parking_lot exposes a classic mutex type

pub struct KCriticalSection {
    lock: RawMutex,
    recursion_count: i32
}

impl KCriticalSection {
    pub fn new() -> Self {
        Self {
            lock: RawMutex::INIT,
            recursion_count: 0
        }
    }

    pub fn enter(&mut self) {
        self.lock.lock();
        self.recursion_count += 1;
    }

    pub fn leave(&mut self) {
        if self.recursion_count == 0 {
            return;
        }

        self.recursion_count -= 1;
        if self.recursion_count == 0 {
            let scheduled_cores_mask = KScheduler::select_threads();

            unsafe {
                self.lock.unlock();
            }

            let cur_thread = get_current_thread();

            let is_cur_thread_schedulable = cur_thread.get().is_schedulable;
            if is_cur_thread_schedulable {
                // TODO: EnableScheduling
            }
            else {
                // TODO: EnableSchedulingFromForeignThread

                cur_thread.get().scheduler_wait_event.wait();
            }
        }
        else {
            unsafe {
                self.lock.unlock();
            }
        }
    }
}

pub struct KCriticalSectionGuard<'a> {
    lock: &'a mut KCriticalSection
}

impl<'a> KCriticalSectionGuard<'a> {
    pub fn new(lock: &'a mut KCriticalSection) -> Self {
        lock.enter();

        Self {
            lock: lock
        }
    }
}

impl<'a> Drop for KCriticalSectionGuard<'a> {
    fn drop(&mut self) {
        self.lock.leave();
    }
}

static mut G_CRITICAL_SECTION: Option<KCriticalSection> = None;

unsafe fn ensure_critical_section() {
    if G_CRITICAL_SECTION.is_none() {
        G_CRITICAL_SECTION = Some(KCriticalSection::new());
    }
}

pub fn get_critical_section() -> &'static mut KCriticalSection {
    unsafe {
        ensure_critical_section();

        G_CRITICAL_SECTION.as_mut().unwrap()
    }
}

// ---

// KThread

pub const CPU_CORE_COUNT: usize = 4;
pub const INVALID_CPU_CORE: i32 = -1;
pub const PRIORITY_COUNT: usize = 0x40;
pub const IDLE_THREAD_PRIORITY: i32 = 0x40;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u16)]
pub enum ThreadState {
    Initialized = 0,
    Waiting     = 1,
    Runnable    = 2,
    Terminated  = 3,

    ProcessSuspended = 1 << 4,
    ThreadSuspended = 1 << 5,
    DebugSuspended = 1 << 6,
    BacktraceSuspended = 1 << 7,
    InitSuspended = 1 << 8,

    LowMask = 0xF,
    HighMask = 0xFFF0,
    ForcePauseMask = 0x70,
}

impl ThreadState {
    pub fn update_flags(&mut self, other: Self) {
        *self = unsafe {
            core::mem::transmute((((*self as u16) & (ThreadState::HighMask as u16)) | ((other as u16) | (ThreadState::LowMask as u16))))
        };
    }

    pub fn get_low_flags(self) -> Self {
        unsafe {
            core::mem::transmute((self as u16) & (ThreadState::LowMask as u16))
        }
    }
}

static mut G_THREAD_ID_COUNTER: Mutex<u64> = parking_lot::const_mutex(0);

pub fn new_thread_id() -> u64 {
    unsafe {
        let thread_id = *G_THREAD_ID_COUNTER.lock() + 1;
        *G_THREAD_ID_COUNTER.lock() = thread_id;
        return thread_id;
    }
}

static mut G_THREAD_RESELECTION_REQUESTED: bool = false;

pub fn set_thread_reselection_requested(flag: bool) {
    unsafe {
        G_THREAD_RESELECTION_REQUESTED = flag;
    }
}

pub fn thread_reselection_requested() -> bool {
    unsafe {
        G_THREAD_RESELECTION_REQUESTED
    }
}

pub struct KThread {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    has_exited: bool,
    pub is_schedulable: bool,
    base_priority: i32,
    pub scheduler_wait_event: ManualResetEvent,
    pub should_be_terminated: bool,
    pub state: ThreadState,
    pub sync_cancelled: bool,
    pub waiting_sync: bool,
    pub signaled_obj: Option<Shared<dyn KSynchronizationObject + Send + Sync>>,
    pub active_core: i32,
    pub preferred_core: i32,
    pub cur_core: i32,
    pub affinity_mask: i64,
    pub owner_process: Option<Shared<KProcess>>,
    pub cpu_exec_ctx: Option<cpu::ExecutionContext>,
    pub siblings_per_core: Vec<Option<Shared<KThread>>>,
    pub priority: i32,
    pub host_thread_builder: Option<Builder>,
    pub host_thread_handle: Option<JoinHandle<()>>,
    pub ctx: KThreadContext,
    pub id: u64
}

impl KAutoObject for KThread {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }

    fn destroy(&mut self) {
        // remove thread from kprocess
    }
}

impl KSynchronizationObject for KThread {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }

    fn is_signaled(&self) -> bool {
        self.has_exited
    }
}

impl KThread {
    pub fn new(owner_process: Option<Shared<KProcess>>, host_thread_name: String, priority: i32, cpu_core: i32, exec_ctx_args: Option<(u64, usize)>) -> Result<Shared<Self>> {
        let host_builder = Builder::new().name(host_thread_name);

        let cpu_exec_ctx = match owner_process.as_ref() {
            Some(owner_proc) => match exec_ctx_args {
                Some((entry_addr, stack_size)) => {
                    owner_proc.get().increment_refcount();
                    Some(owner_proc.get().cpu_ctx.create_execution_context(stack_size, entry_addr)?)
                },
                None => None,
            },
            None => None
        };

        // Rust has an awful support for arrays, forces us to use Vec for this case :P
        let mut siblings_per_core: Vec<Option<Shared<KThread>>> = Vec::with_capacity(CPU_CORE_COUNT);
        for _ in 0..CPU_CORE_COUNT {
            siblings_per_core.push(None);
        }

        Ok(Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            has_exited: false,
            should_be_terminated: false,
            is_schedulable: true,
            base_priority: priority,
            scheduler_wait_event: ManualResetEvent::new(State::Unset),
            state: ThreadState::Initialized,
            sync_cancelled: false,
            waiting_sync: false,
            signaled_obj: None,
            active_core: cpu_core,
            preferred_core: cpu_core,
            cur_core: cpu_core,
            affinity_mask: bit!(cpu_core as i64),
            owner_process: owner_process,
            cpu_exec_ctx: cpu_exec_ctx,
            siblings_per_core: siblings_per_core,
            priority: priority,
            host_thread_builder: Some(host_builder),
            host_thread_handle: None,
            ctx: KThreadContext::new(),
            id: new_thread_id()
        }))
    }

    pub fn new_host(owner_process: Option<Shared<KProcess>>, host_thread_name: String, priority: i32, cpu_core: i32) -> Result<Shared<Self>> {
        Self::new(owner_process, host_thread_name, priority, cpu_core, None)
    }

    fn adjust_scheduling(thread: &mut Shared<KThread>, old_state_flags: ThreadState) {
        if old_state_flags == thread.get().state {
            return;
        }

        if !thread.get().is_schedulable {
            // TODO: ensure thread is started...?

            if thread.get().state == ThreadState::Runnable {
                thread.get().scheduler_wait_event.set();
            }
            else {
                thread.get().scheduler_wait_event.reset();
            }

            return;
        }

        if old_state_flags == ThreadState::Runnable {
            if thread.get().active_core >= 0 {
                get_priority_queue().unschedule(thread.get().priority, thread.get().active_core, thread.clone());
            }

            for core in 0..CPU_CORE_COUNT as i32 {
                if (core != thread.get().active_core) && (((thread.get().affinity_mask >> core as i64) & 1) != 0) {
                    get_priority_queue().unsuggest(thread.get().priority, thread.get().active_core, thread.clone());
                }
            }
        }
        else if thread.get().state == ThreadState::Runnable {
            if thread.get().active_core >= 0 {
                get_priority_queue().schedule(thread.get().priority, thread.get().active_core, thread.clone());
            }

            for core in 0..CPU_CORE_COUNT as i32 {
                if (core != thread.get().active_core) && (((thread.get().affinity_mask >> core as i64) & 1) != 0) {
                    get_priority_queue().suggest(thread.get().priority, thread.get().active_core, thread.clone());
                }
            }
        }

        // TODO: ThreadReselectionRequested
    }

    pub fn reschedule(thread: &mut Shared<KThread>, new_state_flags: ThreadState) {
        let guard = KCriticalSectionGuard::new(get_critical_section());

        let old_state = thread.get().state;
        thread.get().state.update_flags(new_state_flags);
        Self::adjust_scheduling(thread, old_state);
    }

    fn exec_thread_fn(thread: Shared<KThread>) {
        set_current_thread(thread.clone());

        let mut cpu_exec_ctx_handle = thread.get().cpu_exec_ctx.as_mut().unwrap().get_handle();
        let exec_start_addr = thread.get().cpu_exec_ctx.as_mut().unwrap().exec_start_addr;
        let exec_end_addr = thread.get().cpu_exec_ctx.as_mut().unwrap().exec_end_addr;

        cpu_exec_ctx_handle.start(0u64, 0xBEEFu32, exec_start_addr, exec_end_addr).unwrap();
    }

    fn host_thread_fn<F: FnOnce() + Send + 'static>(thread: Shared<KThread>, f: F) {
        set_current_thread(thread);

        f();
    }

    pub fn start_exec(thread: &mut Shared<KThread>) -> Result<()> {
        result_return_unless!(thread.get().host_thread_builder.is_some(), 0x1);

        let builder = thread.get().host_thread_builder.take();

        let thread_entry_clone = thread.clone();
        thread.get().host_thread_handle = Some(builder.unwrap().spawn(|| {
            Self::exec_thread_fn(thread_entry_clone);
        }).unwrap());

        Ok(())
    }

    pub fn start_host<F: FnOnce() + Send + 'static>(thread: &mut Shared<KThread>, f: F) -> Result<()> {
        result_return_unless!(thread.get().host_thread_builder.is_some(), 0x1);

        let builder = thread.get().host_thread_builder.take();

        let thread_entry_clone = thread.clone();
        thread.get().host_thread_handle = Some(builder.unwrap().spawn(|| {
            Self::host_thread_fn(thread_entry_clone, f);
        }).unwrap());

        Ok(())
    }

    pub fn is_termination_requested(&self) -> bool {
        self.should_be_terminated && (self.state == ThreadState::Terminated)
    }
}

#[thread_local]
static mut G_CURRENT_THREAD: Option<Shared<KThread>> = None;

fn set_current_thread(thread: Shared<KThread>) {
    unsafe {
        G_CURRENT_THREAD = Some(thread);
    }
}

pub fn has_current_thread() -> bool {
    unsafe {
        G_CURRENT_THREAD.is_some()
    }
}

pub fn get_current_thread() -> Shared<KThread> {
    unsafe {
        assert!(has_current_thread());

        G_CURRENT_THREAD.as_ref().unwrap().clone()
    }
}

// ---

// KThreadContext

pub struct KThreadContext {
    lock: AtomicBool
}

impl KThreadContext {
    pub fn new() -> Self {
        Self {
            lock: AtomicBool::new(false)
        }
    }

    pub fn lock(&self) -> bool {
        let old_val = self.lock.load(Ordering::SeqCst);
        self.lock.store(true, Ordering::SeqCst);
        old_val == false
    }

    pub fn unlock(&self) {
        self.lock.store(false, Ordering::SeqCst);
    }
}

// ---

// KPriorityQueue

pub struct KPriorityQueue {
    pub scheduled_threads_per_prio_per_core: Vec<Vec<Vec<Shared<KThread>>>>,
    pub scheduled_priority_masks_per_core: [u64; CPU_CORE_COUNT],
    pub suggested_threads_per_prio_per_core: Vec<Vec<Vec<Shared<KThread>>>>,
    pub suggested_priority_masks_per_core: [u64; CPU_CORE_COUNT],
}

impl KPriorityQueue {
    fn ensure_queues_ready(&mut self) {
        if self.scheduled_threads_per_prio_per_core.is_empty() {
            for core in 0..CPU_CORE_COUNT {
                let mut scheduled_threads_per_prio: Vec<Vec<Shared<KThread>>> = Vec::new();
                let mut suggested_threads_per_prio: Vec<Vec<Shared<KThread>>> = Vec::new();
                for prio in 0..PRIORITY_COUNT {
                    scheduled_threads_per_prio.push(Vec::new());
                    suggested_threads_per_prio.push(Vec::new());
                }
                self.scheduled_threads_per_prio_per_core.push(scheduled_threads_per_prio);
                self.suggested_threads_per_prio_per_core.push(suggested_threads_per_prio);
            }
        }
    }

    pub const fn new() -> Self {
        Self {
            scheduled_threads_per_prio_per_core: Vec::new(),
            scheduled_priority_masks_per_core: [0; CPU_CORE_COUNT],
            suggested_threads_per_prio_per_core: Vec::new(),
            suggested_priority_masks_per_core: [0; CPU_CORE_COUNT]
        }
    }

    pub fn suggest(&mut self, prio: i32, cpu_core: i32, mut thread: Shared<KThread>) {
        self.ensure_queues_ready();

        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = Some(thread.clone());

            let queue = &mut self.suggested_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.insert(0, thread.clone());
            self.suggested_priority_masks_per_core[cpu_core as usize] |= bit!(prio);
        }
    }

    pub fn unsuggest(&mut self, prio: i32, cpu_core: i32, mut thread: Shared<KThread>) {
        self.ensure_queues_ready();
        
        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = None;

            let queue = &mut self.suggested_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.retain(|s_thread| !s_thread.ptr_eq(&thread));
            
            if queue.is_empty() {
                self.suggested_priority_masks_per_core[cpu_core as usize] &= !bit!(prio);
            }
        }
    }

    pub fn schedule(&mut self, prio: i32, cpu_core: i32, mut thread: Shared<KThread>) {
        self.ensure_queues_ready();

        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = Some(thread.clone());

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.push(thread.clone());
            self.scheduled_priority_masks_per_core[cpu_core as usize] |= bit!(prio);
        }
    }

    pub fn schedule_prepend(&mut self, prio: i32, cpu_core: i32, mut thread: Shared<KThread>) {
        self.ensure_queues_ready();

        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = Some(thread.clone());

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.insert(0, thread.clone());
            self.scheduled_priority_masks_per_core[cpu_core as usize] |= bit!(prio);
        }
    }

    pub fn reschedule(&mut self, prio: i32, cpu_core: i32, mut thread: Shared<KThread>) -> Option<Shared<KThread>> {
        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = None;

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.retain(|s_thread| !s_thread.ptr_eq(&thread));
            queue.push(thread.clone());

            return Some(queue.first().unwrap().clone());
        }

        None
    }

    pub fn unschedule(&mut self, prio: i32, cpu_core: i32, mut thread: Shared<KThread>) {
        self.ensure_queues_ready();
        
        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = None;

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.retain(|s_thread| !s_thread.ptr_eq(&thread));
            
            if queue.is_empty() {
                self.scheduled_priority_masks_per_core[cpu_core as usize] &= !bit!(prio);
            }
        }
    }

    pub fn trailing_zero_count(val: u64) -> u64 {
        for i in 0..64 {
            if (val & bit!(i)) != 0 {
                return i;
            }
        }

        return 64;
    }

    fn get_thread_list(&self, core: i32, suggested: bool) -> Vec<Shared<KThread>> {
        let (thread_list, mut cur_priority_mask) = match suggested {
            true => (&self.suggested_threads_per_prio_per_core, self.suggested_priority_masks_per_core[core as usize]),
            false => (&self.scheduled_threads_per_prio_per_core, self.scheduled_priority_masks_per_core[core as usize])
        };

        let mut ret_thread_list: Vec<Shared<KThread>> = Vec::new();
        loop {
            let priority = Self::trailing_zero_count(cur_priority_mask) as i32;
            if priority == PRIORITY_COUNT as i32 {
                break;
            }

            let cur_thread_list = &thread_list[core as usize][priority as usize];
            for thread in cur_thread_list {
                ret_thread_list.push(thread.clone());
            }

            cur_priority_mask &= bit!(priority as u64);
        }

        return ret_thread_list;
    }

    pub fn get_scheduled_threads_for_core(&self, core: i32) -> Vec<Shared<KThread>> {
        self.get_thread_list(core, false)
    }

    pub fn get_suggested_threads_for_core(&self, core: i32) -> Vec<Shared<KThread>> {
        self.get_thread_list(core, true)
    }

    pub fn transfer_thread_to_core(&mut self, priority: i32, dst_core: i32, thread: &Shared<KThread>) {
        let src_core = thread.get().active_core;
        if src_core != dst_core {
            thread.get().active_core = dst_core;

            if src_core >= 0 {
                self.unschedule(priority, src_core, thread.clone());
            }

            if dst_core >= 0 {
                self.unsuggest(priority, dst_core, thread.clone());
                self.schedule(priority, dst_core, thread.clone());
            }

            if src_core >= 0 {
                self.suggest(priority, src_core, thread.clone());
            }
        }
    }
}

static mut G_PRIORITY_QUEUE: KPriorityQueue = KPriorityQueue::new();

pub fn get_priority_queue() -> &'static mut KPriorityQueue {
    unsafe {
        &mut G_PRIORITY_QUEUE
    }
}

// ---

// KScheduler

static mut G_SCHEDULERS: Vec<KScheduler> = Vec::new();

pub fn get_scheduler(cpu_core: i32) -> &'static mut KScheduler {
    unsafe {
        &mut G_SCHEDULERS[cpu_core as usize]
    }
}

pub fn initialize_schedulers() -> Result<()> {
    unsafe {
        if G_SCHEDULERS.is_empty() {
            for core in 0..CPU_CORE_COUNT as i32 {
                G_SCHEDULERS.push(KScheduler::new(core)?);
            }
            for scheduler in &mut G_SCHEDULERS {
                scheduler.start()?;
            }
        }
    }

    Ok(())
}

pub struct KScheduler {
    cpu_core: i32,
    needs_scheduling: Mutex<bool>,
    selected_thread: Mutex<Option<Shared<KThread>>>,
    idle_interrupt_event: AutoResetEvent,
    cur_thread: Shared<KThread>,
    idle_thread: Shared<KThread>,
    pub prev_thread: Option<Shared<KThread>>,
    pub last_context_switch_instant: time::Instant
}

impl KScheduler {
    pub fn new(cpu_core: i32) -> Result<Self> {
        let mut idle_thread = KThread::new_host(None, format!("pg.kern.thread.SchedulerIdleThreadForCore{}", cpu_core), IDLE_THREAD_PRIORITY, cpu_core)?;  

        Ok(Self {
            cpu_core: cpu_core,
            needs_scheduling: Mutex::new(false),
            selected_thread: Mutex::new(None),
            idle_interrupt_event: AutoResetEvent::new(State::Unset),
            cur_thread: idle_thread.clone(),
            idle_thread: idle_thread,
            prev_thread: None,
            last_context_switch_instant: time::Instant::now()
        })
    }

    fn idle_thread_fn(cpu_core: i32) {
        println!("Current thread name: {}", get_current_thread().get().host_thread_handle.as_ref().unwrap().thread().name().unwrap());
    
        let mut scheduler = get_scheduler(cpu_core);
        loop {
            *scheduler.needs_scheduling.lock() = false;
            // memory barrier?
            let selected_thread = match scheduler.selected_thread.lock().as_ref() {
                Some(thread) => Some(thread.clone()),
                None => None
            };
            let next_thread = scheduler.pick_next_thread(selected_thread);

            if !next_thread.ptr_eq(&scheduler.idle_thread) {
                scheduler.idle_thread.get().scheduler_wait_event.reset();

                next_thread.get().scheduler_wait_event.set();
                scheduler.idle_thread.get().scheduler_wait_event.wait();
            }

            scheduler.idle_interrupt_event.wait();
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let cpu_core_copy = self.cpu_core;
        KThread::start_host(&mut self.idle_thread, move || {
            Self::idle_thread_fn(cpu_core_copy);
        })
    }

    fn pick_next_thread(&mut self, mut selected_thread: Option<Shared<KThread>>) -> Shared<KThread> {
        loop {
            if let Some(thread) = selected_thread {
                println!("PA");
                if thread.get().ctx.lock() {
                    self.switch_to(Some(thread.clone()));
                    if !*self.needs_scheduling.lock() {
                        return self.selected_thread.lock().as_ref().unwrap().clone();
                    }

                    println!("PB");
                    thread.get().ctx.unlock();
                    println!("PC");
                }
                else {
                    return self.idle_thread.clone();
                }
            }
            else {
                self.switch_to(None);
                return self.idle_thread.clone();
            }

            println!("PA1");
            *self.needs_scheduling.lock() = false;
            selected_thread = Some(self.selected_thread.lock().as_ref().unwrap().clone());
            println!("PA2");
        }
    }

    fn switch_to(&mut self, next_thread: Option<Shared<KThread>>) {
        let thread = next_thread.unwrap_or(self.idle_thread.clone());
        let cur_thread = get_current_thread();

        if !cur_thread.ptr_eq(&thread) {
            let cur_instant = time::Instant::now();
            let ticks_delta = cur_instant.duration_since(self.last_context_switch_instant);

            /* cur thread add cpu time */

            if has_current_process() {
                /* cur process add cpu time */
            }

            self.last_context_switch_instant = cur_instant;

            if has_current_process() {
                println!("AS");
                let is_thread_running = !cur_thread.get().is_termination_requested();
                let is_in_same_core = cur_thread.get().active_core == self.cpu_core;
                self.prev_thread = match is_thread_running && is_in_same_core {
                    true => Some(cur_thread.clone()),
                    false => None
                };
                println!("ASS");
            }
            else if cur_thread.ptr_eq(&self.idle_thread) {
                self.prev_thread = None;
            }
        }

        if thread.get().cur_core != self.cpu_core {
            thread.get().cur_core = self.cpu_core;
        }

        self.cur_thread = thread;
    }

    pub fn schedule(&mut self) {
        *self.needs_scheduling.lock() = false;

        let cur_thread = get_current_thread();

        let selected_thread = match self.selected_thread.lock().as_ref() {
            Some(thread) => Some(thread.clone()),
            None => None
        };

        if let Some(sel_thread) = selected_thread.as_ref() {
            if cur_thread.ptr_eq(sel_thread) {
                return;
            }
        }

        cur_thread.get().scheduler_wait_event.reset();
        cur_thread.get().ctx.unlock();

        for core in 0..CPU_CORE_COUNT as i32 {
            println!("Qt {}", core);
            get_scheduler(core).idle_interrupt_event.set();
            println!("Qtt {}", core);
        }

        let next_thread = self.pick_next_thread(selected_thread);
        next_thread.get().scheduler_wait_event.set();

        if /* current thread exec ctx running? */ true {
            next_thread.get().scheduler_wait_event.wait();
        }
        else {
            cur_thread.get().is_schedulable = false;
            cur_thread.get().cur_core = INVALID_CPU_CORE;
        }
    }

    fn select_thread(&mut self, next_thread: Option<Shared<KThread>>) -> u64 {
        let mut prev_selected_thread = self.selected_thread.lock();

        let threads_match = match (next_thread.is_none() && prev_selected_thread.is_none()) {
            true => true,
            false => next_thread.as_ref().unwrap().ptr_eq(prev_selected_thread.as_ref().unwrap())
        };

        if !threads_match {
            if let Some(prev_selected_thread_v) = prev_selected_thread.as_ref() {
                // TODO: set last scheduled time
            }

            *prev_selected_thread = next_thread;
            *self.needs_scheduling.lock() = true;
            bit!(self.cpu_core)
        }
        else {
            0
        }
    }

    pub fn select_threads() -> u64 {
        if !thread_reselection_requested() {
            return 0;
        }

        set_thread_reselection_requested(false);
        
        let mut scheduled_cores_mask = 0u64;
        for core in 0..CPU_CORE_COUNT as i32 {
            let thread = match get_priority_queue().get_scheduled_threads_for_core(core).first() {
                Some(thread_ref) => Some(thread_ref.clone()),
                None => None
            };
            scheduled_cores_mask |= get_scheduler(core).select_thread(thread);
        }

        for core in 0..CPU_CORE_COUNT as i32 {
            if get_priority_queue().get_scheduled_threads_for_core(core).is_empty() {
                let mut dst_thread: Option<Shared<KThread>> = None;

                let mut src_cores_highest_priority_threads: Vec<i32> = Vec::with_capacity(CPU_CORE_COUNT);

                for suggested_thread in &get_priority_queue().get_suggested_threads_for_core(core) {
                    let active_core = suggested_thread.get().active_core;
                    let is_scheduler_selected_thread = match &*get_scheduler(active_core).selected_thread.lock() {
                        Some(selected_thread) => suggested_thread.ptr_eq(selected_thread),
                        None => false
                    };
                    if (active_core < 0) || is_scheduler_selected_thread {
                        dst_thread = Some(suggested_thread.clone());
                        break;
                    }

                    src_cores_highest_priority_threads.push(active_core);
                }

                if let Some(dst_thread_v) = dst_thread {
                    let dst_priority = dst_thread_v.get().priority;
                    if dst_priority >= 2 {
                        get_priority_queue().transfer_thread_to_core(dst_priority, core, &dst_thread_v);
                        scheduled_cores_mask |= get_scheduler(core).select_thread(Some(dst_thread_v.clone()));
                    }
                    continue;
                }

                for src_core in src_cores_highest_priority_threads {
                    if let Some(src_thread) = get_priority_queue().get_scheduled_threads_for_core(src_core).get(1) {
                        let orig_selected_thread = get_scheduler(src_core).selected_thread.lock();
                        
                        scheduled_cores_mask |= get_scheduler(src_core).select_thread(Some(src_thread.clone()));

                        let priority = orig_selected_thread.as_ref().unwrap().get().priority;
                        get_priority_queue().transfer_thread_to_core(priority, core, orig_selected_thread.as_ref().unwrap());
                    }
                }
            }
        }

        scheduled_cores_mask
    }
}

// ---