use unicorn::unicorn_const::uc_error;
use core::result::Result as CoreResult;
use crate::result::*;

pub const RESULT_MODULE: u32 = 505;

pub const UNICORN_ERROR_BASE: u32 = 1000;

result_define_group!(RESULT_MODULE => {
    UnicornOutOfMemory: UNICORN_ERROR_BASE + 1,
    UnicornUnsupportedArch: UNICORN_ERROR_BASE + 2,
    UnicornInvalidHandle: UNICORN_ERROR_BASE + 3,
    UnicornInvalidMode: UNICORN_ERROR_BASE + 4,
    UnicornUnsupportedVersion: UNICORN_ERROR_BASE + 5,
    UnicornReadUnmappedMemory: UNICORN_ERROR_BASE + 6,
    UnicornWriteUnmappedMemory: UNICORN_ERROR_BASE + 7,
    UnicornFetchUnmappedMemory: UNICORN_ERROR_BASE + 8,
    UnicornInvalidHookType: UNICORN_ERROR_BASE + 9,
    UnicornInvalidInstruction: UNICORN_ERROR_BASE + 10,
    UnicornInvalidMemoryMapping: UNICORN_ERROR_BASE + 11,
    UnicornWriteProtectedMemory: UNICORN_ERROR_BASE + 12,
    UnicornReadProtectedMemory: UNICORN_ERROR_BASE + 13,
    UnicornFetchProtectedMemory: UNICORN_ERROR_BASE + 14,
    UnicornInvalidArgument: UNICORN_ERROR_BASE + 15,
    UnicornReadUnaligned: UNICORN_ERROR_BASE + 16,
    UnicornWriteUnaligned: UNICORN_ERROR_BASE + 17,
    UnicornFetchUnaligned: UNICORN_ERROR_BASE + 18,
    UnicornHookAlreadyExists: UNICORN_ERROR_BASE + 19,
    UnicornInsufficientResource: UNICORN_ERROR_BASE + 20,
    UnicornCpuException: UNICORN_ERROR_BASE + 21
});

pub fn convert_unicorn_error<T>(r: CoreResult<T, uc_error>) -> Result<T> {
    r.map_err(|err| match err {
        uc_error::NOMEM => ResultUnicornOutOfMemory::make(),
        uc_error::ARCH => ResultUnicornUnsupportedArch::make(),
        uc_error::HANDLE => ResultUnicornInvalidHandle::make(),
        uc_error::MODE => ResultUnicornInvalidMode::make(),
        uc_error::VERSION => ResultUnicornUnsupportedVersion::make(),
        uc_error::READ_UNMAPPED => ResultUnicornReadUnmappedMemory::make(),
        uc_error::WRITE_UNMAPPED => ResultUnicornWriteUnmappedMemory::make(),
        uc_error::FETCH_UNMAPPED => ResultUnicornFetchUnmappedMemory::make(),
        uc_error::HOOK => ResultUnicornInvalidHookType::make(),
        uc_error::INSN_INVALID => ResultUnicornInvalidInstruction::make(),
        uc_error::MAP => ResultUnicornInvalidMemoryMapping::make(),
        uc_error::WRITE_PROT => ResultUnicornWriteProtectedMemory::make(),
        uc_error::READ_PROT => ResultUnicornReadProtectedMemory::make(),
        uc_error::FETCH_PROT => ResultUnicornFetchProtectedMemory::make(),
        uc_error::ARG => ResultUnicornInvalidArgument::make(),
        uc_error::READ_UNALIGNED => ResultUnicornReadUnaligned::make(),
        uc_error::WRITE_UNALIGNED => ResultUnicornWriteUnaligned::make(),
        uc_error::FETCH_UNALIGNED => ResultUnicornFetchUnaligned::make(),
        uc_error::HOOK_EXIST => ResultUnicornHookAlreadyExists::make(),
        uc_error::RESOURCE => ResultUnicornInsufficientResource::make(),
        uc_error::EXCEPTION => ResultUnicornCpuException::make(),
        _ => panic!("Invalid uc_error value")
    })
}