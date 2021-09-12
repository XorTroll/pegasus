#include <pg/emu/emu_Base.hpp>
#include <pg/emu/cpu/cpu_CpuContext.hpp>
#include <pg/emu/kern/kern_SvcHandler.hpp>

#include <cstring>

int main() {
    PG_RESULT_ASSERT(pg::emu::Initialize());

    pg::emu::cpu::CpuContext cpu_ctx;
    PG_RESULT_ASSERT(cpu_ctx.Initialize());

    auto nso = fopen("/mnt/c/Users/XaboF/OneDrive/Desktop/pegasus/nso_test/nso_test.nso", "rb");
    if(nso) {
        fseek(nso, 0, SEEK_END);
        const auto size = ftell(nso);
        rewind(nso);

        auto nso_data = new pg::u8[size]();
        fread(nso_data, size, 1, nso);

        PG_RESULT_ASSERT(cpu_ctx.LoadNso(0x8000000, nso_data, size));

        delete[] nso_data;

        fclose(nso);
    }

    const auto start_rc = cpu_ctx.Start();
    pg::u64 pc;
    cpu_ctx.ReadRegister<UC_ARM64_REG_PC>(pc);
    printf("Finish PC: 0x%lX\n", pc);
    PG_RESULT_ASSERT(start_rc);

    PG_RESULT_ASSERT(cpu_ctx.Finalize());
    return 0;
}