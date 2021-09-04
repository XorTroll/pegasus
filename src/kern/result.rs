use crate::result::*;

pub const RESULT_MODULE: u32 = 1;

result_define_group!(RESULT_MODULE => {
    OutOfSessions: 7,

    InvalidArgument: 14,

    NotImplemented: 33,

    StopProcessingException: 54,

    NoSynchronizationObject: 57,

    TerminationRequested: 59,

    NoEvent: 70,

    InvalidSize: 101,
    InvalidAddress: 102,
    OutOfResource: 103,
    OutOfMemory: 104,
    OutOfHandles: 105,
    InvalidCurrentMemory: 106,

    InvalidNewMemoryPermission: 108,

    InvalidMemoryRegion: 110,

    InvalidPriority: 112,
    InvalidCoreId: 113,
    InvalidHandle: 114,
    InvalidPointer: 115,
    InvalidCombination: 116,
    TimedOut: 117,
    Cancelled: 118,
    OutOfRange: 119,
    InvalidEnumValue: 120,
    NotFound: 121,
    Busy: 122,
    SessionClosed: 123,
    NotHandled: 124,
    InvalidState: 125,
    ReservedUsed: 126,
    NotSupported: 127,
    Debug: 128,
    NoThread: 129,
    UnknownThread: 130,
    PortClosed: 131,
    LimitReached: 132,
    InvalidMemoryPool: 133,

    ReceiveListBroken: 258,
    OutOfAddressSpace: 259,
    MessageTooLarge: 260,

    InvalidProcessId: 517,
    InvalidThreadId: 518,
    InvalidId: 519,
    ProcessTerminated: 520
});