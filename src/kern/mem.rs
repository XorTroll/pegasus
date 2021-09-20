use super::svc;

// KMemoryBlock

bit_enum! {
    KMemoryState(u32) {
        None = 0,
        StateMask = 0xFF,
        All = u32::MAX,

        CanReprotect = bit!(8),
        CanDebug = bit!(9),
        CanUseIpc = bit!(10),
        CanUseNonDeviceIpc = bit!(11),
        CanUseNonSecureIpc = bit!(12),
        Mapped = bit!(13),
        CodeFlag = bit!(14),
        CanAlias = bit!(15),
        CanCodeAlias = bit!(16),
        CanTransfer = bit!(17),
        CanQueryPhysical = bit!(18),
        CanDeviceMap = bit!(19),
        CanAlignedDeviceMap = bit!(20),
        CanIpcUserBuffer = bit!(21),
        ReferenceCounted = bit!(22),
        CanMapProcess = bit!(23),
        CanChangeAttribute = bit!(24),
        CanCodeMemory = bit!(25),

        FlagsData = bit_group!(Self [CanReprotect, CanUseIpc, CanUseNonDeviceIpc, CanUseNonSecureIpc, Mapped, CanAlias, CanTransfer, CanQueryPhysical, CanDeviceMap, CanAlignedDeviceMap, CanIpcUserBuffer, ReferenceCounted, CanChangeAttribute]).get(),
        FlagsCode = bit_group!(Self [CanDebug, CanUseIpc, CanUseNonDeviceIpc, CanUseNonSecureIpc, Mapped, CodeFlag, CanQueryPhysical, CanDeviceMap, CanAlignedDeviceMap, ReferenceCounted]).get(),
        FlagsMisc = bit_group!(Self [Mapped, ReferenceCounted, CanQueryPhysical, CanDeviceMap]).get(),

        Free = svc::MemoryState::Free as u32,
        Io = svc::MemoryState::Io as u32 | bit_group!(Self [Mapped]).get(),
        Static = svc::MemoryState::Static as u32 | bit_group!(Self [Mapped, CanQueryPhysical]).get(),
        Code = svc::MemoryState::Code as u32 | bit_group!(Self [FlagsCode, CanMapProcess]).get(),
        CodeData = svc::MemoryState::CodeData as u32 | bit_group!(Self [FlagsData, CanMapProcess, CanCodeMemory]).get(),
        Normal = svc::MemoryState::Normal as u32 | bit_group!(Self [FlagsData, CanCodeMemory]).get(),
        Shared = svc::MemoryState::Shared as u32 | bit_group!(Self [Mapped, ReferenceCounted]).get()
    }
}

pub const fn convert_memory_state(state: KMemoryState) -> svc::MemoryState {
    unsafe {
        std::mem::transmute(state & KMemoryState::StateMask())
    }
}

pub struct KMemoryBlock {

}

// KMemoryBlockSlabManager

pub struct KMemoryBlockSlabManager {
    capacity: usize,
    pub count: usize
}

impl KMemoryBlockSlabManager {
    pub const fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity,
            count: 0
        }
    }

    pub const fn can_allocate(&self, count: usize) -> bool {
        (self.count + count) <= self.capacity
    }
}

// ---

// KMemoryBlockManager

pub struct KMemoryBlockManager {

}

impl KMemoryBlockManager {
    pub fn new(address_space_start: u64, address_space_end: u64, slab_manager: u8) {

    }
}

// ---