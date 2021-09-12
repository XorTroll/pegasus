#include <pg/emu/cpu/cpu_CpuContext.hpp>
#include <pg/emu/kern/kern_SvcHandler.hpp>
#include <pg/emu/util/util_Exception.hpp>
#include <pg/ldr/ldr_Nso.hpp>
#include <pg/ldr/ldr_Results.hpp>
#include <pg/util/util_Align.hpp>
#include <lz4.h>

namespace pg::emu::cpu {

    namespace {

        std::map<u32, HookedInstructionHandler> g_HookedInstructions;

        void UnicornCodeHook(uc_engine *unicorn_engine, u64 address, u32 size, void *cpu_ctx_ref) {
            auto &cpu_ctx = *reinterpret_cast<CpuContext*>(cpu_ctx_ref);
            u32 cur_instruction;
            PG_RESULT_ASSERT(result::ConvertFromUnicornErrorCode(uc_mem_read(unicorn_engine, address, &cur_instruction, sizeof(cur_instruction))));

            const auto find_hooked_insn = g_HookedInstructions.find(cur_instruction);
            if(find_hooked_insn != g_HookedInstructions.end()) {
                PG_RESULT_ASSERT(find_hooked_insn->second(cpu_ctx));
            }
        }

        bool UnicornInvalidMemoryAccessHook(uc_engine *unicorn_engine, uc_mem_type type, u64 address, u32 size, u64 value, void*) {
            u64 pc;
            uc_reg_read(unicorn_engine, UC_ARM64_REG_PC, &pc);
            printf("Mem hook -> PC: 0x%lX\n", pc);

            switch(type) {
                case UC_MEM_READ_UNMAPPED:
                    util::ThrowExceptionFormatted<std::runtime_error>("not ok - Read from invalid memory at 0x%llX, data size = %u", address, size);
                    return false;
                case UC_MEM_WRITE_UNMAPPED:
                    util::ThrowExceptionFormatted<std::runtime_error>("not ok - Write to invalid memory at 0x%llX, data size = %u, data value = 0x%llX", address, size, value);
                    return false;
                case UC_MEM_FETCH_PROT:
                    util::ThrowExceptionFormatted<std::runtime_error>("not ok - Fetch from non-executable memory at 0x%llX", address);
                    return false;
                case UC_MEM_WRITE_PROT:
                    util::ThrowExceptionFormatted<std::runtime_error>("not ok - Write to non-writeable memory at 0x%llX, data size = %u, data value = 0x%llX", address, size, value);
                    return false;
                case UC_MEM_READ_PROT:
                    util::ThrowExceptionFormatted<std::runtime_error>("not ok - Read from non-readable memory at 0x%llX, data size = %u", address, size);
                    return false;
                default:
                    util::ThrowExceptionFormatted<std::runtime_error>("not ok - UC_HOOK_MEM_INVALID type: %d at 0x%llX", type, address);
                    return false;
            }
        }

        bool UnicornInvalidInstructionHook(uc_engine *unicorn_engine, void*) {
            throw std::runtime_error("Invalid instruction!");
        }

        void UnicornInterruptHook(uc_engine *unicorn_engine, u32 interrupt_no, void*) {
            printf("Unhandled interrupt: %d\n", interrupt_no);
        }

    }

    Result CpuContext::Initialize() {
        // Start the unicorn engine.
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_open(UC_ARCH_ARM64, UC_MODE_ARM, &this->unicorn_engine)));

        u64 fpv = 3 << 20;
        uc_reg_write(this->unicorn_engine, UC_ARM64_REG_CPACR_EL1, &fpv);

        // Register the code hook.
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_hook_add(this->unicorn_engine, &this->unicorn_code_hook, UC_HOOK_CODE, reinterpret_cast<void*>(UnicornCodeHook), this, 1, 0)));

        // Register the invalid memory access hook.
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_hook_add(this->unicorn_engine, &this->unicorn_invalid_mem_access_hook, UC_HOOK_MEM_INVALID, reinterpret_cast<void*>(UnicornInvalidMemoryAccessHook), nullptr, 1, 0)));

        // Register the invalid instruction hook.
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_hook_add(this->unicorn_engine, &this->unicorn_invalid_insn_hook, UC_HOOK_INSN_INVALID, reinterpret_cast<void*>(UnicornInvalidInstructionHook), nullptr, 1, 0)));

        // Register the interrupt hook.
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_hook_add(this->unicorn_engine, &this->unicorn_int_hook, UC_HOOK_INTR, reinterpret_cast<void*>(UnicornInterruptHook), nullptr, 1, 0)));

        return ResultSuccess;
    }

    Result CpuContext::Finalize() {
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_close(this->unicorn_engine)));
        this->unicorn_engine = nullptr;

        if(this->text_data != nullptr) {
            delete[] this->text_data;
            this->text_data = nullptr;
            this->text_data_size = 0;
        }

        if(this->rodata_data != nullptr) {
            delete[] this->rodata_data;
            this->rodata_data = nullptr;
            this->rodata_data_size = 0;
        }

        if(this->data_data != nullptr) {
            delete[] this->data_data;
            this->data_data = nullptr;
            this->data_data_size = 0;
        }

        return ResultSuccess;
    }

    Result CpuContext::ReadMemory(const u64 address, void *out_data, const size_t out_data_size) {
        return result::ConvertFromUnicornErrorCode(uc_mem_read(this->unicorn_engine, address, out_data, out_data_size));
    }
    
    Result CpuContext::WriteMemory(const u64 address, const void *data, const size_t data_size) {
        return result::ConvertFromUnicornErrorCode(uc_mem_write(this->unicorn_engine, address, data, data_size));
    }

    Result CpuContext::LoadNso(const u64 load_address, void *nso_data, const size_t nso_data_size) {
        const auto nso_header = reinterpret_cast<ldr::NsoHeader*>(nso_data);
        PG_RETURN_UNLESS(nso_header->magic == ldr::NsoHeader::Magic, ldr::result::ResultInvalidNso);

        const auto text_section_size = nso_header->text_segment.section_size;
        this->text_data_size = pg::util::AlignUp(text_section_size, 0x1000);
        this->text_data = new u8[this->text_data_size]();
        auto text_file_data = reinterpret_cast<u8*>(nso_data) + nso_header->text_segment.file_offset;
        if(static_cast<bool>(nso_header->flags & ldr::NsoFlags::TextCompressed)) {
            PG_RETURN_UNLESS(LZ4_decompress_safe(reinterpret_cast<const char*>(text_file_data), reinterpret_cast<char*>(this->text_data), nso_header->text_file_size, text_section_size) == text_section_size, ldr::result::ResultInvalidNso);
        }
        else {
            PG_RETURN_UNLESS(text_section_size == nso_header->text_file_size, ldr::result::ResultInvalidNso);
            std::memcpy(this->text_data, text_file_data, text_section_size);
        }
        this->text_address = load_address + nso_header->text_segment.memory_offset;
        printf("Mapping .text (size 0x%lX) at address 0x%lX...\n", this->text_data_size, this->text_address);
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_mem_map_ptr(this->unicorn_engine, this->text_address, this->text_data_size, UC_PROT_READ | UC_PROT_EXEC, this->text_data)));

        const auto rodata_section_size = nso_header->rodata_segment.section_size;
        this->rodata_data_size = pg::util::AlignUp(rodata_section_size, 0x1000);
        this->rodata_data = new u8[this->rodata_data_size]();
        auto rodata_file_data = reinterpret_cast<u8*>(nso_data) + nso_header->rodata_segment.file_offset;
        if(static_cast<bool>(nso_header->flags & ldr::NsoFlags::RodataCompressed)) {
            PG_RETURN_UNLESS(LZ4_decompress_safe(reinterpret_cast<const char*>(rodata_file_data), reinterpret_cast<char*>(this->rodata_data), nso_header->rodata_file_size, rodata_section_size) == rodata_section_size, ldr::result::ResultInvalidNso);
        }
        else {
            PG_RETURN_UNLESS(rodata_section_size == nso_header->rodata_file_size, ldr::result::ResultInvalidNso);
            std::memcpy(this->rodata_data, rodata_file_data, rodata_section_size);
        }
        this->rodata_address = load_address + nso_header->rodata_segment.memory_offset;
        printf("Mapping .rodata (size 0x%lX) at address 0x%lX...\n", this->rodata_data_size, this->rodata_address);
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_mem_map_ptr(this->unicorn_engine, this->rodata_address, this->rodata_data_size, UC_PROT_READ, this->rodata_data)));

        const auto data_section_size = nso_header->data_segment.section_size;
        this->data_data_size = pg::util::AlignUp(data_section_size, 0x1000);
        this->data_data = new u8[this->data_data_size]();
        auto data_file_data = reinterpret_cast<u8*>(nso_data) + nso_header->data_segment.file_offset;
        if(static_cast<bool>(nso_header->flags & ldr::NsoFlags::DataCompressed)) {
            PG_RETURN_UNLESS(LZ4_decompress_safe(reinterpret_cast<const char*>(data_file_data), reinterpret_cast<char*>(this->data_data), nso_header->data_file_size, data_section_size) == data_section_size, ldr::result::ResultInvalidNso);
        }
        else {
            PG_RETURN_UNLESS(data_section_size == nso_header->data_file_size, ldr::result::ResultInvalidNso);
            std::memcpy(this->data_data, data_file_data, data_section_size);
        }
        this->data_address = load_address + nso_header->data_segment.memory_offset;
        printf("Mapping .data (size 0x%lX) at address 0x%lX...\n", this->data_data_size, this->data_address);
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_mem_map_ptr(this->unicorn_engine, this->data_address, this->data_data_size, UC_PROT_READ | UC_PROT_WRITE, this->data_data)));

        this->bss_data_size = pg::util::AlignUp(nso_header->bss_size, 0x1000);
        this->bss_data = new u8[this->bss_data_size]();
        this->bss_address = this->data_address + this->data_data_size;
        printf("Mapping .bss (size 0x%lX) at address 0x%lX...\n", this->bss_data_size, this->bss_address);
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_mem_map_ptr(this->unicorn_engine, this->bss_address, this->bss_data_size, UC_PROT_READ | UC_PROT_WRITE, this->bss_data)));

        this->stack_address = pg::util::AlignUp(this->bss_address + this->bss_data_size + load_address, 0x1000);
        const auto stack_size = 0x100000;
        const auto stack_top = this->stack_address + stack_size;
        this->stack_data = new u8[stack_size]();
        this->stack_data_size = stack_size;
        printf("Mapping stack (size 0x%lX) at address 0x%lX...\n", this->stack_data_size, this->stack_address);
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_mem_map_ptr(this->unicorn_engine, this->stack_address, this->stack_data_size, UC_PROT_READ | UC_PROT_WRITE, this->stack_data)));
        
        this->tls_address = pg::util::AlignUp(this->stack_address + this->stack_data_size + load_address, 0x1000);
        this->tls_data_size = pg::util::AlignUp<size_t>(0x200, 0x1000);
        this->tls_data = new u8[this->tls_data_size]();
        printf("Mapping TLS (size 0x%lX) at address 0x%lX...\n", this->tls_data_size, this->tls_address);
        PG_RESULT_TRY(result::ConvertFromUnicornErrorCode(uc_mem_map_ptr(this->unicorn_engine, this->tls_address, this->tls_data_size, UC_PROT_READ | UC_PROT_WRITE, this->tls_data)));

        PG_RESULT_TRY(this->WriteRegister<UC_ARM64_REG_X0, u64>(0));
        PG_RESULT_TRY(this->WriteRegister<UC_ARM64_REG_X1, u32>(0xBABA));
        PG_RESULT_TRY(this->WriteRegister<UC_ARM64_REG_SP>(stack_top));
        PG_RESULT_TRY(this->WriteRegister<UC_ARM64_REG_TPIDRRO_EL0>(this->tls_address));

        return ResultSuccess;
    }

    Result CpuContext::Start() {
        return result::ConvertFromUnicornErrorCode(uc_emu_start(this->unicorn_engine, this->text_address, this->text_address + this->text_data_size, 0, 0));
    }

    void RegisterInstructionHook(const u32 instruction, HookedInstructionHandler handler) {
        // Add or replace the instruction handler.
        g_HookedInstructions[instruction] = handler;
    }

}