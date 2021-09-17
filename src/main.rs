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
#![feature(derive_default_enum)]
#![feature(specialization)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]

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
use util::make_log_guard;

#[macro_use]
pub mod ipc;

pub mod ldr;

pub mod emu;

pub mod kern;

pub mod os;

pub mod sm;

pub mod fs;
use fs::FileSystem;

pub mod set;

pub mod proc;

fn main() {
    println!("Hello World!");

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Generate backtrace
        // TODO: backtrace without panic calls, just everything before the panic?
        let backtrace = Backtrace::new();

        // Guard to prevent other thread logs to mix with the panic printing
        let _guard = make_log_guard();

        // Invoke the default panic handler
        orig_hook(panic_info);

        // Print the backtrace, guarding everything to stop other threads from logging
        println!("A panic happened in this thread:");
        println!("{:?}", backtrace);

        // Exit everything, panic = unrecoverable error
        println!("Exiting...");
        process::exit(1);
    }));

    kern::initialize().unwrap();
    proc::initialize().unwrap();

    let mut cpu_ctx = emu::cpu::Context::new();

    let exefs = fs::HostFileSystem::new(String::from("flog.exe"));
    let (start_addr, npdm) = cpu_ctx.load_program(exefs, 0x6900000).unwrap();
    let main_thread_host_name = format!("ext.{}.MainThread", npdm.meta.name);

    let mut process = kern::proc::KProcess::new(Some(cpu_ctx), npdm).unwrap();
    let (mut main_thread, main_thread_handle) = kern::proc::KProcess::create_main_thread(&mut process, main_thread_host_name, start_addr).unwrap();
    log_line!("Running main test program at {:#X}...", start_addr);
    kern::thread::KThread::start_exec(&mut main_thread, 0u64, main_thread_handle).unwrap();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        log_line!("Main --- loop update");
    }
}