
#pragma once

namespace pg::kern {

    class KFutureSchedulerObject {
        public:
            virtual void TimeUp() = 0;
    };

}