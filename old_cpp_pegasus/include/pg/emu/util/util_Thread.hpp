
#pragma once
#include <pg/pg_Result.hpp>
#include <pthread.h>
#include <functional>

namespace pg::emu::util {

    class Thread {
        private:
            pthread_t inner_thread;
            const char *name;
            std::function<void()> entry;

            static void *EntrypointImpl(void *thread_ref);

        public:
            Thread() : inner_thread(), name(nullptr), entry() {}
            Thread(const char *name, std::function<void()> entry) : inner_thread(), name(name), entry(entry) {}
            Result Start();
            Result Join();

            inline void SetName(const char *name) {
                this->name = name;
            }

            inline const char *GetName() const {
                return this->name;
            }

            inline bool HasName() const {
                return this->name != nullptr;
            } 
    };

    void InitializeMainThread(const char *name);
    Thread &GetCurrentThread();

}