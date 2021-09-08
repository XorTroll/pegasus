#![feature(const_btree_new)]
#![feature(const_trait_impl)]
#![feature(const_fn_trait_bound)]
#![feature(thread_local)]
#![feature(seek_stream_len)]
#![feature(coerce_unsized)]
#![feature(unsize)]
#![feature(const_mut_refs)]
#![feature(const_raw_ptr_deref)]
#![feature(thread_id_value)]

// For bit_enum enum names
#![allow(non_snake_case)]

use backtrace::Backtrace;
use std::path::PathBuf;
use std::panic;
use std::process;

#[macro_use]
pub mod result;

#[macro_use]
pub mod util;
use util::SharedObject;

pub mod kern;

pub mod fs;
use fs::FileSystem;

use crate::util::make_log_guard;

pub mod ldr;

pub mod emu;

pub mod proc;

fn main() -> result::Result<()> {
    println!("Hello World!");

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Guard to prevent other thread logs to mix with the panic printing
        let _guard = make_log_guard();

        // Invoke the default panic handler
        orig_hook(panic_info);

        // Generate and print backtrace, guarding everything to stop other threads from logging
        println!("A panic happened in this thread:");
        let backtrace = Backtrace::new();
        println!("{:?}", backtrace);

        // Exit everything, panic = unrecoverable error
        println!("Exiting...");
        process::exit(1);
    }));

    let fs = util::make_shared(fs::HostFileSystem::new(String::from("nso_test")));

    let npdm_file = fs.get().open_file(PathBuf::from("nso_test.npdm"), fs::FileOpenMode::Read())?;

    let mut npdm_data: Vec<u8> = vec![0; npdm_file.get().get_size()?];
    npdm_file.get().read(0, &mut npdm_data, fs::ReadOption::None)?;

    let npdm = ldr::npdm::NpdmData::new(&npdm_data)?;
    // log_line!("{:?}", npdm);

    kern::initialize()?;
    proc::sm::start_process()?;

    let mut cpu_ctx = emu::cpu::Context::new();
    
    let nso_file = fs.get().open_file(PathBuf::from("nso_test.nso"), fs::FileOpenMode::Read())?;

    let mut nso_data: Vec<u8> = vec![0; nso_file.get().get_size()?];
    nso_file.get().read(0, &mut nso_data, fs::ReadOption::None)?;

    let nso_start_addr = cpu_ctx.load_nso(0x6900000, nso_data)?;
    let main_thread_host_name = format!("ext.{}.MainThread", npdm.meta.name);

    let mut process = kern::proc::KProcess::new(Some(cpu_ctx), npdm)?;
    let (mut main_thread, main_thread_handle) = kern::proc::KProcess::create_main_thread(&mut process, main_thread_host_name, nso_start_addr)?;
    kern::thread::KThread::start_exec(&mut main_thread, 0u64, main_thread_handle)?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        log_line!("HOSTMAIN 5 secs elapsed");
    }

    Ok(())
}