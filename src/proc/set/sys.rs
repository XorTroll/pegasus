use crate::ipc::sf;
use crate::ipc::sf::set::ISystemSettingsServer;
use crate::ipc::server;
use crate::set::*;
use crate::result::*;

pub struct SystemSettingsServer {
    session: sf::Session
}

impl ISystemSettingsServer for SystemSettingsServer {
    fn get_firmware_version(&mut self, out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        todo!("get_firmware_version");
    }

    fn get_firmware_version_2(&mut self, out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) -> Result<()> {
        todo!("get_firmware_version_2");
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