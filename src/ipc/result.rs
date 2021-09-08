pub const RESULT_MODULE: u32 = 11;

result_define_group!(RESULT_MODULE => {
    UnsupportedOperation: 1,
    // Range(OutOfResource: 100, 299)
    OutOfSessionMemory: 102,
    // Range (OutOfSessions: 131, 139)
    PointerBufferTooSmall: 141,

    OutOfDomains: 200,

    // Range(CommunicationError: 300, 349)
    SessionClosed: 301,

    InvalidRequestSize: 402,
    UnknownCommandType: 403,

    InvalidCmifRequest: 420,

    TargetNotDomain: 491,
    DomainObjectNotFound: 492
});