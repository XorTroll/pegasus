#include <pg/kern/kern_KSynchronization.hpp>
#include <pg/kern/kern_KCriticalSection.hpp>
#include <pg/kern/kern_KThread.hpp>
#include <pg/kern/kern_Results.hpp>

namespace pg::kern {

    namespace {

        KCriticalSection g_CriticalSection;

    }

    Result WaitFor(const std::vector<KSynchronizationObject*> &objs, i64 timeout, i32 &out_handle_idx) {
        g_CriticalSection.Enter();

        // Check if any of the objects are already signaled
        for(i32 i = 0; i < objs.size(); i++) {
            if(objs[i]->IsSignaled()) {
                out_handle_idx = i;

                g_CriticalSection.Leave();
                return ResultSuccess;
            }
        }

        if(timeout == 0) {
            g_CriticalSection.Leave();
            return result::ResultTimedOut;
        }

        auto cur_thread = GetCurrentThread();

        if(cur_thread->ShouldBeTerminated()) {
            g_CriticalSection.Leave();
            return result::ResultTerminationRequested;
        }
        else if(...) {

        }
        else {
            for(auto &obj: objs) {
                obj->AddWaitingThread(cur_thread);
            }

            cur_thread->waiting_sync = true;
            cur_thread->signaled_obj = nullptr;
            // objsyncresult

            cur_thread->Reschedule(ThreadState::Waiting);

            if(timeout > 0) {
                
            }
        }
    }

}