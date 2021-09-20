use std::mem;
use crate::kern::svc;
use crate::util;
use crate::result::*;

use super::result;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum AddressSpaceType {
    AS32Bit = 0,
    AS64BitLegacy = 1,
    AS32BitNoReserved = 2,
    AS64Bit = 3
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct MetaFlags {
    pub bits: u8
}

impl MetaFlags {
    pub const fn is_64bit(&self) -> bool {
        read_bits!(0, 0, self.bits) != 0
    }

    pub const fn set_64bit(&mut self, is_64bit: bool) {
        write_bits!(0, 0, self.bits, is_64bit as u8);
    }

    pub const fn get_address_space(&self) -> AddressSpaceType {
        unsafe {
            mem::transmute(read_bits!(1, 3, self.bits) as u8)
        }
    }

    pub const fn set_address_space(&mut self, addr_space: AddressSpaceType) {
        unsafe {
            write_bits!(1, 3, self.bits, mem::transmute::<AddressSpaceType, u8>(addr_space));
        }
    }

    pub const fn optimize_memory_allocation(&self) -> bool {
        read_bits!(4, 4, self.bits) != 0
    }

    pub const fn set_optimize_memory_allocation(&mut self, optimize_memory_allocation: bool) {
        write_bits!(4, 4, self.bits, optimize_memory_allocation as u8);
    }

    pub const fn disable_device_address_space_merge(&self) -> bool {
        read_bits!(5, 5, self.bits) != 0
    }

    pub const fn set_disable_device_address_space_merge(&mut self, disable_device_address_space_merge: bool) {
        write_bits!(5, 5, self.bits, disable_device_address_space_merge as u8);
    }

    pub const fn new(is_64bit: bool, addr_space: AddressSpaceType, optimize_memory_allocation: bool, disable_device_address_space_merge: bool) -> Self {
        let mut flags = Self {
            bits: 0
        };
        flags.set_64bit(is_64bit);
        flags.set_address_space(addr_space);
        flags.set_optimize_memory_allocation(optimize_memory_allocation);
        flags.set_disable_device_address_space_merge(disable_device_address_space_merge);

        flags
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Meta {
    pub magic: u32,
    pub acid_signature_key_generation: u32,
    pub reserved_1: [u8; 0x4],
    pub flags: MetaFlags,
    pub reserved_2: u8,
    pub main_thread_priority: u8,
    pub main_thread_cpu_core: u8,
    pub reserved_3: [u8; 0x4],
    pub system_resource_size: u32,
    pub version: u32,
    pub main_thread_stack_size: u32,
    pub name: util::CString<0x10>,
    pub product_code: util::CString<0x10>,
    pub reserved_4: [u8; 0x30],
    pub aci0_offset: u32,
    pub aci0_size: u32,
    pub acid_offset: u32,
    pub acid_size: u32
}

impl Meta {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"META");
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Aci0 {
    pub magic: u32,
    pub reserved_1: [u8; 0xC],
    pub program_id: u64,
    pub reserved_2: [u8; 0x8],
    pub fs_access_control_offset: u32,
    pub fs_access_control_size: u32,
    pub service_access_control_offset: u32,
    pub service_access_control_size: u32,
    pub kernel_capability_offset: u32,
    pub kernel_capability_size: u32,
    pub reserved_3: [u8; 0x8]
}

impl Aci0 {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"ACI0");
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum MemoryRegion {
    Application = 0,
    Applet = 1,
    SecureSystem = 2,
    NonSecureSystem = 3
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct AcidFlags {
    pub bits: u32
}

impl AcidFlags {
    pub const fn has_production_flag(&self) -> bool {
        read_bits!(0, 0, self.bits) != 0
    }

    pub const fn set_production_flag(&mut self, production_flag: bool) {
        write_bits!(0, 0, self.bits, production_flag as u32);
    }

    pub const fn has_unqualified_approval(&self) -> bool {
        read_bits!(1, 1, self.bits) != 0
    }

    pub const fn set_unqualified_approval(&mut self, unqualified_approval: bool) {
        write_bits!(1, 1, self.bits, unqualified_approval as u32);
    }

    pub const fn new(production_flag: bool, unqualified_approval: bool) -> Self {
        let mut flags = Self {
            bits: 0
        };
        flags.set_production_flag(production_flag);
        flags.set_unqualified_approval(unqualified_approval);
        flags
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Acid {
    pub rsa_signature: [u8; 0x100],
    pub rsa_nca_sig_public_key: [u8; 0x100],
    pub magic: u32,
    pub size: u32,
    pub reserved_1: [u8; 0x4],
    pub flags: AcidFlags,
    pub program_id_min: u64,
    pub program_id_max: u64,
    pub fs_access_control_offset: u32,
    pub fs_access_control_size: u32,
    pub service_access_control_offset: u32,
    pub service_access_control_size: u32,
    pub kernel_capability_offset: u32,
    pub kernel_capability_size: u32,
    pub reserved_2: [u8; 0x8]
}

impl Acid {
    pub const MAGIC: u32 = u32::from_le_bytes(*b"ACID");
}

bit_enum! {
    FsAccessFlag (u64) {
        ApplicationInfo = bit!(0),
        BootModeControl = bit!(1),
        Calibration = bit!(2),
        SystemSaveData = bit!(3),
        GameCard = bit!(4),
        SaveDataBackup = bit!(5),
        SaveDataManagement = bit!(6),
        BisAllRaw = bit!(7),
        GameCardRaw = bit!(8),
        GameCardPrivate = bit!(9),
        SetTime = bit!(10),
        ContentManager = bit!(11),
        ImageManager = bit!(12),
        CreateSaveData = bit!(13),
        SystemSaveDataManagement = bit!(14),
        BisFileSystem = bit!(15),
        SystemUpdate = bit!(16),
        SaveDataMeta = bit!(17),
        DeviceSaveData = bit!(18),
        SettingsControl = bit!(19),
        SystemData = bit!(20),
        SdCard = bit!(21),
        Host = bit!(22),
        FillBis = bit!(23),
        CorruptSaveData = bit!(24),
        SaveDataForDebug = bit!(25),
        FormatSdCard = bit!(26),
        GetRightsId = bit!(27),
        RegisterExternalKey = bit!(28),
        RegisterUpdatePartition = bit!(29),
        SaveDataTransfer = bit!(30),
        DeviceDetection = bit!(31),
        AccessFailureResolution = bit!(32),
        SaveDataTransferV2 = bit!(33),
        RegisterProgramIndexMapInfo = bit!(34),
        CreateOwnSaveData = bit!(35),
        MoveCacheStorage = bit!(36),
        Debug = bit!(62),
        FullPermission = u64::MAX
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum Accessibility {
    Read = 1,
    Write = 2,
    ReadWrite = 3
}

#[derive(Debug)]
pub struct Aci0FsAccessControlData {
    pub version: u8,
    pub flags: FsAccessFlag,
    pub content_owner_info_offset: u32,
    pub content_owner_info_size: u32,
    pub content_owner_ids: Vec<u64>,
    pub save_data_owner_info_offset: u32,
    pub save_data_owner_info_size: u32,
    pub accessibilities: Vec<Accessibility>,
    pub save_data_owner_ids: Vec<u64>,
}

impl Aci0FsAccessControlData {
    pub fn new(fs_access_control: &[u8]) -> Result<Self> {
        let mut offset = 0usize;
        let version: u8 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        offset += 3; // Padding
        let flags: FsAccessFlag = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let content_owner_info_offset: u32 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let content_owner_info_size: u32 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let save_data_owner_info_offset: u32 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let save_data_owner_info_size: u32 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        
        let mut content_owner_ids: Vec<u64> = Vec::new();
        if content_owner_info_size > 0 {
            offset = content_owner_info_offset as usize;

            let content_owner_id_count: u32 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
            for _ in 0..content_owner_id_count {
                content_owner_ids.push(util::slice_read_val_advance(fs_access_control, &mut offset)?);
            }
        }

        let mut accesibilities: Vec<Accessibility> = Vec::new();
        let mut save_data_owner_ids: Vec<u64> = Vec::new();
        if save_data_owner_info_size > 0 {
            offset = save_data_owner_info_offset as usize;

            let save_data_owner_id_count: u32 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
            for _ in 0..save_data_owner_id_count {
                accesibilities.push(util::slice_read_val_advance(fs_access_control, &mut offset)?);
            }

            offset = util::align_up(offset, 4); // Aligned to 4 bytes

            for _ in 0..save_data_owner_id_count {
                save_data_owner_ids.push(util::slice_read_val_advance(fs_access_control, &mut offset)?);
            }
        }

        Ok(Self {
            version: version,
            flags: flags,
            content_owner_info_offset: content_owner_info_offset,
            content_owner_info_size: content_owner_info_size,
            content_owner_ids: content_owner_ids,
            save_data_owner_info_offset: save_data_owner_info_offset,
            save_data_owner_info_size: save_data_owner_info_size,
            accessibilities: accesibilities,
            save_data_owner_ids: save_data_owner_ids
        })
    }
}

#[derive(Debug)]
pub struct AcidFsAccessControlData {
    pub version: u8,
    pub flags: FsAccessFlag,
    pub content_owner_id_min: u64,
    pub content_owner_id_max: u64,
    pub content_owner_ids: Vec<u64>,
    pub save_data_owner_id_min: u64,
    pub save_data_owner_id_max: u64,
    pub save_data_owner_ids: Vec<u64>,
}

impl AcidFsAccessControlData {
    pub fn new(fs_access_control: &[u8]) -> Result<Self> {
        let mut offset = 0usize;
        let version: u8 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let content_owner_id_count: u8 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let save_data_owner_id_count: u8 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        offset += 1; // Padding
        let flags: FsAccessFlag = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let content_owner_id_min: u64 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let content_owner_id_max: u64 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let save_data_owner_id_min: u64 = util::slice_read_val_advance(fs_access_control, &mut offset)?;
        let save_data_owner_id_max: u64 = util::slice_read_val_advance(fs_access_control, &mut offset)?;

        let mut content_owner_ids: Vec<u64> = Vec::new();
        for _ in 0..content_owner_id_count {
            content_owner_ids.push(util::slice_read_val_advance(fs_access_control, &mut offset)?);
        }

        let mut save_data_owner_ids: Vec<u64> = Vec::new();
        for _ in 0..save_data_owner_id_count {
            save_data_owner_ids.push(util::slice_read_val_advance(fs_access_control, &mut offset)?);
        }

        Ok(Self {
            version: version,
            flags: flags,
            content_owner_id_min: content_owner_id_min,
            content_owner_id_max: content_owner_id_max,
            content_owner_ids: content_owner_ids,
            save_data_owner_id_min: save_data_owner_id_min,
            save_data_owner_id_max: save_data_owner_id_max,
            save_data_owner_ids: save_data_owner_ids
        })
    }
}

#[derive(Debug)]
pub struct ServiceAccessControlEntry {
    pub name: String,
    pub is_server: bool
}

impl ServiceAccessControlEntry {
    pub fn new(name: String, is_server: bool) -> Self {
        Self {
            name: name,
            is_server: is_server
        }
    }
}

#[derive(Debug)]
pub struct ServiceAccessControlData {
    pub services: Vec<ServiceAccessControlEntry>
}

impl ServiceAccessControlData {
    pub fn new(service_access_control: &[u8]) -> Result<Self> {
        let mut offset = 0usize;
        
        let mut services: Vec<ServiceAccessControlEntry> = Vec::new();
        while offset < service_access_control.len() {
            let info_byte: u8 = util::slice_read_val_advance(service_access_control, &mut offset)?;
            let service_name_len = read_bits!(0, 2, info_byte) as usize + 1;
            let is_server = read_bits!(7, 7, info_byte) != 0;
            
            let service_name_data = util::slice_read_data_advance(service_access_control, &mut offset, service_name_len)?;
            let service_name = String::from_utf8(service_name_data).unwrap();
            services.push(ServiceAccessControlEntry::new(service_name, is_server));
        }

        Ok(Self {
            services: services
        })
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct ThreadInfo {
    pub highest_priority: u8,
    pub lowest_priority: u8,
    pub min_core_number: u8,
    pub max_core_number: u8
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum PermissionType {
    ReadWrite = 0,
    ReadOnly = 1
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum MappingType {
    Io = 0,
    Static = 1
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct MemoryMap {
    pub address: u64,
    pub perm_type: PermissionType,
    pub size: usize,
    pub map_type: MappingType
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct IoMemoryMap {
    pub address: u64
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum RegionType {
    NoMapping = 0,
    KernelTraceBuffer = 1,
    OnMemoryBootImage = 2,
    DTB = 3
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct MemoryRegionMap {
    pub type_0: RegionType,
    pub is_read_only_0: bool,
    pub type_1: RegionType,
    pub is_read_only_1: bool,
    pub type_2: RegionType,
    pub is_read_only_2: bool
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct EnableInterrupts {
    pub intr_no_0: u8,
    pub intr_no_1: u8
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum ProgramType {
    System = 0,
    Application = 1,
    Applet = 2
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct MiscParams {
    pub program_type: ProgramType
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct KernelVersion {
    pub major: u8,
    pub minor: u8
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct MiscFlags {
    pub enable_debug: bool,
    pub force_debug: bool
}

#[derive(Debug)]
pub struct KernelCapabilityData {
    pub thread_info: Option<ThreadInfo>,
    pub enabled_svcs: Vec<svc::SvcId>,
    pub memory_maps: Vec<MemoryMap>,
    pub io_memory_maps: Vec<IoMemoryMap>,
    pub mem_region_maps: Vec<MemoryRegionMap>,
    pub enable_interrupts: Option<EnableInterrupts>,
    pub misc_params: Option<MiscParams>,
    pub kernel_version: Option<KernelVersion>,
    pub handle_table_size: Option<u16>,
    pub misc_flags: Option<MiscFlags>
}

const fn is_lowest_clear_bit(val: u32, bit: u8) -> bool {
    (val & (bit!(bit + 1) - 1)) == (bit!(bit) - 1)
}

impl KernelCapabilityData {
    pub fn new(kernel_capabilities: &[u8]) -> Result<Self> {
        let mut offset = 0usize;

        let mut capability_data = Self {
            thread_info: None,
            enabled_svcs: Vec::new(),
            memory_maps: Vec::new(),
            io_memory_maps: Vec::new(),
            mem_region_maps: Vec::new(),
            enable_interrupts: None,
            misc_params: None,
            kernel_version: None,
            handle_table_size: None,
            misc_flags: None
        };

        while offset < kernel_capabilities.len() {
            let val_1: u32 = util::slice_read_val_advance(kernel_capabilities, &mut offset)?;

            if is_lowest_clear_bit(val_1, 3) {
                let highest_priority = read_bits!(4, 9, val_1) as u8;
                let lowest_priority = read_bits!(10, 15, val_1) as u8;
                let min_core_number = read_bits!(16, 23, val_1) as u8;
                let max_core_number = read_bits!(24, 31, val_1) as u8;
                
                capability_data.thread_info = Some(ThreadInfo {
                    highest_priority: highest_priority,
                    lowest_priority: lowest_priority,
                    min_core_number: min_core_number,
                    max_core_number: max_core_number
                });
            }
            else if is_lowest_clear_bit(val_1, 4) {
                let svc_mask = read_bits!(5, 28, val_1);
                let index = read_bits!(29, 31, val_1) as u8;
                for i in 0..24u8 {
                    if (svc_mask & bit!(i)) != 0 {
                        let raw_svc_id = i + index * 24;
                        if let Some(svc_id) = svc::SvcId::from(raw_svc_id) {
                            capability_data.enabled_svcs.push(svc_id);
                        }
                        else {
                            // TODO: ignore, error...? many homebrew NPDMs have SVC 0x0 (invalid one), for instance...
                            log_line!("(warning) Unsupported/invalid SVC: {:#X}", raw_svc_id);
                        }
                    }
                }
            }
            else if is_lowest_clear_bit(val_1, 6) {
                let val_2: u32 = util::slice_read_val_advance(kernel_capabilities, &mut offset)?;
                if is_lowest_clear_bit(val_2, 6) {
                    let address = read_bits!(7, 30, val_1) as u64;
                    let permission_type: PermissionType = unsafe {
                        mem::transmute(read_bits!(31, 31, val_1) as u8)
                    };
                    let size = read_bits!(7, 26, val_2) as usize;
                    // Bits 27-30 reserved
                    let mapping_type: MappingType = unsafe {
                        mem::transmute(read_bits!(31, 31, val_2) as u8)
                    };

                    capability_data.memory_maps.push(MemoryMap {
                        address: address,
                        perm_type: permission_type,
                        size: size,
                        map_type: mapping_type
                    });
                }
            }
            else if is_lowest_clear_bit(val_1, 7) {
                let address = read_bits!(8, 31, val_1) as u64; 

                capability_data.io_memory_maps.push(IoMemoryMap {
                    address: address
                });
            }
            else if is_lowest_clear_bit(val_1, 10) {
                let region_type_0: RegionType = unsafe {
                    mem::transmute(read_bits!(11, 16, val_1) as u8)
                };
                let is_read_only_0 = read_bits!(17, 17, val_1) != 0;
                let region_type_1: RegionType = unsafe {
                    mem::transmute(read_bits!(18, 23, val_1) as u8)
                };
                let is_read_only_1 = read_bits!(24, 24, val_1) != 0;
                let region_type_2: RegionType = unsafe {
                    mem::transmute(read_bits!(25, 30, val_1) as u8)
                };
                let is_read_only_2 = read_bits!(31, 31, val_1) != 0;

                capability_data.mem_region_maps.push(MemoryRegionMap {
                    type_0: region_type_0,
                    is_read_only_0: is_read_only_0,
                    type_1: region_type_1,
                    is_read_only_1: is_read_only_1,
                    type_2: region_type_2,
                    is_read_only_2: is_read_only_2
                });
            }
            else if is_lowest_clear_bit(val_1, 11) {
                let intr_no_0 = read_bits!(12, 21, val_1) as u8;
                let intr_no_1 = read_bits!(22, 31, val_1) as u8;

                capability_data.enable_interrupts = Some(EnableInterrupts {
                    intr_no_0: intr_no_0,
                    intr_no_1: intr_no_1
                });
            }
            else if is_lowest_clear_bit(val_1, 13) {
                let program_type: ProgramType = unsafe {
                    mem::transmute(read_bits!(14, 16, val_1) as u8)
                };

                capability_data.misc_params = Some(MiscParams {
                    program_type: program_type
                });
            }
            else if is_lowest_clear_bit(val_1, 14) {
                let kernel_ver_minor = read_bits!(15, 18, val_1) as u8;
                let kernel_ver_major = read_bits!(19, 31, val_1) as u8;

                capability_data.kernel_version = Some(KernelVersion {
                    major: kernel_ver_major,
                    minor: kernel_ver_minor
                });
            }
            else if is_lowest_clear_bit(val_1, 15) {
                let handle_table_size = read_bits!(16, 25, val_1) as u16;

                capability_data.handle_table_size = Some(handle_table_size);
            }
            else if is_lowest_clear_bit(val_1, 16) {
                let enable_debug = read_bits!(17, 17, val_1) != 0;
                let force_debug = read_bits!(18, 18, val_1) != 0;

                capability_data.misc_flags = Some(MiscFlags {
                    enable_debug: enable_debug,
                    force_debug: force_debug
                })
            }
            else {
                return result::ResultUnknownCapability::make_err();
            }
        }

        Ok(capability_data)
    }
}

#[derive(Debug)]
pub struct NpdmData {
    pub meta: Meta,
    pub aci0: Aci0,
    pub aci0_fs_access_control: Aci0FsAccessControlData,
    pub aci0_service_access_control: ServiceAccessControlData,
    pub aci0_kernel_capabilities: KernelCapabilityData,
    pub acid: Acid,
    pub acid_fs_access_control: AcidFsAccessControlData,
    pub acid_service_access_control: ServiceAccessControlData,
    pub acid_kernel_capabilities: KernelCapabilityData
}

impl NpdmData {
    pub fn new(npdm: &[u8]) -> Result<Self> {
        let meta: Meta = util::slice_read_val(npdm, None)?;
        result_return_unless!(meta.magic == Meta::MAGIC, result::ResultInvalidMeta);

        let aci0: Aci0 = util::slice_read_val(npdm, Some(meta.aci0_offset as usize))?;
        result_return_unless!(aci0.magic == Aci0::MAGIC, result::ResultInvalidMeta);

        let aci0_fs_access_control_data = util::slice_read_data(npdm, Some(meta.aci0_offset as usize + aci0.fs_access_control_offset as usize), aci0.fs_access_control_size as usize)?;
        let aci0_fs_access_control = Aci0FsAccessControlData::new(&aci0_fs_access_control_data)?;
        let aci0_service_access_control_data = util::slice_read_data(npdm, Some(meta.aci0_offset as usize + aci0.service_access_control_offset as usize), aci0.service_access_control_size as usize)?;
        let aci0_service_access_control = ServiceAccessControlData::new(&aci0_service_access_control_data)?;
        let aci0_kernel_capabilities_data = util::slice_read_data(npdm, Some(meta.aci0_offset as usize + aci0.kernel_capability_offset as usize), aci0.kernel_capability_size as usize)?;
        let aci0_kernel_capabilities = KernelCapabilityData::new(&aci0_kernel_capabilities_data)?;

        let acid: Acid = util::slice_read_val(npdm, Some(meta.acid_offset as usize))?;
        result_return_unless!(acid.magic == Acid::MAGIC, result::ResultInvalidMeta);

        let acid_fs_access_control_data = util::slice_read_data(npdm, Some(meta.acid_offset as usize + acid.fs_access_control_offset as usize), acid.fs_access_control_size as usize)?;
        let acid_fs_access_control = AcidFsAccessControlData::new(&acid_fs_access_control_data)?;
        let acid_service_access_control_data = util::slice_read_data(npdm, Some(meta.acid_offset as usize + acid.service_access_control_offset as usize), acid.service_access_control_size as usize)?;
        let acid_service_access_control = ServiceAccessControlData::new(&acid_service_access_control_data)?;
        let acid_kernel_capabilities_data = util::slice_read_data(npdm, Some(meta.acid_offset as usize + acid.kernel_capability_offset as usize), acid.kernel_capability_size as usize)?;
        let acid_kernel_capabilities = KernelCapabilityData::new(&acid_kernel_capabilities_data)?;

        Ok(Self {
            meta: meta,
            aci0: aci0,
            aci0_fs_access_control: aci0_fs_access_control,
            aci0_service_access_control: aci0_service_access_control,
            aci0_kernel_capabilities: aci0_kernel_capabilities,
            acid: acid,
            acid_fs_access_control: acid_fs_access_control,
            acid_service_access_control: acid_service_access_control,
            acid_kernel_capabilities: acid_kernel_capabilities
        })
    }
}