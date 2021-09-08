use parking_lot::Mutex;

use crate::kern::svc::Handle;
use crate::kern::{proc::KProcess, thread::KThread, svc};
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
pub struct ServiceName {
    pub value: u64,
}

impl ServiceName {
    pub const fn from(value: u64) -> Self {
        Self { value: value }
    }
    
    pub const fn new(name: &str) -> Self {
        // Note: for the name to be valid, it should end with at least a NUL terminator (use the nul!("name") macro present in this crate for that)
        let value = unsafe { *(name.as_ptr() as *const u64) };
        Self::from(value)
    }

    pub const fn is_empty(&self) -> bool {
        self.value == 0
    }

    pub const fn empty() -> Self {
        Self::from(0)
    }
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

fn main_thread_fn() {
    log_line!("Hello World!");

    let sm_server_port_handle = svc::manage_named_port("sm:", 0x57).unwrap();
    log_line!("Managed 'sm:' port handle: {:#X}", sm_server_port_handle);

    loop {
        log_line!("Loop update...");

        match svc::wait_synchronization(&[sm_server_port_handle], -1) {
            Ok(idx) => panic!("Wait succeeded at index {}!", idx),
            Err(rc) => log_line!("Wait failed: {0} --- {0:?}", rc)
        };
    }
}