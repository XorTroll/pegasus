
#pragma once
#include <pg/pg_Result.hpp>
#include <atomic>

namespace pg::kern {

    class KAutoObject {
        private:
            std::atomic<int> ref_count;

        protected:
            virtual void Destroy() {}

        public:
            KAutoObject() : ref_count(1) {}

            Result SetName(const char *name);
            void IncrementReferenceCount();
            void DecrementReferenceCount();
    };

    Result FindNamedObject(const char *name, KAutoObject *&out_obj);
    Result RemoveNamedObject(const char *name);

}