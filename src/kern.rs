use core::time;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};
use parking_lot::Mutex;
use rsevents::{AutoResetEvent, Awaitable};
use rsevents::State;
use crate::kern::thread::KConditionVariable;
use crate::util::{Shared, SharedObject, SharedAny, SharedCast, make_shared};
use crate::result::*;

pub mod thread;
use thread::ThreadState;
use thread::KThread;
use thread::get_critical_section;
use thread::make_critical_section_guard;

use self::svc::LimitableResource;
use self::thread::initialize_schedulers;

pub mod mem;

pub mod proc;

pub mod ipc;

pub mod svc;

pub mod result;

pub trait KAutoObject: Send + Sync {
    fn get_refcount(&mut self) -> &mut AtomicI32;

    fn increment_refcount(&mut self) {
        let refcount = self.get_refcount();
        refcount.fetch_add(1, Ordering::SeqCst);
        assert!(refcount.load(Ordering::SeqCst) > 1);
    }

    fn decrement_refcount(&mut self) {
        let refcount = self.get_refcount();
        let new_val = refcount.load(Ordering::SeqCst);
        assert!(new_val >= 0);

        if new_val == 0 {
            self.destroy();
        }
    }

    fn destroy(&mut self) {
    }
}

static mut G_NAMED_OBJECT_TABLE: Mutex<BTreeMap<String, SharedAny>> = parking_lot::const_mutex(BTreeMap::new());

pub fn register_named_object<K: KAutoObject + 'static>(obj: Shared<K>, name: &str) -> Result<()> {
    unsafe {
        let name_s = String::from(name);
        let mut named_object_table = G_NAMED_OBJECT_TABLE.lock();

        // TODO
        result_return_unless!(!named_object_table.contains_key(&name_s), 0xBABA);

        named_object_table.insert(name_s, obj.as_any());
        Ok(())
    }
}

pub fn remove_named_object(name: &str) -> Result<()> {
    unsafe {
        let name_s = String::from(name);
        let mut named_object_table = G_NAMED_OBJECT_TABLE.lock();
        // TODO
        result_return_unless!(named_object_table.contains_key(&name_s), 0xBABA);
        
        named_object_table.remove(&name_s);
        Ok(())
    }
}

pub fn find_named_object<K: KAutoObject + 'static>(name: &str) -> Result<Shared<K>> {
    unsafe {
        let name_s = String::from(name);
        let mut named_object_table = G_NAMED_OBJECT_TABLE.lock();

        if let Some(obj) = named_object_table.get(&name_s) {
            Ok(obj.cast::<K>())
        }
        else {
            // TODO
            Err(ResultCode::new(0xBABA))
        }
    }
}

// KSynchronizationObject

pub trait KSynchronizationObject : KAutoObject {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>>;

    fn add_waiting_thread(&mut self, thread: Shared<KThread>) -> usize {
        let waiting_threads = self.get_waiting_threads();
        let index = waiting_threads.len();
        waiting_threads.push(thread);
        index
    }

    fn remove_waiting_thread(&mut self, index: usize) {
        let waiting_threads = self.get_waiting_threads();

        if index < waiting_threads.len() {
            waiting_threads.remove(index);
        }
        // TODO: error if not found?
    }

    fn signal(obj: &mut Shared<Self>) where Self: 'static + Sized + Send + Sync {
        let _ = make_critical_section_guard();

        if obj.get().is_signaled() {
            let obj_clone = obj.clone();
            for wait_thread in obj.get().get_waiting_threads() {
                if wait_thread.get().state.get_low_flags() == ThreadState::Waiting {
                    wait_thread.get().signaled_obj = Some(obj_clone.clone());
                    KThread::reschedule(wait_thread, ThreadState::Runnable);
                }
            }
        }
    }

    fn is_signaled(&self) -> bool {
        false
    }
}

pub fn wait_for_sync_objects(objs: &mut [Shared<dyn KSynchronizationObject + Send + Sync>], timeout: i64) -> Result<usize> {
    get_critical_section().enter();

    for i in 0..objs.len() {
        let obj = &objs[i];

        if obj.get().is_signaled() {
            get_critical_section().leave();
            return Ok(i);
        }
    }

    if timeout == 0 {
        get_critical_section().leave();
        return Err(result::ResultTimedOut::make());
    }

    let mut cur_thread = thread::get_current_thread();

    if cur_thread.get().should_be_terminated || (cur_thread.get().state == ThreadState::Terminated) {
        get_critical_section().leave();
        return Err(result::ResultTerminationRequested::make());
    }
    else if cur_thread.get().sync_cancelled {
        get_critical_section().leave();
        return Err(result::ResultCancelled::make());
    }
    else {
        let mut thread_idxs: Vec<usize> = Vec::with_capacity(objs.len());
        for obj in objs.iter_mut() {
            thread_idxs.push(obj.get().add_waiting_thread(cur_thread.clone()));
        }

        cur_thread.get().waiting_sync = true;
        cur_thread.get().signaled_obj = None;
        
        KThread::reschedule(&mut cur_thread, ThreadState::Waiting);

        if timeout > 0 {
            // ScheduleFutureInvocation
        }

        get_critical_section().leave();

        cur_thread.get().waiting_sync = false;

        if timeout > 0 {
            // UnscheduleFutureInvocation
        }

        get_critical_section().enter();

        if let Some(signaled_obj) = cur_thread.get().signaled_obj.as_ref() {
            for i in 0..objs.len() {
                let obj = &mut objs[i];
                let index = thread_idxs[i];

                obj.get().remove_waiting_thread(index);
     
                if obj.ptr_eq(signaled_obj) {
                    get_critical_section().leave();
                    return Ok(i);
                }
            }
        }
    }

    get_critical_section().leave();
    return Err(result::ResultTimedOut::make());
}

// ---

static mut G_TIME_MANAGER: Option<KTimeManager> = None;

#[inline]
pub fn get_time_manager() -> &'static mut KTimeManager {
    unsafe {
        assert!(G_TIME_MANAGER.is_some());

        G_TIME_MANAGER.as_mut().unwrap()
    }
}

pub fn initialize_time_manager() -> Result<()> {
    unsafe {
        if G_TIME_MANAGER.is_none() {
            G_TIME_MANAGER = Some(KTimeManager::new()?);

            get_time_manager().start()?;
        }
    }

    Ok(())
}

// KFutureSchedulerObject

pub trait KFutureSchedulerObject: KAutoObject {
    fn time_up(&mut self);
}

// ---

// KTimeManager

pub struct KTimeManager {
    wait_event: AutoResetEvent,
    waiting_objs: Vec<(Shared<dyn KFutureSchedulerObject>, Instant)>,
    work_thread: Shared<KThread>
}

impl KTimeManager {
    pub fn new() -> Result<Self> {
        let work_thread = KThread::new_host(None, String::from("pg.kern.KTimeManagerWorkThread"), 10, 3)?;

        Ok(Self {
            wait_event: AutoResetEvent::new(State::Unset),
            waiting_objs: Vec::new(),
            work_thread: work_thread
        })
    }

    fn work_thread_fn() {
        log_line!("Hello World!");

        let time_manager = get_time_manager();
        loop {
            let next = {
                let _ = make_critical_section_guard();

                time_manager.waiting_objs.sort_by(|(_, a), (_, b)| a.cmp(b));
                time_manager.waiting_objs.first()
            };

            if let Some((next_obj, next_instant)) = next {
                let cur_instant = Instant::now();
                if *next_instant > cur_instant {
                    time_manager.wait_event.wait_for(next_instant.duration_since(cur_instant));
                }
                
                if Instant::now() >= *next_instant {
                    let _ = make_critical_section_guard();

                    for i in 0..time_manager.waiting_objs.len() {
                        let (obj, _) = &time_manager.waiting_objs[i];
                        if next_obj.ptr_eq(obj) {
                            let (r_obj, _) = time_manager.waiting_objs.remove(i);
                            r_obj.get().time_up();
                            break;
                        }
                    }
                }
            }
            else {
                time_manager.wait_event.wait();
            }
        }
    }

    pub fn start(&mut self) -> Result<()> {
        KThread::start_host(&mut self.work_thread, Self::work_thread_fn)
    }

    pub fn schedule_future_invocation(&mut self, obj: Shared<dyn KFutureSchedulerObject>, timeout: Duration) {
        todo!("schedule_future_invocation");
    }

    pub fn unschedule_future_invocation(&mut self, obj: Shared<dyn KFutureSchedulerObject>) {
        let _ = make_critical_section_guard();

        self.waiting_objs.retain(|(wait_obj, _)| !obj.ptr_eq(wait_obj));
    }
}

// ---

// KResourceLimit

pub const LIMITABLE_RESOURCE_COUNT: usize = 5;

pub struct KResourceLimit {
    refcount: AtomicI32,
    limit_values: [u64; LIMITABLE_RESOURCE_COUNT],
    current_values: [u64; LIMITABLE_RESOURCE_COUNT],
    current_hints: [u64; LIMITABLE_RESOURCE_COUNT],
    peak_values: [u64; LIMITABLE_RESOURCE_COUNT],
    waiting_threads: Vec<Shared<KThread>>,
    waiting_thread_count: usize
}

impl KAutoObject for KResourceLimit {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

impl KResourceLimit {
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

    pub fn new() -> Shared<Self> {
        make_shared(Self {
            refcount: AtomicI32::new(1),
            limit_values: [0; LIMITABLE_RESOURCE_COUNT],
            current_values: [0; LIMITABLE_RESOURCE_COUNT],
            current_hints: [0; LIMITABLE_RESOURCE_COUNT],
            peak_values: [0; LIMITABLE_RESOURCE_COUNT],
            waiting_threads: Vec::new(),
            waiting_thread_count: 0
        })
    }

    pub fn reserve(&mut self, kind: LimitableResource, value: u64, custom_timeout: Option<Duration>) -> Result<()> {
        let timeout = custom_timeout.unwrap_or(Self::DEFAULT_TIMEOUT);
        let instant = Instant::now() + timeout;
        let idx = kind as usize;
        result_return_unless!(self.current_hints[idx] < self.limit_values[idx], result::ResultInvalidState);
        
        let mut new_current_value = self.current_values[idx] + value;
        while (new_current_value > self.limit_values[idx]) && ((self.current_hints[idx] + value) <= self.limit_values[idx]) {
            self.waiting_thread_count += 1;
            KConditionVariable::wait(&mut self.waiting_threads, timeout);
            self.waiting_thread_count -= 1;

            new_current_value = self.current_values[idx] + value;

            if Instant::now() > instant {
                break;
            }
        }

        result_return_unless!(new_current_value <= self.limit_values[idx], result::ResultLimitReached);

        self.current_values[idx] += value;
        self.current_hints[idx] += value;
        Ok(())
    }

    pub fn release(&mut self, kind: LimitableResource, value: u64, hint: u64) {
        let idx = kind as usize;

        self.current_values[idx] -= value;
        self.current_hints[idx] -= hint;

        if self.waiting_thread_count > 0 {
            KConditionVariable::notify_all(&mut self.waiting_threads);
        }
    }

    pub fn get_remaining_value(&self, kind: LimitableResource) -> u64 {
        let idx = kind as usize;
        self.limit_values[idx] - self.current_values[idx]
    }

    pub fn set_limit_value(&mut self, kind: LimitableResource, value: u64) -> Result<()> {
        let idx = kind as usize;
        result_return_unless!(self.current_values[idx] <= self.limit_values[idx], result::ResultInvalidState);

        self.limit_values[idx] = value;
        Ok(())
    }
}

// ---

pub fn initialize() -> Result<()> {
    initialize_schedulers()?;
    initialize_time_manager()?;

    Ok(())
}