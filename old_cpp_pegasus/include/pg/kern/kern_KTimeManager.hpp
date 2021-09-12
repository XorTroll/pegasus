
#pragma once
#include <pg/kern/kern_KFutureSchedulerObject.hpp>
#include <pg/emu/util/util_Thread.hpp>

namespace pg::kern {

    class KTimeManager {
        private:
            struct WaitingObject {
                KFutureSchedulerObject *obj;
                i64 time_point;
            };

            emu::util::Thread work_thread;
            std::vector<WaitingObject> waiting_objs;

        public:
            KTimeManager();
    };

}