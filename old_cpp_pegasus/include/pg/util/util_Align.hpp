
#pragma once
#include <pg/pg_Base.hpp>

namespace pg::util {

    template<typename T>
    inline constexpr T AlignUp(const T value, const size_t size) {
        const auto mask = size - 1;
        return (value + mask) & ~mask;
    }

}