use crate::ipc::sf;
use crate::ipc::sf::set::ISystemSettingsServer;
use crate::ipc::server;
use crate::set::*;
use crate::result::*;
use crate::util::CString;

pub struct SystemSettingsServer {
    session: sf::Session
}

fn current_fw_version(with_revision: bool) -> Result<FirmwareVersion> {
    // TODO: temporary, we'll read it from system files in the future when more stuff gets implemented

    Ok(FirmwareVersion {
        major: 5,
        minor: 1,
        micro: 0,
        pad_1: 0,
        revision_major: match with_revision {
            true => 6,
            false => 0
        },
        revision_minor: match with_revision {
            true => 9,
            false => 0
        },
        pad_2: 0,
        pad_3: 0,
        platform: CString::from_str("PC")?,
        version_hash: CString::from_str("BABABABA")?,
        display_version: CString::from_str("5.1.0")?,
        display_title: CString::from_str("PegasusSDK Firmware for PC 5.1.0-69")?,
    })
}

impl ISystemSettingsServer for SystemSettingsServer {
    fn get_firmware_version(&mut self, mut out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        log_line!("get_firmware_version...");

        out_version.set_as(current_fw_version(false)?);
        Ok(())
    }

    fn get_firmware_version_2(&mut self, mut out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        log_line!("get_firmware_version_2...");
        // Note: same as GetFirmwareVersion, but including the revision fields

        out_version.set_as(current_fw_version(true)?);
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