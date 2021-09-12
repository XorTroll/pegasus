
#pragma once
#include <pg/pg_Result.hpp>
#include <pg/emu/cpu/cpu_Results.hpp>
#include <pg/kern/kern_Base.hpp>
#include <unicorn/unicorn.h>
#include <functional>
#include <map>

namespace pg::emu::cpu {

    class CpuContext;

    using HookedInstructionHandler = std::function<Result(CpuContext&)>;

    class CpuContext {
        private:
            uc_engine *unicorn_engine;
            uc_hook unicorn_code_hook;
            uc_hook unicorn_invalid_mem_access_hook;
            uc_hook unicorn_invalid_insn_hook;
            uc_hook unicorn_int_hook;
            std::map<u32, HookedInstructionHandler> hooked_instructions;
            u64 start_address;
            u8 *text_data;
            size_t text_data_size;
            u64 text_address;
            u8 *rodata_data;
            size_t rodata_data_size;
            u64 rodata_address;
            u8 *data_data;
            size_t data_data_size;
            u64 data_address;
            u8 *bss_data;
            size_t bss_data_size;
            u64 bss_address;
            u8 *stack_data;
            size_t stack_data_size;
            u64 stack_address;
            u8 *tls_data;
            size_t tls_data_size;
            u64 tls_address;

        public:
            CpuContext() : unicorn_engine(nullptr), hooked_instructions() {}

            Result Initialize();
            Result Finalize();

            template<uc_arm64_reg Reg, typename T>
            Result ReadRegister(T &out_t) {
                return result::ConvertFromUnicornErrorCode(uc_reg_read(this->unicorn_engine, Reg, std::addressof(out_t)));
            }

            template<uc_arm64_reg Reg, typename T>
            Result WriteRegister(const T &t) {
                return result::ConvertFromUnicornErrorCode(uc_reg_write(this->unicorn_engine, Reg, std::addressof(t)));
            }

            Result ReadMemory(const u64 address, void *out_data, const size_t out_data_size);
            Result WriteMemory(const u64 address, const void *data, const size_t data_size);

            template<typename T>
            inline Result ReadMemoryValue(const u64 address, T &out_t) {
                return this->ReadMemory(address, std::addressof(out_t), sizeof(out_t));
            }

            template<typename T>
            inline Result WriteMemoryValue(const u64 address, const T &t) {
                return this->WriteMemory(address, std::addressof(t), sizeof(t));
            }

            Result LoadNso(const u64 load_address, void *nso_data, const size_t nso_data_size);
            Result Start();
    };

    void RegisterInstructionHook(const u32 instruction, HookedInstructionHandler handler);

    inline void RegisterSvcHook(const kern::SvcId svc_id, HookedInstructionHandler handler) {
        const u32 svc_instruction = 0xD4000001 | (static_cast<u8>(svc_id) << 5);
        return RegisterInstructionHook(svc_instruction, handler);
    }

}