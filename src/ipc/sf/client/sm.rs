use crate::result::*;
use crate::ipc;
use crate::ipc::sf;
use crate::ipc::sf::client;

pub use crate::sm::*;
pub use crate::ipc::sf::sm::*;

pub struct UserInterface {
    session: sf::Session
}

impl sf::IObject for UserInterface {
    fn get_session(&mut self) -> &mut sf::Session {
        &mut self.session
    }

    fn get_command_table(&self) -> sf::CommandMetadataTable {
        vec! [
            ipc_cmif_interface_make_command_meta!(register_client: 0),
            ipc_cmif_interface_make_command_meta!(get_service_handle: 1),
            ipc_cmif_interface_make_command_meta!(register_service: 2),
            ipc_cmif_interface_make_command_meta!(unregister_service: 3),
            ipc_cmif_interface_make_command_meta!(detach_client: 4),

            ipc_tipc_interface_make_command_meta!(register_client: 0),
            ipc_tipc_interface_make_command_meta!(get_service_handle: 1),
            ipc_tipc_interface_make_command_meta!(register_service: 2),
            ipc_tipc_interface_make_command_meta!(unregister_service: 3),
            ipc_tipc_interface_make_command_meta!(detach_client: 4)
        ]
    }
}

impl client::IClientObject for UserInterface {
    fn new(session: sf::Session) -> Self {
        Self { session: session }
    }
}

impl IUserInterface for UserInterface {
    fn register_client(&mut self, process_id: sf::ProcessId) -> Result<()> {
        ipc_client_send_request_command!([self.session.object_info; 0] (process_id) => ())
    }

    fn get_service_handle(&mut self, name: ServiceName) -> Result<sf::MoveHandle> {
        ipc_client_send_request_command!([self.session.object_info; 1] (name) => (service_handle: sf::MoveHandle))
    }

    fn register_service(&mut self, name: ServiceName, is_light: bool, max_sessions: u32) -> Result<sf::MoveHandle> {
        match self.session.object_info.protocol {
            ipc::CommandProtocol::Cmif => ipc_client_send_request_command!([self.session.object_info; 2] (name, is_light, max_sessions) => (port_handle: sf::MoveHandle)),
            ipc::CommandProtocol::Tipc => ipc_client_send_request_command!([self.session.object_info; 2] (name, max_sessions, is_light) => (port_handle: sf::MoveHandle))
        }
    }

    fn unregister_service(&mut self, name: ServiceName) -> Result<()> {
        ipc_client_send_request_command!([self.session.object_info; 3] (name) => ())
    }

    fn detach_client(&mut self, process_id: sf::ProcessId) -> Result<()> {
        ipc_client_send_request_command!([self.session.object_info; 4] (process_id) => ())
    }
}

impl client::INamedPort for UserInterface {
    fn get_name() -> &'static str {
        "sm:"
    }

    fn post_initialize(&mut self) -> Result<()> {
        /*
        if version::get_version() >= version::Version::new(12, 0, 0) {
            self.session.object_info.protocol = ipc::CommandProtocol::Tipc;
        }
        */
        self.register_client(sf::ProcessId::new())
    }
}