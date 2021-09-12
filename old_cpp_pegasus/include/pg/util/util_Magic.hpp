
#pragma once
#include <pg/pg_Base.hpp>

namespace pg::util {

    template<typename T, u64 N>
    consteval T MakeMagic(const char (&str)[N]) {
        static_assert(sizeof(T) == N - 1);

        auto magic = T();
        for(T i = 0; i < sizeof(T); i++) {
            magic |= static_cast<T>(str[i]) << i * 8;
        }
        return magic;
    }

}