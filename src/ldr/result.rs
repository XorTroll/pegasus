pub const RESULT_MODULE: u32 = 9;

result_define_group!(RESULT_MODULE => {
    TooLongArgument: 1,
    TooManyArguments: 2,
    TooLargeMeta: 3,
    InvalidMeta: 4,
    InvalidNso: 5,
    InvalidPath: 6,
    TooManyProcesses: 7,
    NotPinned: 8,
    InvalidProgramId: 9,
    InvalidVersion: 10,
    InvalidAcidSignature: 11,
    InvalidNcaSignature: 12,

    InsufficientAddressSpace: 51,
    InvalidNro: 52,
    InvalidNrr: 53,
    InvalidSignature: 54,
    InsufficientNroRegistrations: 55,
    InsufficientNrrRegistrations: 56,
    NroAlreadyLoaded: 57,

    InvalidAddress: 81,
    InvalidSize: 82,
    NotLoaded: 84,
    NotRegistered: 85,
    InvalidSession: 86,
    InvalidProcess: 87,

    UnknownCapability: 100,
    InvalidCapabilityKernelFlags: 103,
    InvalidCapabilitySyscallMask: 104,
    InvalidCapabilityMapRange: 106,
    InvalidCapabilityMapPage: 107,
    InvalidCapabilityMapRegion: 110,
    InvalidCapabilityInterruptPair: 111,
    InvalidCapabilityProgramType: 113,
    InvalidCapabilityKernelVersion: 114,
    InvalidCapabilityHandleTable: 115,
    InvalidCapabilityDebugFlags: 116,

    InternalError: 200
});