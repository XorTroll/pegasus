#![feature(const_btree_new)]
#![feature(const_trait_impl)]
#![feature(const_fn_trait_bound)]
#![feature(thread_local)]
#![feature(seek_stream_len)]
#![feature(coerce_unsized)]
#![feature(unsize)]

// For bit_enum enum names
#![allow(non_snake_case)]

use std::path::PathBuf;
use std::panic;
use std::process;

use crate::fs::FileSystem;

#[macro_use]
pub mod result;

#[macro_use]
pub mod util;

pub mod kern;

pub mod fs;

pub mod ldr;

pub mod emu;

fn main() -> result::Result<()> {
    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // invoke the default handler and exit the process
        orig_hook(panic_info);
        println!("A panic happened - exiting pegasus...");
        process::exit(1);
    }));

    let mut fs = util::Shared::new(fs::HostFileSystem::new(String::from("/mnt/c/Users/XaboF/OneDrive/Desktop/pegasus/nso_test")));

    let mut npdm_file = fs.get().open_file(PathBuf::from("nso_test.npdm"), fs::FileOpenMode::Read())?;

    let mut npdm_data: Vec<u8> = vec![0; npdm_file.get().get_size()?];
    npdm_file.get().read(0, &mut npdm_data, fs::ReadOption::None)?;

    let npdm = ldr::npdm::NpdmData::new(&npdm_data)?;
    println!("{:?}", npdm);

    kern::thread::initialize_schedulers()?;

    let mut cpu_ctx = emu::cpu::Context::new();
    
    let mut nso_file = fs.get().open_file(PathBuf::from("nso_test.nso"), fs::FileOpenMode::Read())?;

    let mut nso_data: Vec<u8> = vec![0; nso_file.get().get_size()?];
    nso_file.get().read(0, &mut nso_data, fs::ReadOption::None)?;

    let nso_start_addr = cpu_ctx.load_nso(0x6900000, nso_data)?;
    let main_thread_host_name = format!("ext.{}.MainThread", npdm.meta.name);

    let process = kern::proc::KProcess::new(cpu_ctx, npdm);
    let mut main_thread = kern::proc::KProcess::create_main_thread(&process, main_thread_host_name, nso_start_addr)?;
    kern::thread::KThread::start_exec(&mut main_thread)?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
        println!("HOSTMAIN 5 secs elapsed");
    }

    println!("Done!");

    Ok(())
}