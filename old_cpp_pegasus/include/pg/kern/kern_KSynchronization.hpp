
#pragma once
#include <pg/pg_Result.hpp>
#include <pg/kern/kern_KSynchronizationObject.hpp>

namespace pg::kern {

    Result WaitFor(const std::vector<KSynchronizationObject*> &objs, i64 timeout, i32 &out_handle_idx);

}