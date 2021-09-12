
#pragma once
#include <pg/util/util_Magic.hpp>

namespace pg::ldr {

    enum class NsoFlags : u32 {
        TextCompressed = 1 << 0,
        RodataCompressed = 1 << 1,
        DataCompressed = 1 << 2,
        TextCheckHash = 1 << 3,
        RodataCheckHash = 1 << 4,
        DataCheckHash = 1 << 5
    };
    PG_DEFINE_FLAG_ENUM(NsoFlags, u32)

    struct NsoSegmentHeader {
        u32 file_offset;
        u32 memory_offset;
        u32 section_size;
    };

    struct NsoRodataRelativeSegmentHeader {
        u32 offset;
        u32 size;
    };

    struct NsoHeader {
        u32 magic;
        u32 version;
        u8 reserved_1[4];
        NsoFlags flags;
        NsoSegmentHeader text_segment;
        u32 module_name_offset;
        NsoSegmentHeader rodata_segment;
        u32 module_name_size;
        NsoSegmentHeader data_segment;
        u32 bss_size;
        u8 module_id[0x20];
        u32 text_file_size;
        u32 rodata_file_size;
        u32 data_file_size;
        u8 reserved_2[0x1C];
        NsoRodataRelativeSegmentHeader rodata_api_info_segment;
        NsoRodataRelativeSegmentHeader rodata_dynstr_segment;
        NsoRodataRelativeSegmentHeader rodata_dynsym_segment;
        u8 text_hash[0x20];
        u8 rodata_hash[0x20];
        u8 data_hash[0x20];

        static constexpr auto Magic = util::MakeMagic<u32>("NSO0");
    };
    static_assert(sizeof(NsoHeader) == 0x100);

}