use super::svc;

pub const PAGE_SIZE: usize = 0x1000;

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
        Shared = svc::MemoryState::Shared as u32 | bit_group!(Self [Mapped, ReferenceCounted]).get(),

        AliasCode = svc::MemoryState::AliasCode as u32 | bit_group!(Self [FlagsCode, CanMapProcess, CanCodeAlias]).get(),
        AliasCodeData = svc::MemoryState::AliasCode as u32 | bit_group!(Self [FlagsData, CanMapProcess, CanCodeAlias, CanCodeMemory]).get(),

        Ipc = svc::MemoryState::Ipc as u32 | bit_group!(Self [FlagsMisc, CanAlignedDeviceMap, CanUseIpc, CanUseNonSecureIpc, CanUseNonDeviceIpc]).get(),
        
        Stack = svc::MemoryState::Stack as u32 | bit_group!(Self [FlagsMisc, CanAlignedDeviceMap, CanUseIpc, CanUseNonSecureIpc, CanUseNonDeviceIpc]).get(),
        
        ThreadLocal = svc::MemoryState::ThreadLocal as u32 | bit_group!(Self [Mapped, ReferenceCounted]).get(),
        
        Transfered = svc::MemoryState::Transfered as u32 | bit_group!(Self [FlagsMisc, CanAlignedDeviceMap, CanChangeAttribute, CanUseIpc, CanUseNonSecureIpc, CanUseNonDeviceIpc]).get(),
        
        SharedTransfered = svc::MemoryState::SharedTransfered as u32 | bit_group!(Self [FlagsMisc, CanAlignedDeviceMap, CanUseNonSecureIpc, CanUseNonDeviceIpc]).get(),
        
        SharedCode = svc::MemoryState::SharedCode as u32 | bit_group!(Self [Mapped, ReferenceCounted, CanUseNonSecureIpc, CanUseNonDeviceIpc]).get(),

        Inaccessible = svc::MemoryState::Inaccessible as u32,

        NonSecureIpc = svc::MemoryState::NonSecureIpc as u32 | bit_group!(Self [FlagsMisc, CanAlignedDeviceMap, CanUseNonSecureIpc, CanUseNonDeviceIpc]).get(),

        NonDeviceIpc = svc::MemoryState::NonDeviceIpc as u32 | bit_group!(Self [FlagsMisc, CanUseNonDeviceIpc]).get(),

        Kernel = svc::MemoryState::Kernel as u32 | bit_group!(Self [Mapped]).get(),

        GeneratedCode = svc::MemoryState::GeneratedCode as u32 | bit_group!(Self [Mapped, ReferenceCounted, CanDebug]).get(),

        CodeOut = svc::MemoryState::CodeOut as u32 | bit_group!(Self [Mapped, ReferenceCounted]).get(),

        Coverage = svc::MemoryState::Coverage as u32 | bit_group!(Self [Mapped]).get()
    }
}

pub const fn convert_memory_state(state: KMemoryState) -> svc::MemoryState {
    unsafe {
        std::mem::transmute(state & KMemoryState::StateMask())
    }
}

bit_enum! {
    KMemoryPermission(u8) {
        None = 0,
        All = u8::MAX,

        KernelShift = 3,

        KernelRead = (svc::MemoryPermission::Read().get() as u8) << Self::KernelShift().get(),
        KernelWrite = (svc::MemoryPermission::Write().get() as u8) << Self::KernelShift().get(),
        KernelExecute = (svc::MemoryPermission::Execute().get() as u8) << Self::KernelShift().get(),

        NotMapped = bit!(2 * Self::KernelShift().get()),

        KernelReadWrite = bit_group!(Self [KernelRead, KernelWrite]).get(),
        KernelReadExecute = bit_group!(Self [KernelRead, KernelExecute]).get(),

        UserRead = svc::MemoryPermission::Read().get() as u8 | bit_group!(Self [KernelRead]).get(),
        UserWrite = svc::MemoryPermission::Write().get() as u8 | bit_group!(Self [KernelWrite]).get(),
        UserExecute = svc::MemoryPermission::Execute().get() as u8,

        UserReadWrite = bit_group!(Self [UserRead, UserWrite]).get(),
        UserReadExecute = bit_group!(Self [UserRead, UserExecute]).get(),

        UserMask = svc::MemoryPermission::Read().get() as u8 | svc::MemoryPermission::Write().get() as u8 | svc::MemoryPermission::Execute().get() as u8,

        IpcLockChangeMask = bit_group!(Self [NotMapped, UserReadWrite]).get()
    }
}

pub const fn convert_memory_permission(perm: KMemoryPermission) -> svc::MemoryPermission {
    unsafe {
        std::mem::transmute((perm & KMemoryPermission::UserMask()).get() as u32)
    }
}

bit_enum! {
    KMemoryAttribute(u8) {
        None = 0,
        All = u8::MAX,
        UserMask = Self::All().get(),

        Locked = svc::MemoryAttribute::Locked().get() as u8,
        IpcLocked = svc::MemoryAttribute::IpcLocked().get() as u8,
        DeviceShared = svc::MemoryAttribute::DeviceShared().get() as u8,
        Uncached = svc::MemoryAttribute::Uncached().get() as u8,

        SetMask = Self::Uncached().get()
    }
}

pub const fn convert_memory_attribute(attr: KMemoryAttribute) -> svc::MemoryAttribute {
    unsafe {
        std::mem::transmute((attr & KMemoryAttribute::UserMask()).get() as u32)
    }
}

pub struct KMemoryInfo {
    pub addr: u64,
    pub size: usize,
    pub state: KMemoryState,
    pub perm: KMemoryPermission,
    pub attr: KMemoryAttribute,
    pub src_perm: KMemoryPermission,
    pub ipc_refcount: u32,
    pub device_refcount: u32
}

impl KMemoryInfo {
    pub fn convert_info(&self) -> svc::MemoryInfo {
        svc::MemoryInfo {
            base_address: self.addr,
            size: self.size,
            state: convert_memory_state(self.state),
            attr: convert_memory_attribute(self.attr),
            perm: convert_memory_permission(self.perm),
            ipc_refcount: self.ipc_refcount,
            device_refcount: self.device_refcount,
            pad: 0
        }
    }
}

pub struct KMemoryBlock {
    pub base_addr: u64,
    pub page_count: usize,
    pub state: KMemoryState,
    pub perm: KMemoryPermission,
    pub attr: KMemoryAttribute,
    pub src_perm: KMemoryPermission,
    pub ipc_refcount: u32,
    pub device_refcount: u32
}

impl KMemoryBlock {
    pub fn set_state(&mut self, perm: KMemoryPermission, state: KMemoryState, attr: KMemoryAttribute) {
        self.perm = perm;
        self.state = state;
        self.attr &= KMemoryAttribute::IpcLocked() | KMemoryAttribute::DeviceShared();
        self.attr |= attr;
    }

    pub fn set_ipc_mapping_permission(&mut self, perm: KMemoryPermission) {
        let old_ipc_refcount = self.ipc_refcount;
        self.ipc_refcount += 1;

        if old_ipc_refcount == 0 {
            self.src_perm = self.perm;

            self.perm &= !KMemoryPermission::UserReadWrite();
            self.perm |= KMemoryPermission::UserReadWrite() & perm;
        }

        self.attr |= KMemoryAttribute::IpcLocked();
    }

    pub fn restore_ipc_mapping_permission(&mut self) {
        self.ipc_refcount -= 1;

        if self.ipc_refcount == 0 {
            self.perm = self.src_perm;

            self.src_perm = KMemoryPermission::None();

            self.attr &= !KMemoryAttribute::IpcLocked();
        }
    }

    pub fn split_right_at_address(&mut self, addr: u64) -> KMemoryBlock {
        let left_addr = self.base_addr;
        let left_page_count = (addr - left_addr) as usize / PAGE_SIZE;

        self.base_addr = addr;

        KMemoryBlock {
            base_addr: left_addr,
            page_count: left_page_count,
            state: self.state,
            perm: self.perm,
            attr: self.attr,
            src_perm: KMemoryPermission::None(),
            ipc_refcount: self.ipc_refcount,
            device_refcount: self.device_refcount
        }
    }

    pub fn add_pages(&mut self, page_count: usize) {
        self.page_count += page_count;
    }

    pub fn get_info(&self) -> KMemoryInfo {
        KMemoryInfo {
            addr: self.base_addr,
            size: self.page_count * PAGE_SIZE,
            state: self.state,
            perm: self.perm,
            attr: self.attr,
            src_perm: self.src_perm,
            ipc_refcount: self.ipc_refcount,
            device_refcount: self.device_refcount
        }
    }
}

// --

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

// ---

// KPageTable



// ---