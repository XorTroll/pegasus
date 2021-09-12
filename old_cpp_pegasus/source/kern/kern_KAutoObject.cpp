#include <pg/kern/kern_KAutoObject.hpp>
#include <pg/kern/kern_Results.hpp>
#include <pg/util/util_Concurrent.hpp>
#include <map>

namespace pg::kern {

    namespace {

        util::ConcurrentObject<std::map<const char*, KAutoObject*>> g_NamedAutoObjectTable;

    }

    Result KAutoObject::SetName(const char *name) {
        PG_RETURN_UNLESS(g_NamedAutoObjectTable->find(name) == g_NamedAutoObjectTable->end(), result::ResultInvalidState);
        
        g_NamedAutoObjectTable->emplace(name, this);
        return ResultSuccess;
    }

    void KAutoObject::IncrementReferenceCount() {
        this->ref_count++;
        PG_ASSERT(this->ref_count > 1);
    }

    void KAutoObject::DecrementReferenceCount() {
        this->ref_count--;
        PG_ASSERT(this->ref_count >= 0);

        if(this->ref_count == 0) {
            this->Destroy();
        }
    }

    Result FindNamedObject(const char *name, KAutoObject *&out_obj) {
        auto find_obj = g_NamedAutoObjectTable->find(name);
        PG_RETURN_UNLESS(find_obj != g_NamedAutoObjectTable->end(), result::ResultInvalidState);

        out_obj = find_obj->second;
        return ResultSuccess;
    }

    Result RemoveNamedObject(const char *name) {
        PG_RETURN_UNLESS(g_NamedAutoObjectTable->find(name) != g_NamedAutoObjectTable->end(), result::ResultInvalidState);

        g_NamedAutoObjectTable->erase(name);
        return ResultSuccess;
    }

}