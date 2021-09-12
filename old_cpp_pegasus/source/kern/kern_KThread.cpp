#include <pg/kern/kern_KThread.hpp>

namespace pg::kern {

    thread_local KThread *g_CurrentThread;

    KThread *GetCurrentThread() {
        PG_ASSERT(g_CurrentThread != nullptr);
        return g_CurrentThread;
    }

}