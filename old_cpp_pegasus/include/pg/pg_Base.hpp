
#pragma once
#include <cstdint>
#include <cstring>
#include <string>

namespace pg {

    using u8 = uint8_t;
    using u16 = uint16_t;
    using u32 = uint32_t;
    using u64 = uint64_t;
    using i8 = int8_t;
    using i16 = int16_t;
    using i32 = int32_t;
    using i64 = int64_t;

}

#define PG_DEFINE_FLAG_ENUM(enum_type, base_type) \
inline constexpr enum_type operator|(const enum_type lhs, const enum_type rhs) { \
    return static_cast<const enum_type>(static_cast<const base_type>(lhs) | static_cast<const base_type>(rhs)); \
} \
inline constexpr enum_type operator&(const enum_type lhs, const enum_type rhs) { \
    return static_cast<const enum_type>(static_cast<const base_type>(lhs) & static_cast<const base_type>(rhs)); \
}

#define PG_ASSERT(cond) ({ \
    if(!(cond)) { \
        throw std::runtime_error("Assertion failed: '" #cond "'"); \
    } \
})