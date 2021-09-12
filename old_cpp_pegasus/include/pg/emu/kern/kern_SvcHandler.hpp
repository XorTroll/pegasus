
#pragma once
#include <pg/kern/kern_Base.hpp>
#include <pg/emu/cpu/cpu_CpuContext.hpp>

namespace pg::emu::kern {

    using namespace pg::kern;

    using SvcHandlerTable = std::map<SvcId, cpu::HookedInstructionHandler>;

    SvcHandlerTable &GetSvcHandlerTable();

}