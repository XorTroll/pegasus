#include <pg/kern/svc/svc_Break.hpp>
#include <pg/emu/util/util_Exception.hpp>

namespace pg::kern::svc {

    void Break(BreakReason reason, void *arg, size_t arg_size) {
        if(static_cast<bool>(reason & BreakReason::NotificationOnlyFlag)) {
            printf("[Break] Notication only...");
        }
        else {
            if(arg_size == sizeof(Result)) {
                // Breaking with a result value!
                const auto rc = *reinterpret_cast<Result*>(arg);
                emu::util::ThrowExceptionFormatted<std::runtime_error>("[Break] Reason: %d, Result: " PG_RESULT_FMT_STR, static_cast<u32>(reason), 2000 + rc.GetModule(), rc.GetDescription());
            }
            else {
                emu::util::ThrowExceptionFormatted<std::runtime_error>("[Break] Reason: %d, Arg: %p, Size: 0x%lX", static_cast<u32>(reason), arg, arg_size);
            }
        }
    }

}