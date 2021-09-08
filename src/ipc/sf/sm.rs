use crate::sm::*;
use super::*;

pub trait IUserInterface {
    ipc_cmif_tipc_interface_define_command!(register_client: (process_id: sf::ProcessId) => ());
    ipc_cmif_tipc_interface_define_command!(get_service_handle: (name: ServiceName) => (service_handle: sf::MoveHandle));
    ipc_cmif_tipc_interface_define_command!(register_service: (name: ServiceName, is_light: bool, max_sessions: u32) => (port_handle: sf::MoveHandle));
    ipc_cmif_tipc_interface_define_command!(unregister_service: (name: ServiceName) => ());
    ipc_cmif_tipc_interface_define_command!(detach_client: (process_id: sf::ProcessId) => ());
}