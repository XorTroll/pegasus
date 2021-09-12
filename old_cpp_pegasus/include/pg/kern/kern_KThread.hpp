
#pragma once
#include <pg/kern/kern_KSynchronizationObject.hpp>

namespace pg::kern {

    enum class SuspendType : u32 {
        Process   = 0,
        Thread    = 1,
        Debug     = 2,
        Backtrace = 3,
        Init      = 4,

        SuspendType_Count,
    };

    enum class ThreadState : u16 {
        Initialized = 0,
        Waiting     = 1,
        Runnable    = 2,
        Terminated  = 3,

        ProcessSuspended = 1 << 4,
        ThreadSuspended = 1 << 5,
        DebugSuspended = 1 << 6,
        BacktraceSuspended = 1 << 7,
        InitSuspended = 1 << 8,

        LowMask = (1 << 4) - 1,
        HighMask = 0xfff0,
        ForcePauseMask = 0x70,
    };

    class KThread : public KSynchronizationObject {
        public:
            std::atomic_bool should_be_terminated;
            bool waiting_sync;
            KSynchronizationObject *signaled_obj;
            ThreadState state;

            void Reschedule(const ThreadState new_state);
    };

    KThread *GetCurrentThread();

}