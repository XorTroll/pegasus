use std::borrow::Borrow;
use std::borrow::BorrowMut;
use std::collections::BTreeMap;
use std::path::Prefix;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::thread::Thread;
use rsevents::AutoResetEvent;
use rsevents::ManualResetEvent;
use rsevents::State;
use parking_lot::lock_api::RawMutex as RawMutexTrait;
use parking_lot::RawMutex;
use crate::emu::cpu;
use crate::ldr::npdm;
use crate::util::Shared;
use crate::result::*;

pub mod thread;
use thread::ThreadState;
use thread::KThread;
use thread::KCriticalSectionGuard;
use thread::KCriticalSection;

pub mod mem;

pub mod proc;

pub mod svc;

pub mod result;

pub type Handle = u32;

pub trait KAutoObject {
    fn get_refcount(&mut self) -> &mut AtomicI32;

    fn increment_refcount(&mut self) {
        let mut refcount = self.get_refcount();
        refcount.fetch_add(1, Ordering::SeqCst);
        assert!(refcount.load(Ordering::SeqCst) > 1);
    }

    fn decrement_refcount(&mut self) {
        let mut refcount = self.get_refcount();
        let new_val = refcount.load(Ordering::SeqCst);
        assert!(new_val >= 0);

        if new_val == 0 {
            self.destroy();
        }
    }

    fn destroy(&mut self);
}

static mut G_NAMED_OBJECT_TABLE: BTreeMap<String, Shared<dyn KAutoObject>> = BTreeMap::new();

pub fn register_object_name(obj: Shared<dyn KAutoObject>, name: String) -> Result<()> {
    unsafe {
        // TODO
        result_return_unless!(!G_NAMED_OBJECT_TABLE.contains_key(&name), 0xBABA);

        G_NAMED_OBJECT_TABLE.insert(name, obj);
        Ok(())
    }
}

pub fn remove_object_name(name: String) -> Result<()> {
    unsafe {
        // TODO
        result_return_unless!(G_NAMED_OBJECT_TABLE.contains_key(&name), 0xBABA);
        
        G_NAMED_OBJECT_TABLE.remove(&name);
        Ok(())
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
        let mut waiting_threads = self.get_waiting_threads();

        if index < waiting_threads.len() {
            waiting_threads.remove(index);
        }
        // TODO: error if not found?
    }

    fn signal(obj: &mut Shared<Self>) where Self: 'static + Sized + Send + Sync {
        let _ = KCriticalSectionGuard::new(thread::get_critical_section());

        if obj.get().is_signaled() {
            let obj_clone = obj.clone();
            for wait_thread in obj.get().get_waiting_threads() {
                if (wait_thread.get().state.get_low_flags() == ThreadState::Waiting) {
                    wait_thread.get().signaled_obj = Some(obj_clone.clone());
                    KThread::reschedule(wait_thread, ThreadState::Runnable);
                }
            }
        }
    }

    fn is_signaled(&self) -> bool;
}

pub fn wait_for_sync_objects(objs: &mut [Shared<dyn KSynchronizationObject + Send + Sync>], timeout: i64) -> Result<usize> {
    thread::get_critical_section().enter();

    for i in 0..objs.len() {
        let obj = &objs[i];

        if obj.get().is_signaled() {
            thread::get_critical_section().leave();
            return Ok(i);
        }
    }

    if timeout == 0 {
        thread::get_critical_section().leave();
        return Err(result::ResultTimedOut::make());
    }

    let mut cur_thread = thread::get_current_thread();

    if cur_thread.get().should_be_terminated || (cur_thread.get().state == ThreadState::Terminated) {
        thread::get_critical_section().leave();
        return Err(result::ResultTerminationRequested::make());
    }
    else if cur_thread.get().sync_cancelled {
        thread::get_critical_section().leave();
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

        thread::get_critical_section().leave();

        cur_thread.get().waiting_sync = false;

        if timeout > 0 {
            // UnscheduleFutureInvocation
        }

        thread::get_critical_section().enter();

        if let Some(signaled_obj) = cur_thread.get().signaled_obj.as_ref() {
            for i in 0..objs.len() {
                let obj = &mut objs[i];
                let index = thread_idxs[i];

                obj.get().remove_waiting_thread(index);
     
                if obj.ptr_eq(signaled_obj) {
                    thread::get_critical_section().leave();
                    return Ok(i);
                }
            }
        }
    }

    thread::get_critical_section().leave();
    return Err(result::ResultTimedOut::make());
}

// ---

// KTimeManager

struct WaitObject {

}

static mut G_WAIT_EVENT: Option<AutoResetEvent> = None;
static mut G_WAIT_OBJECTS: Vec<WaitObject> = Vec::new();
static mut G_KEEP_RUNNING: bool = true;

fn wait_thread_fn() {
    unsafe {
        G_WAIT_EVENT = Some(AutoResetEvent::new(State::Unset));

        while G_KEEP_RUNNING {
            {
                let guard = KCriticalSectionGuard::new(thread::get_critical_section());
                // ...
            }
        }
    }
}

pub fn initialize_time_manager() {

}

// ---