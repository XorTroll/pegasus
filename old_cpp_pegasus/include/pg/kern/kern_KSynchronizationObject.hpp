
#pragma once
#include <pg/kern/kern_KAutoObject.hpp>
#include <vector>

namespace pg::kern {

    class KThread;

    class KSynchronizationObject : public KAutoObject {
        private:
            std::vector<KThread> waiting_threads;

        public:
            KSynchronizationObject() : waiting_threads() {}

            void AddWaitingThread(KThread *thread);
            void RemoveWaitingThread(KThread *thread);
            virtual void Signal();
            virtual bool IsSignaled();
    };

}