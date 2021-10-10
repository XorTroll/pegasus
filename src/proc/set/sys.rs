use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use cntx::nca::ContentType;
use crate::fs::file_read_val;
use crate::fs::{RomFsFileSystem, FileSystem, FileOpenMode, ReadOption};
use crate::ipc::sf;
use crate::ipc::sf::set::ISystemSettingsServer;
use crate::ipc::server;
use crate::ncm::{ProgramId, StorageId, lookup_content};
use crate::set::*;
use crate::result::*;

pub struct SystemSettingsServer {
    session: sf::Session
}

static mut G_FIRMWARE_VERSION_LOADED: AtomicBool = AtomicBool::new(false);
static mut G_FIRMWARE_VERSION: Option<FirmwareVersion> = None;

fn is_firmware_version_loaded() -> bool {
    unsafe {
        G_FIRMWARE_VERSION_LOADED.load(Ordering::SeqCst)
    }
}

fn load_firmware_version(fw_ver: FirmwareVersion) {
    unsafe {
        G_FIRMWARE_VERSION = Some(fw_ver);
        G_FIRMWARE_VERSION_LOADED.store(true, Ordering::SeqCst);
    }
}

pub fn get_firmware_version(with_revision: bool) -> Result<FirmwareVersion> {
    if !is_firmware_version_loaded() {
        const SYSTEM_VERSION_ID: ProgramId = ProgramId(0x0100000000000809);
        let mut system_version_nca = lookup_content(StorageId::BuiltinSystem, SYSTEM_VERSION_ID, ContentType::Data)?;
        let system_version_fs = RomFsFileSystem::from_nca(&mut system_version_nca, 0)?;

        let system_version_file = system_version_fs.get().open_file(PathBuf::from("file"), FileOpenMode::Read())?;
        let fw_ver: FirmwareVersion = file_read_val(&system_version_file, 0, ReadOption::None)?;

        log_line!("Loaded firmware version: {:#?}", fw_ver);

        load_firmware_version(fw_ver);
    }

    let mut fw_ver = unsafe {
        G_FIRMWARE_VERSION.unwrap()
    };

    if !with_revision {
        fw_ver.revision_major = 0;
        fw_ver.revision_minor = 0;
    }
    Ok(fw_ver)
}

impl ISystemSettingsServer for SystemSettingsServer {
    fn get_firmware_version(&mut self, mut out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        log_line!("get_firmware_version...");

        out_version.set_as(get_firmware_version(false)?);
        Ok(())
    }

    fn get_firmware_version_2(&mut self, mut out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        log_line!("get_firmware_version_2...");
        // Note: same as GetFirmwareVersion, but including the revision fields

        out_version.set_as(get_firmware_version(true)?);
        Ok(())
    }
}

impl sf::IObject for SystemSettingsServer {
    fn get_session(&mut self) -> &mut sf::Session {
        &mut self.session
    }

    fn get_command_table(&self) -> sf::CommandMetadataTable {
        vec! [
            ipc_cmif_interface_make_command_meta!(get_firmware_version: 3),
            ipc_cmif_interface_make_command_meta!(get_firmware_version_2: 4)
        ]
    }
}

impl server::IServerObject for SystemSettingsServer {
    fn new() -> Self {
        Self {
            session: sf::Session::new()
        }
    }
}

impl server::IService for SystemSettingsServer {
    fn get_name() -> &'static str {
        "set:sys"
    }

    fn get_max_sesssions() -> u32 {
        0x57
    }
}