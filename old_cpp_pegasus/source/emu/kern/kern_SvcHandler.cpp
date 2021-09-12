#include <pg/emu/kern/kern_SvcHandler.hpp>
#include <pg/kern/svc/svc_Break.hpp>
#include <pg/kern/svc/svc_Debug.hpp>

namespace pg::emu::kern {

    namespace {

        template<SvcId Id>
        Result UnimplementedSvc(cpu::CpuContext &cpu_ctx) {
            throw std::runtime_error("Called unimplemented SVC: " + std::to_string(static_cast<u8>(Id)));
        }

        template<SvcId Id>
        inline const std::pair<SvcId, cpu::HookedInstructionHandler> MakeUnimplementedSvcHandler() {
            return { Id, UnimplementedSvc<Id> };
        }

        Result DoBreak(cpu::CpuContext &cpu_ctx) {
            svc::BreakReason reason;
            PG_RESULT_TRY(cpu_ctx.ReadRegister<UC_ARM64_REG_W0>(reason));
            u64 arg_addr;
            PG_RESULT_TRY(cpu_ctx.ReadRegister<UC_ARM64_REG_X1>(arg_addr));
            size_t arg_size;
            PG_RESULT_TRY(cpu_ctx.ReadRegister<UC_ARM64_REG_X2>(arg_size));

            if((arg_addr != 0) && (arg_size != 0)) {
                auto arg = new u8[arg_size]();
                PG_RESULT_TRY(cpu_ctx.ReadMemory(arg_addr, arg, arg_size));

                svc::Break(reason, arg, arg_size);

                delete[] arg;
            }
            else {
                svc::Break(reason, nullptr, 0);
            }

            return ResultSuccess;
        }

        Result DoOutputDebugString(cpu::CpuContext &cpu_ctx) {
            u64 str_addr;
            PG_RESULT_TRY(cpu_ctx.ReadRegister<UC_ARM64_REG_X0>(str_addr));
            size_t str_len;
            PG_RESULT_TRY(cpu_ctx.ReadRegister<UC_ARM64_REG_X1>(str_len));
            
            auto str_buf = new char[str_len]();
            PG_RESULT_TRY(cpu_ctx.ReadMemory(str_addr, str_buf, str_len));

            const auto rc = svc::OutputDebugString(str_buf, str_len);

            delete[] str_buf;
            PG_RESULT_TRY(cpu_ctx.WriteRegister<UC_ARM64_REG_W0>(rc));
            return ResultSuccess;
        }

        SvcHandlerTable g_SvcHandlerTable = {
            MakeUnimplementedSvcHandler<SvcId::SetHeapSize>(),
            MakeUnimplementedSvcHandler<SvcId::SetMemoryPermission>(),
            MakeUnimplementedSvcHandler<SvcId::SetMemoryAttribute>(),
            MakeUnimplementedSvcHandler<SvcId::MapMemory>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapMemory>(),
            MakeUnimplementedSvcHandler<SvcId::QueryMemory>(),
            MakeUnimplementedSvcHandler<SvcId::ExitProcess>(),
            MakeUnimplementedSvcHandler<SvcId::CreateThread>(),
            MakeUnimplementedSvcHandler<SvcId::StartThread>(),
            MakeUnimplementedSvcHandler<SvcId::ExitThread>(),
            MakeUnimplementedSvcHandler<SvcId::SleepThread>(),
            MakeUnimplementedSvcHandler<SvcId::GetThreadPriority>(),
            MakeUnimplementedSvcHandler<SvcId::SetThreadPriority>(),
            MakeUnimplementedSvcHandler<SvcId::GetThreadCoreMask>(),
            MakeUnimplementedSvcHandler<SvcId::SetThreadCoreMask>(),
            MakeUnimplementedSvcHandler<SvcId::GetCurrentProcessorNumber>(),
            MakeUnimplementedSvcHandler<SvcId::SignalEvent>(),
            MakeUnimplementedSvcHandler<SvcId::ClearEvent>(),
            MakeUnimplementedSvcHandler<SvcId::MapSharedMemory>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapSharedMemory>(),
            MakeUnimplementedSvcHandler<SvcId::CreateTransferMemory>(),
            MakeUnimplementedSvcHandler<SvcId::CloseHandle>(),
            MakeUnimplementedSvcHandler<SvcId::ResetSignal>(),
            MakeUnimplementedSvcHandler<SvcId::WaitSynchronization>(),
            MakeUnimplementedSvcHandler<SvcId::CancelSynchronization>(),
            MakeUnimplementedSvcHandler<SvcId::ArbitrateLock>(),
            MakeUnimplementedSvcHandler<SvcId::ArbitrateUnlock>(),
            MakeUnimplementedSvcHandler<SvcId::WaitProcessWideKeyAtomic>(),
            MakeUnimplementedSvcHandler<SvcId::SignalProcessWideKey>(),
            MakeUnimplementedSvcHandler<SvcId::GetSystemTick>(),
            MakeUnimplementedSvcHandler<SvcId::ConnectToNamedPort>(),
            MakeUnimplementedSvcHandler<SvcId::SendSyncRequestLight>(),
            MakeUnimplementedSvcHandler<SvcId::SendSyncRequest>(),
            MakeUnimplementedSvcHandler<SvcId::SendSyncRequestWithUserBuffer>(),
            MakeUnimplementedSvcHandler<SvcId::SendAsyncRequestWithUserBuffer>(),
            MakeUnimplementedSvcHandler<SvcId::GetProcessId>(),
            MakeUnimplementedSvcHandler<SvcId::GetThreadId>(),
            { SvcId::Break, DoBreak },
            { SvcId::OutputDebugString, DoOutputDebugString },
            MakeUnimplementedSvcHandler<SvcId::ReturnFromException>(),
            MakeUnimplementedSvcHandler<SvcId::GetInfo>(),
            MakeUnimplementedSvcHandler<SvcId::FlushEntireDataCache>(),
            MakeUnimplementedSvcHandler<SvcId::FlushDataCache>(),
            MakeUnimplementedSvcHandler<SvcId::MapPhysicalMemory>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapPhysicalMemory>(),
            MakeUnimplementedSvcHandler<SvcId::GetFutureThreadInfo>(),
            MakeUnimplementedSvcHandler<SvcId::GetLastThreadInfo>(),
            MakeUnimplementedSvcHandler<SvcId::GetResourceLimitLimitValue>(),
            MakeUnimplementedSvcHandler<SvcId::GetResourceLimitCurrentValue>(),
            MakeUnimplementedSvcHandler<SvcId::SetThreadActivity>(),
            MakeUnimplementedSvcHandler<SvcId::GetThreadContext3>(),
            MakeUnimplementedSvcHandler<SvcId::WaitForAddress>(),
            MakeUnimplementedSvcHandler<SvcId::SignalToAddress>(),
            MakeUnimplementedSvcHandler<SvcId::DumpInfo_KernelDebug>(),
            MakeUnimplementedSvcHandler<SvcId::ChangeKernelTraceState>(),
            MakeUnimplementedSvcHandler<SvcId::CreateSession>(),
            MakeUnimplementedSvcHandler<SvcId::AcceptSession>(),
            MakeUnimplementedSvcHandler<SvcId::ReplyAndReceiveLight>(),
            MakeUnimplementedSvcHandler<SvcId::ReplyAndReceive>(),
            MakeUnimplementedSvcHandler<SvcId::ReplyAndReceiveWithUserBuffer>(),
            MakeUnimplementedSvcHandler<SvcId::CreateEvent>(),
            MakeUnimplementedSvcHandler<SvcId::MapPhysicalMemoryUnsafe>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapPhysicalMemoryUnsafe>(),
            MakeUnimplementedSvcHandler<SvcId::SetUnsafeLimit>(),
            MakeUnimplementedSvcHandler<SvcId::CreateCodeMemory>(),
            MakeUnimplementedSvcHandler<SvcId::ControlCodeMemory>(),
            MakeUnimplementedSvcHandler<SvcId::SleepSystem>(),
            MakeUnimplementedSvcHandler<SvcId::ReadWriteRegister>(),
            MakeUnimplementedSvcHandler<SvcId::SetProcessActivity>(),
            MakeUnimplementedSvcHandler<SvcId::CreateSharedMemory>(),
            MakeUnimplementedSvcHandler<SvcId::MapTransferMemory>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapTransferMemory>(),
            MakeUnimplementedSvcHandler<SvcId::CreateInterruptEvent>(),
            MakeUnimplementedSvcHandler<SvcId::QueryPhysicalAddress>(),
            MakeUnimplementedSvcHandler<SvcId::QueryIoMapping>(),
            MakeUnimplementedSvcHandler<SvcId::CreateDeviceAddressSpace>(),
            MakeUnimplementedSvcHandler<SvcId::AttachDeviceAddressSpace>(),
            MakeUnimplementedSvcHandler<SvcId::DetachDeviceAddressSpace>(),
            MakeUnimplementedSvcHandler<SvcId::MapDeviceAddressSpaceByForce>(),
            MakeUnimplementedSvcHandler<SvcId::MapDeviceAddressSpaceAligned>(),
            MakeUnimplementedSvcHandler<SvcId::MapDeviceAddressSpace>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapDeviceAddressSpace>(),
            MakeUnimplementedSvcHandler<SvcId::InvalidateProcessDataCache>(),
            MakeUnimplementedSvcHandler<SvcId::StoreProcessDataCache>(),
            MakeUnimplementedSvcHandler<SvcId::FlushProcessDataCache>(),
            MakeUnimplementedSvcHandler<SvcId::DebugActiveProcess>(),
            MakeUnimplementedSvcHandler<SvcId::BreakDebugProcess>(),
            MakeUnimplementedSvcHandler<SvcId::TerminateDebugProcess>(),
            MakeUnimplementedSvcHandler<SvcId::GetDebugEvent>(),
            MakeUnimplementedSvcHandler<SvcId::ContinueDebugEvent>(),
            MakeUnimplementedSvcHandler<SvcId::GetProcessList>(),
            MakeUnimplementedSvcHandler<SvcId::GetThreadList>(),
            MakeUnimplementedSvcHandler<SvcId::GetDebugThreadContext>(),
            MakeUnimplementedSvcHandler<SvcId::SetDebugThreadContext>(),
            MakeUnimplementedSvcHandler<SvcId::QueryDebugProcessMemory>(),
            MakeUnimplementedSvcHandler<SvcId::ReadDebugProcessMemory>(),
            MakeUnimplementedSvcHandler<SvcId::WriteDebugProcessMemory>(),
            MakeUnimplementedSvcHandler<SvcId::SetHardwareBreakPoint>(),
            MakeUnimplementedSvcHandler<SvcId::GetDebugThreadParam>(),
            MakeUnimplementedSvcHandler<SvcId::GetSystemInfo>(),
            MakeUnimplementedSvcHandler<SvcId::CreatePort>(),
            MakeUnimplementedSvcHandler<SvcId::ManageNamedPort>(),
            MakeUnimplementedSvcHandler<SvcId::ConnectToPort>(),
            MakeUnimplementedSvcHandler<SvcId::SetProcessMemoryPermission>(),
            MakeUnimplementedSvcHandler<SvcId::MapProcessMemory>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapProcessMemory>(),
            MakeUnimplementedSvcHandler<SvcId::QueryProcessMemory>(),
            MakeUnimplementedSvcHandler<SvcId::MapProcessCodeMemory>(),
            MakeUnimplementedSvcHandler<SvcId::UnmapProcessCodeMemory>(),
            MakeUnimplementedSvcHandler<SvcId::CreateProcess>(),
            MakeUnimplementedSvcHandler<SvcId::StartProcess>(),
            MakeUnimplementedSvcHandler<SvcId::TerminateProcess>(),
            MakeUnimplementedSvcHandler<SvcId::GetProcessInfo>(),
            MakeUnimplementedSvcHandler<SvcId::CreateResourceLimit>(),
            MakeUnimplementedSvcHandler<SvcId::SetResourceLimitLimitValue>(),
            MakeUnimplementedSvcHandler<SvcId::CallSecureMonitor>(),
        };

    }

    SvcHandlerTable &GetSvcHandlerTable() {
        return g_SvcHandlerTable;
    }

}