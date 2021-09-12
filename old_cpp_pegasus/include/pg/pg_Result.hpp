
#pragma once
#include <pg/pg_Base.hpp>
#include <stdexcept>

namespace pg {

    struct Result {
        u32 value;

        static constexpr u32 ModuleBits = 9;
        static constexpr u32 DescriptionBits = 13;
        static constexpr u32 ReservedBits = 10;
        static constexpr u32 DefaultValue = u32();
        static constexpr u32 SuccessValue = DefaultValue;

        static inline constexpr u32 Pack(u32 mod, u32 desc) {
            return mod | (desc << ModuleBits);
        }

        static inline constexpr u32 UnpackModule(u32 value) {
            return value & ~(~DefaultValue << ModuleBits);
        }

        static inline constexpr u32 UnpackDescription(u32 value) {
            return (value >> ModuleBits) & ~(~DefaultValue << DescriptionBits);
        }

        constexpr Result() : value(SuccessValue) {}
        constexpr Result(u32 value) : value(value) {}
        constexpr Result(u32 mod, u32 desc) : value(Pack(mod, desc)) {}

        inline constexpr bool IsSuccess() const {
            return this->value == SuccessValue;
        }

        inline constexpr bool IsFailure() const {
            return !this->IsSuccess();
        }

        inline constexpr u32 GetModule() const {
            return UnpackModule(this->value);
        }

        inline constexpr u32 GetDescription() const {
            return UnpackDescription(this->value);
        }

        inline constexpr u32 GetValue() const {
            return this->value;
        }
        
        inline constexpr operator u32() const {
            return this->GetValue();
        }

    };
    static_assert(sizeof(Result) == 4);

    constexpr Result ResultSuccess = Result();

    class ResultError : virtual public std::runtime_error {
        private:
            Result rc;

        public:
            explicit ResultError(Result rc, const std::string &msg) noexcept : std::runtime_error(msg), rc(rc) {}
            virtual ~ResultError() throw() {}
    };

}

#define PG_RESULT_FMT_STR "%04d-%04d"

#define PG_RESULT_TRY(...) ({ \
    const auto _tmp_rc = static_cast<::pg::Result>(__VA_ARGS__); \
    if(_tmp_rc.IsFailure()) { \
        return _tmp_rc; \
    } \
})

#define PG_RESULT_ASSERT(...) ({ \
    const auto _tmp_rc = static_cast<::pg::Result>(__VA_ARGS__); \
    if(_tmp_rc.IsFailure()) { \
        char msg_str[0x100] = {}; \
        sprintf(msg_str, "Expression '"#__VA_ARGS__ "' failed with result " PG_RESULT_FMT_STR, 2000 + _tmp_rc.GetModule(), _tmp_rc.GetDescription()); \
        throw ::pg::ResultError(_tmp_rc, msg_str); \
    } \
})

#define PG_RETURN_IF(cond, ...) ({ \
    if(cond) { \
        return (__VA_ARGS__); \
    } \
})

#define PG_RETURN_UNLESS(cond, ...) PG_RETURN_IF(!(cond), ##__VA_ARGS__)

#define PG_RESULT_NAMESPACE_DEFINE_MODULE(val) constexpr u32 Module = val

#define PG_RESULT_NAMESPACE_DEFINE(name, val) constexpr Result Result ## name(Module, val)