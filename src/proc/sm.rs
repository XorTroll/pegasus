use parking_lot::Mutex;

use crate::ipc::sf;
use crate::ipc::sf::client::sm::IUserInterface;
use crate::ipc::server;
use crate::kern::svc::Handle;
use crate::kern::{proc::KProcess, thread::KThread, svc};
use crate::sm::*;
use crate::result::*;
use super::EmulatedProcess;

// Code for the emulated 'sm' process

pub fn start_process() -> Result<()> {
    let npdm = EmulatedProcess::make_npdm("sm", 27, 0x2000, 0x0100_0000_0000_1004, vec![
        svc::SvcId::ManageNamedPort,
        /* ... */
    ], 512)?;

    let process = KProcess::new(None, npdm)?;
    let mut main_thread = KProcess::create_main_thread_host(&process, String::from("pg.proc.sm.MainThread"))?;
    KThread::start_host(&mut main_thread, main_thread_fn)?;
    Ok(())
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
#[repr(C)]
struct ServiceInfo {
    name: ServiceName,
    owner_process_id: u64,
    max_sessions: u32,
    is_light: bool,
    port_handle: Handle
}

static mut G_SERVICES: Mutex<Vec<ServiceInfo>> = parking_lot::const_mutex(Vec::new());

fn has_service_info(name: ServiceName) -> bool {
    unsafe {
        let services = G_SERVICES.lock();

        for service in services.iter() {
            if service.name == name {
                return true;
            }
        }

        false
    }
}

fn register_service_info(info: ServiceInfo) {
    unsafe {
        let mut services = G_SERVICES.lock();
        services.push(info);
    }
}

fn find_service_info(name: ServiceName) -> Result<ServiceInfo> {
    unsafe {
        let services = G_SERVICES.lock();

        for service in services.iter() {
            if service.name == name {
                return Ok(*service);
            }
        }
    }

    Err(ResultCode::new(0x88))
}

fn register_service(name: ServiceName, process_id: u64, max_sessions: u32, is_light: bool) -> Result<Handle> {
    result_return_if!(has_service_info(name), 0xb0);
    
    let (server_handle, client_handle) = svc::create_port(max_sessions, is_light, 0)?;
    let service_info = ServiceInfo {
        name: name,
        owner_process_id: process_id,
        max_sessions: max_sessions,
        is_light: is_light,
        port_handle: server_handle
    };
    register_service_info(service_info);

    Ok(client_handle)
}

fn get_service_handle(name: ServiceName) -> Result<Handle> {
    let service_info = find_service_info(name)?;

    svc::connect_to_port(service_info.port_handle)
}

pub struct UserInterface {
    session: sf::Session
}

impl IUserInterface for UserInterface {
    fn register_client(&mut self, process_id: sf::ProcessId) -> Result<()> {
        todo!("register_client - process_id: {}", process_id.process_id);
    }

    fn get_service_handle(&mut self, name: ServiceName) -> Result<sf::MoveHandle> {
        todo!("get_service_handle, name: {}", name.value);
    }

    fn register_service(&mut self, name: ServiceName, is_light: bool, max_sessions: u32) -> Result<sf::MoveHandle> {
        todo!("register_service - name: {}, is_light: {}, max_sessions: {}", name.value, is_light, max_sessions);
    }

    fn unregister_service(&mut self, name: ServiceName) -> Result<()> {
        todo!("unregister_service - name: {}", name.value);
    }

    fn detach_client(&mut self, process_id: sf::ProcessId) -> Result<()> {
        todo!("detach_client - process_id: {}", process_id.process_id);
    }
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
            ipc_cmif_interface_make_command_meta!(detach_client: 4)
        ]
    }
}

impl server::IServerObject for UserInterface {
    fn new() -> Self {
        Self { session: sf::Session::new() }
    }
}

impl server::INamedPort for UserInterface {
    fn get_port_name() -> &'static str {
        "sm:"
    }

    fn get_max_sesssions() -> u32 {
        0x57
    }
}

fn main_thread_fn() {
    log_line!("Hello World!");

    let mut manager: server::ServerManager<0x0> = server::ServerManager::new().unwrap();

    manager.register_named_port_server::<UserInterface>().unwrap();
    manager.loop_process().unwrap();
}