
#pragma once
#include <mutex>

namespace pg::kern {

    class KCriticalSection {
        private:
            std::mutex lock;
            int recursion_count;
        
        public:
            KCriticalSection() : lock(), recursion_count(0) {}

            void Enter();
            void Leave();
    };

}