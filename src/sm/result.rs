pub const RESULT_MODULE: u32 = 21;

result_define_group!(RESULT_MODULE => {
    OutOfProcesses: 1,
    InvalidClient: 2,
    OutOfSessions: 3,
    AlreadyRegistered: 4,
    OutOfServices: 5,
    InvalidServiceName: 6,
    NotRegistered: 7,
    NotAllowed: 8,
    TooLargeAccessControl: 9
});