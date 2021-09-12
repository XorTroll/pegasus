use crate::ipc::sf;
use crate::ipc::sf::set::ISystemSettingsServer;
use crate::ipc::server;
use crate::set::*;
use crate::result::*;
use crate::util::CString;

pub struct SystemSettingsServer {
    session: sf::Session
}

impl ISystemSettingsServer for SystemSettingsServer {
    fn get_firmware_version(&mut self, out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        todo!("get_firmware_version");
    }

    fn get_firmware_version_2(&mut self, mut out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        let fw_ver = FirmwareVersion {
            major: 5,
            minor: 1,
            micro: 0,
            pad_1: 0,
            revision_major: 0,
            revision_minor: 0,
            pad_2: 0,
            pad_3: 0,
            platform: CString::from_str("NintendoSDK for balls")?,
            version_hash: CString::from_str("BABABABA")?,
            display_version: CString::from_str("5.1.0")?,
            display_title: CString::from_str("NintendoSDK for balls 5.1.0")?,
        };
        out_version.set_as(fw_ver);
        Ok(())
        // todo!("get_firmware_version_2");
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