#include <pg/emu/emu_Base.hpp>
#include <pg/emu/util/util_Thread.hpp>
#include <pg/emu/cpu/cpu_CpuContext.hpp>
#include <pg/emu/kern/kern_SvcHandler.hpp>

namespace pg::emu {

    Result Initialize() {
        // Initialize main thread.
        util::InitializeMainThread("emu.MainThread");

        // Register global SVC handlers.
        for(const auto &[svc_id, svc_handler] : kern::GetSvcHandlerTable()) {
            cpu::RegisterSvcHook(svc_id, svc_handler);
        }

        return ResultSuccess;
    }

}