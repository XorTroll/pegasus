use crate::ipc::server;
use crate::kern::{proc::KProcess, thread::KThread};
use crate::result::*;
use super::EmulatedProcess;

// Code for the emulated 'settings' process

pub mod sys;

pub fn start_process() -> Result<()> {
    let npdm = EmulatedProcess::make_npdm("settings", 27, 0x2000, 0x0100_0000_0000_1009, vec![
        /* ... */
    ], 512)?;

    let process = KProcess::new(None, npdm)?;
    let mut main_thread = KProcess::create_main_thread_host(&process, String::from("pg.proc.settings.MainThread"))?;
    KThread::start_host(&mut main_thread, main_thread_fn)?;
    Ok(())
}

fn main_thread_fn() {
    log_line!("Hello World!");

    let mut manager: server::ServerManager<0x100> = server::ServerManager::new().unwrap();

    manager.register_service_server::<sys::SystemSettingsServer>().unwrap();
    manager.loop_process().unwrap();
}