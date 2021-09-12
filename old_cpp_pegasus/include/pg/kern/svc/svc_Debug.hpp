
#pragma once
#include <pg/kern/kern_Base.hpp>

namespace pg::kern::svc {

    Result OutputDebugString(const char *str, size_t str_len);

}