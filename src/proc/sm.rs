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

fn main_thread_fn() {
    log_line!("Hello World!");

    let sm_server_port = svc::manage_named_port("sm:", 0x57).unwrap();
    log_line!("Managed 'sm:' port handle: {:#X}", sm_server_port);

    loop {
        std::thread::sleep(std::time::Duration::from_millis(1));
        log_line!("Loop update...");
    }
}