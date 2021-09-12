
#pragma once
#include <pg/pg_Base.hpp>

namespace pg::emu::util {

    template<typename T, typename ...Ts> requires std::is_base_of_v<std::exception, T>
    __attribute__((noreturn))
    void ThrowExceptionFormatted(const char *fmt, Ts ...ts) {
        char fmt_str[0x400] = {};
        std::sprintf(fmt_str, fmt, ts...);
        throw T(fmt_str);
    }

}