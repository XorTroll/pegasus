pub const RESULT_MODULE: u32 = 10;

result_define_group!(RESULT_MODULE => {
    InvalidHeaderSize: 202,
    InvalidInHeader: 211,
    UnknownCommandId: 221,
    InvalidOutRawSize: 232,
    InvalidNumInObjects: 235,
    InvalidNumOutObjects:236,
    InvalidInObject: 239,

    TargetNotFound: 261,

    OutOfDomainEntries: 301
});