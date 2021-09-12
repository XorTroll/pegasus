#include <pg/kern/svc/svc_Debug.hpp>

namespace pg::kern::svc {

    Result OutputDebugString(const char *str, size_t str_len) {
        printf("[OutputDebugString] %.*s\n", static_cast<int>(str_len), str);

        // Always succeed.
        return ResultSuccess;
    }

}