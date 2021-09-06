use crate::{ldr::npdm::{self, MetaFlags, MiscFlags, MiscParams, ThreadInfo}, util};
use crate::kern::svc;
use crate::result::*;

pub mod sm;

pub struct EmulatedProcess {
}

impl EmulatedProcess {
    pub fn make_npdm(name: &str, main_thread_priority: i32, main_thread_stack_size: usize, program_id: u64, enabled_svcs: Vec<svc::SvcId>, handle_table_size: usize) -> Result<npdm::NpdmData> {
        Ok(npdm::NpdmData {
            meta: npdm::Meta {
                magic: npdm::Meta::MAGIC,
                acid_signature_key_generation: 0,
                reserved_1: [0; 0x4],
                flags: MetaFlags::new(true, npdm::AddressSpaceType::AS64Bit, false, false),
                reserved_2: 0,
                main_thread_priority: main_thread_priority as u8,
                main_thread_cpu_core: 3,
                reserved_3: [0; 0x4],
                system_resource_size: 0,
                version: 0,
                main_thread_stack_size: main_thread_stack_size as u32,
                name: util::CString::from_str(name)?,
                product_code: util::CString::new(),
                reserved_4: [0; 0x30],
                aci0_offset: 0, // Note: this offsets/sizes won't be read
                aci0_size: 0,
                acid_offset: 0,
                acid_size: 0
            },
            aci0: npdm::Aci0 {
                magic: npdm::Aci0::MAGIC,
                reserved_1: [0; 0xC],
                program_id: program_id,
                reserved_2: [0; 0x8],
                fs_access_control_offset: 0, // Same as above
                fs_access_control_size: 0,
                service_access_control_offset: 0,
                service_access_control_size: 0,
                kernel_capability_offset: 0,
                kernel_capability_size: 0,
                reserved_3: [0; 0x8]
            },
            aci0_fs_access_control: npdm::Aci0FsAccessControlData {
                version: 1,
                flags: npdm::FsAccessFlag::FullPermission(),
                content_owner_info_offset: 0,
                content_owner_info_size: 0,
                content_owner_ids: Vec::new(),
                save_data_owner_info_offset: 0,
                save_data_owner_info_size: 0,
                accessibilities: Vec::new(),
                save_data_owner_ids: Vec::new(),
            },
            aci0_service_access_control: npdm::ServiceAccessControlData {
                services: vec![
                    npdm::ServiceAccessControlEntry {
                        name: String::from("*"),
                        is_server: false
                    },
                    npdm::ServiceAccessControlEntry {
                        name: String::from("*"),
                        is_server: true
                    }
                ]
            },
            aci0_kernel_capabilities: npdm::KernelCapabilityData {
                thread_info: Some(ThreadInfo {
                    highest_priority: main_thread_priority as u8,
                    lowest_priority: main_thread_priority as u8,
                    min_core_number: 3,
                    max_core_number: 3
                }),
                enabled_svcs: enabled_svcs.clone(),
                memory_maps: Vec::new(),
                io_memory_maps: Vec::new(),
                mem_region_maps: Vec::new(),
                enable_interrupts: None,
                misc_params: Some(MiscParams {
                    program_type: npdm::ProgramType::System
                }),
                kernel_version: Some(npdm::KernelVersion {
                    major: 3,
                    minor: 0
                }),
                handle_table_size: Some(handle_table_size as u16),
                misc_flags: Some(MiscFlags {
                    enable_debug: false,
                    force_debug: false
                })
            },
            acid: npdm::Acid {
                rsa_signature: [0; 0x100],
                rsa_nca_sig_public_key: [0; 0x100],
                magic: npdm::Acid::MAGIC,
                size: 0,
                reserved_1: [0; 0x4],
                flags: npdm::AcidFlags::new(true, false),
                program_id_min: 0,
                program_id_max: 0,
                fs_access_control_offset: 0, // Same as above
                fs_access_control_size: 0,
                service_access_control_offset: 0,
                service_access_control_size: 0,
                kernel_capability_offset: 0,
                kernel_capability_size: 0,
                reserved_2: [0; 0x8]
            },
            acid_fs_access_control: npdm::AcidFsAccessControlData {
                version: 1,
                flags: npdm::FsAccessFlag::FullPermission(),
                content_owner_id_min: 0,
                content_owner_id_max: 0,
                content_owner_ids: Vec::new(),
                save_data_owner_id_min: 0,
                save_data_owner_id_max: 0,
                save_data_owner_ids: Vec::new(),
            },
            acid_service_access_control: npdm::ServiceAccessControlData {
                services: vec![
                    npdm::ServiceAccessControlEntry {
                        name: String::from("*"),
                        is_server: false
                    },
                    npdm::ServiceAccessControlEntry {
                        name: String::from("*"),
                        is_server: true
                    }
                ]
            },
            acid_kernel_capabilities: npdm::KernelCapabilityData {
                thread_info: Some(ThreadInfo {
                    highest_priority: main_thread_priority as u8,
                    lowest_priority: main_thread_priority as u8,
                    min_core_number: 3,
                    max_core_number: 3
                }),
                enabled_svcs: enabled_svcs.clone(),
                memory_maps: Vec::new(),
                io_memory_maps: Vec::new(),
                mem_region_maps: Vec::new(),
                enable_interrupts: None,
                misc_params: Some(MiscParams {
                    program_type: npdm::ProgramType::System
                }),
                kernel_version: Some(npdm::KernelVersion {
                    major: 3,
                    minor: 0
                }),
                handle_table_size: Some(handle_table_size as u16),
                misc_flags: Some(MiscFlags {
                    enable_debug: false,
                    force_debug: false
                })
            }
        })
    }

}