
#pragma once
#include <mutex>

namespace pg::util {

    template<typename T>
    class ConcurrentObject {
        private:
            std::mutex lock;
            T t;

        public:
            ConcurrentObject() : lock(), t() {}
            ConcurrentObject(const T &t) : lock(), t(t) {}

            inline T *operator->() {
                std::scoped_lock lk(this->lock);
                return std::addressof(this->t);
            }
    };

}