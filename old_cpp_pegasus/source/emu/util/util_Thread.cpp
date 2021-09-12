#include <pg/emu/util/util_Thread.hpp>

namespace pg::emu::util {

    namespace {

        thread_local Thread *g_CurrentThread;
        Thread g_MainThread;

    }

    void *Thread::EntrypointImpl(void *thread_ref) {
        auto thread_ptr = reinterpret_cast<Thread*>(thread_ref);

        // Set current thread.
        g_CurrentThread = thread_ptr;

        // Call actual thread.
        thread_ptr->entry();

        g_CurrentThread = nullptr;
        pthread_exit(nullptr);
        return nullptr;
    }

    Result Thread::Start() {
        PG_RETURN_UNLESS(pthread_create(&this->inner_thread, nullptr, EntrypointImpl, this) == 0, 0xabB);
        return ResultSuccess;
    }

    Result Thread::Join() {
        PG_RETURN_UNLESS(pthread_join(this->inner_thread, nullptr) == 0, 0xabB);
        return ResultSuccess;
    }

    void InitializeMainThread(const char *name) {
        g_MainThread.SetName(name);
        g_CurrentThread = std::addressof(g_MainThread);
    }

    Thread &GetCurrentThread() {
        PG_ASSERT(g_CurrentThread != nullptr);
        return *g_CurrentThread;
    }

}