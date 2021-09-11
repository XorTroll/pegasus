use crate::set::*;
use super::*;

pub trait ISystemSettingsServer {
    ipc_cmif_interface_define_command!(get_firmware_version: (out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) => ());
    ipc_cmif_interface_define_command!(get_firmware_version_2: (out_version: sf::OutFixedPointerBuffer<FirmwareVersion>) => ());
}