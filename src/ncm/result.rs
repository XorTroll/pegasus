pub const RESULT_MODULE: u32 = 5;

result_define_group!(RESULT_MODULE => {
    InvalidContentStorageBase: 1,
    PlaceHolderAlreadyExists: 2,
    PlaceHolderNotFound: 3,
    ContentAlreadyExists: 4,
    ContentNotFound: 5,
    ContentMetaNotFound: 7,
    AllocationFailed: 8,
    UnknownStorage: 12,

    InvalidContentStorage: 100,
    InvalidContentMetaDatabase: 110,
    InvalidPackageFormat: 130,
    InvalidContentHash: 140,

    InvalidInstallTaskState: 160,
    InvalidPlaceHolderFile: 170,
    BufferInsufficient: 180,
    NotSupported: 190,
    NotEnoughInstallSpace: 200,
    SystemUpdateNotFoundInPackage: 210,
    ContentInfoNotFound: 220,
    DeltaNotFound: 237,
    InvalidContentMetaKey: 240,
    IgnorableInstallTicketFailure: 280,

    ContentStorageBaseNotFound: 310,
    ListPartiallyNotCommitted: 330,
    UnexpectedContentMetaPrepared: 360,
    InvalidFirmwareVariation: 380,

    // Range(ContentStorageNotActive: 250, 258)
    GameCardContentStorageNotActive: 251,
    BuiltInSystemContentStorageNotActive: 252,
    BuiltInUserContentStorageNotActive: 253,
    SdCardContentStorageNotActive: 254,
    UnknownContentStorageNotActive: 258,

    // Range(ContentMetaDatabaseNotActive: 260, 268)
    GameCardContentMetaDatabaseNotActive: 261,
    BuiltInSystemContentMetaDatabaseNotActive: 262,
    BuiltInUserContentMetaDatabaseNotActive: 263,
    SdCardContentMetaDatabaseNotActive: 264,
    UnknownContentMetaDatabaseNotActive: 268,

    // Range(InstallTaskCancelled: 290, 299)
    CreatePlaceHolderCancelled: 291,
    WritePlaceHolderCancelled: 292,

    InvalidOperation: 8180,
    // Range(InvalidArgument: 8181, 8191)
    InvalidOffset: 8182
});