
#pragma once
#include <pg/kern/kern_Base.hpp>

namespace pg::kern::svc {

    enum class BreakReason : u32 {
        Panic = 0,
        Assert = 1,
        User = 2,
        PreLoadDll = 3,
        PostLoadDll = 4,
        PreUnloadDll = 5,
        PostUnloadDll = 6,
        CppException = 7,
        NotificationOnlyFlag = 0x80000000
    };
    PG_DEFINE_FLAG_ENUM(BreakReason, u32)

    void Break(BreakReason reason, void *arg, size_t arg_size);

}