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
use crate::fs::FileSystem;
use crate::kern::thread::try_get_current_thread;
use crate::util::Shared;

pub mod os;

pub mod sm;

pub mod fs;

pub mod set;

pub mod ncm;

pub mod proc;

fn main() {
    println!("Hello World!");

    let orig_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // Generate backtrace
        // TODO: backtrace without panic calls, just everything before the panic?
        // TODO: actual code backtrace for external programs?
        let backtrace = Backtrace::new();

        // Guard to prevent other thread logs to mix with the panic printing
        let _guard = make_log_guard();

        // Invoke the default panic handler
        orig_hook(panic_info);

        println!();

        // Show information about the panicking thread/process, if possible
        if let Some(thread) = try_get_current_thread() {
            println!(" ---- Thread/process info ----");
            println!();

            if let Some(proc) = thread.get().owner_process.as_ref() {
                println!("* Process name: '{}'", proc.get().npdm.meta.name.get_str().unwrap());
                println!("* Process ID: {:#X}", proc.get().id);
                println!("* Program ID: {:#018X}", proc.get().npdm.aci0.program_id);

                if let Some(ctx) = proc.get().cpu_ctx.as_ref() {
                    println!("* Modules:");
                    for module in ctx.modules.iter() {
                        let mod_name = match module.get_name() {
                            Some(name) => name,
                            None => String::from("<unk>")
                        };

                        println!(" -- {} (file: {})", mod_name, module.file_name);
                    }
                }
            }
            else {
                println!("* Not a process...");
            }

            // TODO: thread name from TLS
            println!("* Host thread name: '{}'", thread.get().get_host_name());
            println!("* Is emulated thread: {}", thread.get().is_emu_thread());

            // If the thread is from an actual external program, print some of its registers
            if let Some(exec_ctx) = thread.get().cpu_exec_ctx.as_ref() {
                let handle = exec_ctx.get_handle();
                println!("* Registers:");
                println!(" -- PC: {:#X}", handle.read_register::<u64>(emu::cpu::Register::PC).unwrap());
                println!(" -- X0: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X0).unwrap());
                println!(" -- X1: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X1).unwrap());
                println!(" -- X2: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X2).unwrap());
                println!(" -- X3: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X3).unwrap());
                println!(" -- X4: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X4).unwrap());
                println!(" -- X5: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X5).unwrap());
                println!(" -- X6: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X6).unwrap());
                println!(" -- X7: {:#X}", handle.read_register::<u64>(emu::cpu::Register::X7).unwrap());
            }

            println!();
        }

        // Print the backtrace
        println!(" ---- Emulator backtrace ----");
        println!();

        println!("{:?}", backtrace);

        // Exit everything, panic = unrecoverable error
        println!("Exiting...");
        process::exit(1);
    }));

    emu::cfg::initialize().unwrap();
    ncm::initialize().unwrap();

    {
        // Test

        let mut system_version_nca = ncm::lookup_content(ncm::StorageId::BuiltinSystem, 0x0100000000000809, cntx::nca::ContentType::Data).unwrap();
        let mut system_version_fs = system_version_nca.open_romfs_filesystem(0).unwrap();

        let mut fw_ver: set::FirmwareVersion = unsafe {
            core::mem::zeroed()
        };
        let fw_ver_buf = unsafe {
            std::slice::from_raw_parts_mut(&mut fw_ver as *mut _ as *mut u8, std::mem::size_of::<set::FirmwareVersion>())
        };
        system_version_fs.read_file(String::from("file"), 0, fw_ver_buf).unwrap();

        log_line!("Read fw ver: {:?}", fw_ver);
    }

    kern::initialize().unwrap();
    proc::initialize().unwrap();

    enum TestRunKind {
        SystemTitle(ncm::ProgramId),
        TestNso(String)
    }

    // let run_kind = TestRunKind::SystemTitle(0x0100000000001000);
    let run_kind = TestRunKind::TestNso(String::from("nso_test/build/exefs"));

    // Simplify running different kinds of programs while main is not properly finished (can't get to test IPC with system titles without implementing several SVCs)

    let exefs: Shared<dyn FileSystem> = match run_kind {
        TestRunKind::SystemTitle(program_id) => {
            let mut system_title_nca = ncm::lookup_content(ncm::StorageId::BuiltinSystem, program_id, cntx::nca::ContentType::Program).unwrap();
            fs::PartitionFileSystem::from_nca(&mut system_title_nca, 0).unwrap()
        },
        TestRunKind::TestNso(exefs_path) => {
            fs::HostFileSystem::new(exefs_path)
        }
    };

    let mut cpu_ctx = emu::cpu::Context::new();
    let (start_addr, npdm) = cpu_ctx.load_program(exefs, 0x6900000).unwrap();
    let process_name = npdm.meta.name.get_string().unwrap();
    let main_thread_host_name = format!("ext.{}.MainThread", process_name);

    let mut process = kern::proc::KProcess::new(Some(cpu_ctx), npdm).unwrap();
    let (mut main_thread, main_thread_handle) = kern::proc::KProcess::create_main_thread(&mut process, main_thread_host_name, start_addr).unwrap();
    log_line!("Running process '{}' at {:#X}...", process_name, start_addr);
    kern::thread::KThread::start_exec(&mut main_thread, 0u64, main_thread_handle).unwrap();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        log_line!("Main --- loop update");
    }
}