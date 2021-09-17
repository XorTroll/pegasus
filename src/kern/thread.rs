use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::thread::Builder;
use std::thread::JoinHandle;
use std::time::{self, Duration};
use parking_lot::Mutex;
use rsevents::AutoResetEvent;
use rsevents::Awaitable;
use rsevents::ManualResetEvent;
use rsevents::State;
use crate::emu::cpu;
use crate::util::{Shared, RecursiveLock, new_recursive_lock};
use crate::result::*;
use crate::os::ThreadLocalRegion;
use super::{KAutoObject, KFutureSchedulerObject, get_time_manager};
use super::KSynchronizationObject;
use super::proc::KProcess;
use super::proc::has_current_process;
use super::result;

// KCriticalSection
// Note: thanks Rust for only supporting mutex functionality through guards/wrapping objects, luckily parking_lot exposes raw mutex typea

pub struct KCriticalSection {
    lock: RecursiveLock,
    recursion_count: i32
}

impl KCriticalSection {
    pub const fn new() -> Self {
        Self {
            lock: new_recursive_lock(),
            recursion_count: 0
        }
    }

    pub fn enter(&mut self) {
        // log_line!("KCriticalSection enter");
        self.lock.lock();
        self.recursion_count += 1;
    }

    pub fn leave(&mut self) {
        // log_line!("KCriticalSection leave");
        if self.recursion_count == 0 {
            return;
        }

        self.recursion_count -= 1;
        if self.recursion_count == 0 {
            let scheduled_cores_mask = KScheduler::select_threads();

            unsafe {
                self.lock.unlock();
            }

            let cur_thread = try_get_current_thread();

            let is_cur_thread_schedulable = match cur_thread.as_ref() {
                Some(thread) => thread.get().is_schedulable,
                None => false
            };
            if is_cur_thread_schedulable {
                KScheduler::enable_scheduling(scheduled_cores_mask);
            }
            else {
                KScheduler::enable_scheduling_from_foreign_thread(scheduled_cores_mask);

                if let Some(thread) = cur_thread.as_ref() {
                    /* If exec ctx running: */
                    get_scheduler_wait_event(thread).wait();
                }
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

static mut G_CRITICAL_SECTION: KCriticalSection = KCriticalSection::new();

#[inline]
pub fn get_critical_section() -> &'static mut KCriticalSection {
    unsafe {
        &mut G_CRITICAL_SECTION
    }
}

#[inline]
pub fn make_critical_section_guard<'a>() -> KCriticalSectionGuard<'a> {
    KCriticalSectionGuard::new(get_critical_section())
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
    Waiting = 1,
    Runnable = 2,
    Terminated = 3,

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
            core::mem::transmute(((*self as u16) & (ThreadState::HighMask as u16)) | ((other as u16) & (ThreadState::LowMask as u16)))
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
        let mut thread_id_counter = G_THREAD_ID_COUNTER.lock();
        *thread_id_counter += 1;
        return *thread_id_counter;
    }
}

static mut G_THREAD_RESELECTION_REQUESTED: bool = false;

#[inline]
pub fn set_thread_reselection_requested(flag: bool) {
    unsafe {
        G_THREAD_RESELECTION_REQUESTED = flag;
    }
}

#[inline]
pub fn thread_reselection_requested() -> bool {
    unsafe {
        G_THREAD_RESELECTION_REQUESTED
    }
}

static mut G_THREAD_SCHEDULER_WAIT_EVENTS: Vec<(Shared<KThread>, ManualResetEvent)> = Vec::new();

fn register_scheduler_wait_event(thread: &Shared<KThread>) {
    unsafe {
        G_THREAD_SCHEDULER_WAIT_EVENTS.push((thread.clone(), ManualResetEvent::new(State::Unset)));
    }
}

pub fn get_scheduler_wait_event(thread: &Shared<KThread>) -> &'static mut ManualResetEvent {
    unsafe {
        for i in 0..G_THREAD_SCHEDULER_WAIT_EVENTS.len() {
            let (s_thread, s_event) = &mut G_THREAD_SCHEDULER_WAIT_EVENTS[i];
            if s_thread.ptr_eq(thread) {
                return s_event;
            }
        }
    }

    panic!("Scheduler wait event not found!");
}

pub struct KThread {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    has_exited: bool,
    pub is_schedulable: bool,
    force_pause_state: ThreadState,
    pub sync_result: ResultCode,
    base_priority: i32,
    pub should_be_terminated: bool,
    pub state: ThreadState,
    pub sync_cancelled: bool,
    pub waiting_sync: bool,
    pub signaled_obj: Option<Shared<dyn KSynchronizationObject>>,
    pub active_core: i32,
    pub preferred_core: i32,
    pub cur_core: i32,
    pub affinity_mask: i64,
    pub owner_process: Option<Shared<KProcess>>,
    pub cpu_exec_ctx: Option<cpu::ExecutionContext>,
    pub emu_tlr: [u8; 0x100],
    pub siblings_per_core: Vec<Option<Shared<KThread>>>,
    pub withholder: Option<Vec<Shared<KThread>>>,
    pub withholder_entry: Option<Shared<KThread>>,
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

impl KFutureSchedulerObject for KThread {
    fn time_up(&mut self) {
        todo!("time_up");
    }
}

impl KThread {
    pub fn new(owner_process: Option<Shared<KProcess>>, host_thread_name: String, priority: i32, cpu_core: i32, exec_ctx_args: Option<(u64, usize)>) -> Result<Shared<Self>> {
        let host_builder = Builder::new().name(host_thread_name);

        let cpu_exec_ctx = match owner_process.as_ref() {
            Some(owner_proc) => match exec_ctx_args {
                Some((entry_addr, stack_size)) => match owner_proc.get().cpu_ctx.as_ref() {
                    Some(cpu_ctx) => {
                        // owner_proc.get().increment_refcount();
                        Some(cpu_ctx.create_execution_context(stack_size, entry_addr)?)
                    },
                    None => None
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

        // TODO: force pause flags if owner paused...

        let thread = Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            has_exited: false,
            should_be_terminated: false,
            is_schedulable: true,
            force_pause_state: ThreadState::Initialized,
            sync_result: result::ResultNoThread::make(),
            base_priority: priority,
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
            emu_tlr: [0; 0x100],
            siblings_per_core: siblings_per_core,
            withholder: None,
            withholder_entry: None,
            priority: priority,
            host_thread_builder: Some(host_builder),
            host_thread_handle: None,
            ctx: KThreadContext::new(),
            id: new_thread_id()
        });

        register_scheduler_wait_event(&thread);
        Ok(thread)
    }

    pub fn new_host(owner_process: Option<Shared<KProcess>>, host_thread_name: String, priority: i32, cpu_core: i32) -> Result<Shared<Self>> {
        Self::new(owner_process, host_thread_name, priority, cpu_core, None)
    }

    fn set_new_state(thread: &mut Shared<KThread>, new_flags: ThreadState) {
        let _guard = make_critical_section_guard();

        let old_flags = thread.get().state;
        thread.get().state.update_flags(new_flags);

        if old_flags.get_low_flags() != new_flags {
            Self::adjust_scheduling(thread, old_flags);
        }
    }

    fn adjust_scheduling(thread: &mut Shared<KThread>, old_state_flags: ThreadState) {
        let cur_state = thread.get().state;
        if old_state_flags == cur_state {
            return;
        }

        let is_not_schedulable = !thread.get().is_schedulable;
        if is_not_schedulable {
            // TODO: ensure thread is started...?

            if cur_state == ThreadState::Runnable {
                get_scheduler_wait_event(thread).set();
            }
            else {
                get_scheduler_wait_event(thread).reset();
            }

            return;
        }

        let active_core = thread.get().active_core;
        let priority = thread.get().priority;
        let affinity_mask = thread.get().affinity_mask;

        if old_state_flags == ThreadState::Runnable {
            if active_core >= 0 {
                get_priority_queue().unschedule(priority, active_core, thread.clone());
            }

            for core in 0..CPU_CORE_COUNT as i32 {
                if (core != active_core) && (((affinity_mask >> core as i64) & 1) != 0) {
                    get_priority_queue().unsuggest(priority, active_core, thread.clone());
                }
            }
        }
        else if cur_state == ThreadState::Runnable {
            if active_core >= 0 {
                get_priority_queue().schedule(priority, active_core, thread.clone());
            }

            for core in 0..CPU_CORE_COUNT as i32 {
                if (core != active_core) && (((affinity_mask >> core as i64) & 1) != 0) {
                    get_priority_queue().suggest(priority, active_core, thread.clone());
                }
            }
        }

        set_thread_reselection_requested(true);
    }

    pub fn reschedule(thread: &mut Shared<KThread>, new_state_flags: ThreadState) {
        let _guard = make_critical_section_guard();

        let old_state = thread.get().state;
        thread.get().state.update_flags(new_state_flags);
        Self::adjust_scheduling(thread, old_state);
    }

    fn exec_thread_fn<T: Copy + Send + Sync + 'static, U: Copy + Send + Sync + 'static>(thread: Shared<KThread>, arg_x0: T, arg_x1: U) {
        set_current_thread(thread.clone());

        let mut cpu_exec_ctx_handle = thread.get().cpu_exec_ctx.as_mut().unwrap().get_handle();
        let exec_start_addr = thread.get().cpu_exec_ctx.as_mut().unwrap().exec_start_addr;
        let exec_end_addr = thread.get().cpu_exec_ctx.as_mut().unwrap().exec_end_addr;

        cpu_exec_ctx_handle.start(arg_x0, arg_x1, exec_start_addr, exec_end_addr).unwrap();

        reset_current_thread();
    }

    fn host_thread_fn<F: FnOnce() + Send + 'static>(thread: Shared<KThread>, f: F) {
        set_current_thread(thread.clone());

        f();

        reset_current_thread();
    }

    fn do_start<F: FnOnce() + Send + 'static>(thread: &mut Shared<KThread>, f: F) -> Result<()> {
        /* if kern not initialized, <...> */

        let _guard = make_critical_section_guard();

        let should_be_terminated = thread.get().should_be_terminated;
        if !should_be_terminated {
            let cur_thread = try_get_current_thread();
            
            loop {
                let cur_state = thread.get().state;

                if cur_state == ThreadState::Terminated {
                    break;
                }

                if let Some(cur_thread) = cur_thread.as_ref() {
                    if cur_thread.get().is_termination_requested() {
                        break;
                    }
                }

                result_return_unless!(cur_state.get_low_flags() == ThreadState::Initialized, result::ResultInvalidState);
                
                if cur_thread.is_none() || (cur_thread.as_ref().unwrap().get().force_pause_state == ThreadState::Initialized) {
                    let force_pause_state = thread.get().force_pause_state;
                    if thread.get().owner_process.is_some() && (force_pause_state != ThreadState::Initialized) {
                        todo!("CombineForcePauseFlags");
                    }

                    Self::set_new_state(thread, ThreadState::Runnable);

                    let builder = thread.get().host_thread_builder.take();
                    thread.get().host_thread_handle = Some(builder.unwrap().spawn(f).unwrap());

                    return Ok(());
                }
            }
        }

        result::ResultTerminationRequested::make_err()
    }

    pub fn start_exec<T: Copy + Send + Sync + 'static, U: Copy + Send + Sync + 'static>(thread: &mut Shared<KThread>, arg_x0: T, arg_x1: U) -> Result<()> {
        result_return_unless!(thread.get().host_thread_builder.is_some(), 0x1);

        let thread_entry_clone = thread.clone();
        Self::do_start(thread, move || {
            Self::exec_thread_fn(thread_entry_clone, arg_x0, arg_x1);
        })
    }

    pub fn start_host<F: FnOnce() + Send + 'static>(thread: &mut Shared<KThread>, f: F) -> Result<()> {
        result_return_unless!(thread.get().host_thread_builder.is_some(), 0x1);

        let thread_entry_clone = thread.clone();
        Self::do_start(thread, move || {
            Self::host_thread_fn(thread_entry_clone, f);
        })
    }

    pub fn is_termination_requested(&self) -> bool {
        self.should_be_terminated || (self.state == ThreadState::Terminated)
    }

    pub fn is_emu_thread(&self) -> bool {
        self.cpu_exec_ctx.is_none()
    }

    pub fn get_tlr_ptr(&mut self) -> *mut u8 {
        if let Some(exec_ctx) = self.cpu_exec_ctx.as_mut() {
            exec_ctx.tlr.data.as_mut_ptr()
        }
        else {
            self.emu_tlr.as_mut_ptr()
        }
    }

    pub fn get_thread_local_region(&mut self) -> &'static mut ThreadLocalRegion {
        unsafe {
            &mut *(self.get_tlr_ptr() as *mut ThreadLocalRegion)
        }
    }

    pub fn get_host_name(&self) -> &str {
        self.host_thread_handle.as_ref().unwrap().thread().name().unwrap()
    }
}

#[thread_local]
static mut G_CURRENT_THREAD: Option<Shared<KThread>> = None;

#[inline]
fn set_current_thread(thread: Shared<KThread>) {
    unsafe {
        G_CURRENT_THREAD = Some(thread);
    }
}

#[inline]
fn reset_current_thread() {
    unsafe {
        G_CURRENT_THREAD = None;
    }
}

#[inline]
pub fn has_current_thread() -> bool {
    unsafe {
        G_CURRENT_THREAD.is_some()
    }
}

#[inline]
pub fn try_get_current_thread() -> Option<Shared<KThread>> {
    unsafe {
        G_CURRENT_THREAD.clone()
    }
}

#[inline]
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
            for _ in 0..CPU_CORE_COUNT {
                let mut scheduled_threads_per_prio: Vec<Vec<Shared<KThread>>> = Vec::new();
                let mut suggested_threads_per_prio: Vec<Vec<Shared<KThread>>> = Vec::new();
                for _ in 0..PRIORITY_COUNT {
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

    pub fn suggest(&mut self, prio: i32, cpu_core: i32, thread: Shared<KThread>) {
        self.ensure_queues_ready();

        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = Some(thread.clone());

            let queue = &mut self.suggested_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.insert(0, thread.clone());
            self.suggested_priority_masks_per_core[cpu_core as usize] |= bit!(prio);
        }
    }

    pub fn unsuggest(&mut self, prio: i32, cpu_core: i32, thread: Shared<KThread>) {
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

    pub fn schedule(&mut self, prio: i32, cpu_core: i32, thread: Shared<KThread>) {
        self.ensure_queues_ready();

        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = Some(thread.clone());

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.push(thread.clone());
            self.scheduled_priority_masks_per_core[cpu_core as usize] |= bit!(prio);
        }
    }

    pub fn schedule_prepend(&mut self, prio: i32, cpu_core: i32, thread: Shared<KThread>) {
        self.ensure_queues_ready();

        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = Some(thread.clone());

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.insert(0, thread.clone());
            self.scheduled_priority_masks_per_core[cpu_core as usize] |= bit!(prio);
        }
    }

    pub fn reschedule(&mut self, prio: i32, cpu_core: i32, thread: Shared<KThread>) -> Option<Shared<KThread>> {
        if prio < PRIORITY_COUNT as i32 {
            thread.get().siblings_per_core[cpu_core as usize] = None;

            let queue = &mut self.scheduled_threads_per_prio_per_core[cpu_core as usize][prio as usize];
            queue.retain(|s_thread| !s_thread.ptr_eq(&thread));
            queue.push(thread.clone());

            return Some(queue.first().unwrap().clone());
        }

        None
    }

    pub fn unschedule(&mut self, prio: i32, cpu_core: i32, thread: Shared<KThread>) {
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

    fn get_thread_list(&self, core: i32, suggested: bool) -> Vec<Shared<KThread>> {
        let (thread_list, mut cur_priority_mask) = match suggested {
            true => (&self.suggested_threads_per_prio_per_core, self.suggested_priority_masks_per_core[core as usize]),
            false => (&self.scheduled_threads_per_prio_per_core, self.scheduled_priority_masks_per_core[core as usize])
        };

        let mut ret_thread_list: Vec<Shared<KThread>> = Vec::new();
        loop {
            let priority = cur_priority_mask.trailing_zeros() as i32;
            if priority == PRIORITY_COUNT as i32 {
                break;
            }

            let cur_thread_list = &thread_list[core as usize][priority as usize];
            for thread in cur_thread_list {
                ret_thread_list.push(thread.clone());
            }

            cur_priority_mask &= !bit!(priority as u64);
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

#[inline]
pub fn get_priority_queue() -> &'static mut KPriorityQueue {
    unsafe {
        &mut G_PRIORITY_QUEUE
    }
}

// ---

// KScheduler

static mut G_SCHEDULERS: Vec<KScheduler> = Vec::new();

#[inline]
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
        let idle_thread = KThread::new_host(None, format!("pg.kern.thread.KSchedulerIdleThreadForCore{}", cpu_core), IDLE_THREAD_PRIORITY, cpu_core)?;  

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
        log_line!("Hello World!");
    
        let scheduler = get_scheduler(cpu_core);
        loop {
            *scheduler.needs_scheduling.lock() = false;
            // TODO: memory barrier? (Ryujinx does so, might not be necessary at all here...)
            let selected_thread = scheduler.selected_thread.lock().clone();
            let next_thread = scheduler.pick_next_thread(selected_thread);

            if !next_thread.ptr_eq(&scheduler.idle_thread) {
                get_scheduler_wait_event(&next_thread).set();

                get_scheduler_wait_event(&scheduler.idle_thread).reset();
                get_scheduler_wait_event(&scheduler.idle_thread).wait();
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

    fn pick_next_thread(&mut self, selected_thread: Option<Shared<KThread>>) -> Shared<KThread> {
        let mut sel_thread = selected_thread.clone();
        loop {
            if let Some(sel_thread_v) = sel_thread.as_ref() {
                let thread_ctx_lock = sel_thread_v.get().ctx.lock();
                if thread_ctx_lock {
                    self.switch_to(Some(sel_thread_v.clone()));
                    if !*self.needs_scheduling.lock() {
                        return sel_thread_v.clone();
                    }

                    sel_thread_v.get().ctx.unlock();
                }
                else {
                    return self.idle_thread.clone();
                }
            }
            else {
                self.switch_to(None);
                return self.idle_thread.clone();
            }

            *self.needs_scheduling.lock() = false;
            // memory barrier?
            sel_thread = Some(self.selected_thread.lock().as_ref().unwrap().clone());
        }
    }

    fn switch_to(&mut self, next_thread: Option<Shared<KThread>>) {
        let thread = next_thread.unwrap_or(self.idle_thread.clone());
        let cur_thread = get_current_thread();

        if !cur_thread.ptr_eq(&thread) {
            let cur_instant = time::Instant::now();
            let _ticks_delta = cur_instant.duration_since(self.last_context_switch_instant);

            // TODO: cur thread add cpu time

            if has_current_process() {
                // TODO: cur process add cpu time
            }

            self.last_context_switch_instant = cur_instant;

            if has_current_process() {
                let is_thread_running = !cur_thread.get().is_termination_requested();
                let is_in_same_core = cur_thread.get().active_core == self.cpu_core;
                self.prev_thread = match is_thread_running && is_in_same_core {
                    true => Some(cur_thread.clone()),
                    false => None
                };
            }
            else if cur_thread.ptr_eq(&self.idle_thread) {
                self.prev_thread = None;
            }
        }

        let cur_core = thread.get().cur_core;
        if cur_core != self.cpu_core {
            thread.get().cur_core = self.cpu_core;
        }

        self.cur_thread = thread;
    }

    pub fn schedule(&mut self) {
        *self.needs_scheduling.lock() = false;

        let cur_thread = get_current_thread();
        let selected_thread = self.selected_thread.lock().clone();

        if let Some(sel_thread) = selected_thread.as_ref() {
            if cur_thread.ptr_eq(sel_thread) {
                return;
            }
        }

        get_scheduler_wait_event(&cur_thread).reset();
        cur_thread.get().ctx.unlock();

        for core in 0..CPU_CORE_COUNT as i32 {
            get_scheduler(core).idle_interrupt_event.set();
        }

        let next_thread = self.pick_next_thread(selected_thread);
        get_scheduler_wait_event(&next_thread).set();

        if /* current thread exec ctx running? */ true {
            get_scheduler_wait_event(&cur_thread).wait();
        }
        else {
            cur_thread.get().is_schedulable = false;
            cur_thread.get().cur_core = INVALID_CPU_CORE;
        }
    }

    fn select_thread(&mut self, next_thread: Option<Shared<KThread>>) -> u64 {
        let mut prev_selected_thread = self.selected_thread.lock();

        let threads_match = match next_thread.is_some() && prev_selected_thread.is_some() {
            true => next_thread.as_ref().unwrap().ptr_eq(prev_selected_thread.as_ref().unwrap()),
            false => next_thread.is_none() && prev_selected_thread.is_none()
        };

        if !threads_match {
            if let Some(_prev_selected_thread_v) = prev_selected_thread.as_ref() {
                // todo!("select_thread set last scheduled time");
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
            let thread = get_priority_queue().get_scheduled_threads_for_core(core).first().map(|thread| thread.clone());
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
                    if (active_core < 0) || !is_scheduler_selected_thread {
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
                        scheduled_cores_mask |= get_scheduler(core).select_thread(Some(orig_selected_thread.as_ref().unwrap().clone()));
                    }
                }
            }
        }

        scheduled_cores_mask
    }

    pub fn enable_scheduling(scheduled_cores_mask: u64) {
        let cur_core = get_current_thread().get().cur_core;
        let cur_scheduler = get_scheduler(cur_core);

        cur_scheduler.reschedule_other_cores_self(scheduled_cores_mask);
        cur_scheduler.reschedule_current_core();
    }

    pub fn enable_scheduling_from_foreign_thread(scheduled_cores_mask: u64) {
        Self::reschedule_other_cores(scheduled_cores_mask);
    }

    fn reschedule_other_cores_self(&self, scheduled_cores_mask: u64) {
        Self::reschedule_other_cores(scheduled_cores_mask & !bit!(self.cpu_core));
    }
    
    fn reschedule_other_cores(scheduled_cores_mask: u64) {
        let mut mask = scheduled_cores_mask;
        while mask != 0 {
            let core_to_signal = mask.trailing_zeros() as i32;
            let scheduler = get_scheduler(core_to_signal);

            if !scheduler.cur_thread.ptr_eq(&scheduler.idle_thread) {
                todo!("Request to reschedule");
            }

            scheduler.idle_interrupt_event.set();
            mask &= !bit!(core_to_signal);
        }
    }

    fn reschedule_current_core(&mut self) {
        if *self.needs_scheduling.lock() {
            self.schedule();
        }
    }
}

// ---

// KConditionVariable

pub struct KConditionVariable {
}

impl KConditionVariable {
    pub fn wait(thread_list: &mut Vec<Shared<KThread>>, timeout: Duration) {
        get_critical_section().enter();

        let mut cur_thread = get_current_thread();
        cur_thread.get().withholder = Some(thread_list.clone());
        KThread::reschedule(&mut cur_thread, ThreadState::Waiting);
        let withholder_idx = thread_list.len();
        cur_thread.get().withholder_entry = Some(cur_thread.clone());
        let cur_thread_clone = cur_thread.clone();
        cur_thread.get().withholder.as_mut().unwrap().push(cur_thread_clone);

        if cur_thread.get().is_termination_requested() {
            thread_list.remove(withholder_idx);

            KThread::reschedule(&mut cur_thread, ThreadState::Runnable);
            cur_thread.get().withholder = None;

            get_critical_section().leave();
        }
        else {
            if !timeout.is_zero() {
                get_time_manager().schedule_future_invocation(cur_thread.clone(), timeout);
            }

            get_critical_section().leave();

            if !timeout.is_zero() {
                get_time_manager().unschedule_future_invocation(cur_thread.clone());
            }
        }
    }

    pub fn notify_all(thread_list: &mut Vec<Shared<KThread>>) {
        let _guard = make_critical_section_guard();

        let mut remove_withholder_entries: Vec<Shared<KThread>> = Vec::new();
        for thread in thread_list.iter_mut() {
            if let Some(withholder_entry) = thread.get().withholder_entry.as_ref() {
                remove_withholder_entries.push(withholder_entry.clone());
            }

            thread.get().withholder_entry = None;
            thread.get().withholder = None;
            KThread::reschedule(thread, ThreadState::Runnable);
        }

        for obj in remove_withholder_entries.iter() {
            thread_list.retain(|thread_obj| !thread_obj.ptr_eq(obj));
        }
    }
}