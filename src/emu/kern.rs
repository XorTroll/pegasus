use std::collections::BTreeMap;
use crate::emu::cpu;
use crate::emu::cpu::result;
use crate::kern;
use crate::kern::svc::{self, BreakReason};
use crate::result::*;

static mut G_SVC_HANDLERS: BTreeMap<svc::SvcId, cpu::HookedInstructionHandlerFn> = BTreeMap::new();

fn do_break(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let reason: BreakReason = ctx_h.read_register(cpu::Register::W0)?;
    let arg_addr: u64 = ctx_h.read_register(cpu::Register::X1)?;
    let arg_len: usize = ctx_h.read_register(cpu::Register::X2)?;

    let mut arg: Vec<u8> = vec![0; arg_len];
    if arg_len > 0 {
        ctx_h.read_memory(arg_addr, &mut arg)?;
    }

    let rc = ResultCode::from(svc::break_(reason, &arg));
    ctx_h.write_register(cpu::Register::W0, rc)?;
    Ok(())
}

fn do_output_debug_string(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let str_addr: u64 = ctx_h.read_register(cpu::Register::X0)?;
    let str_len: usize = ctx_h.read_register(cpu::Register::X1)?;

    let mut str_buf: Vec<u8> = vec![0; str_len];
    result::convert_unicorn_error(ctx_h.0.mem_read(str_addr, &mut str_buf))?;
    let msg = std::str::from_utf8(&str_buf).unwrap();

    let rc = ResultCode::from(svc::output_debug_string(msg));
    ctx_h.write_register(cpu::Register::W0, rc)?;
    Ok(())
}

fn do_sleep_thread(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let timeout: i64 = ctx_h.read_register(cpu::Register::X0)?;

    todo!("SleepThread with timeout {:#X}", timeout);
}

unsafe fn create_svc_handlers() {
    G_SVC_HANDLERS.insert(svc::SvcId::SleepThread, Box::new(do_sleep_thread));
    G_SVC_HANDLERS.insert(svc::SvcId::Break, Box::new(do_break));
    G_SVC_HANDLERS.insert(svc::SvcId::OutputDebugString, Box::new(do_output_debug_string));
}

pub fn try_find_svc_handler(key: &svc::SvcId) -> Option<&cpu::HookedInstructionHandlerFn> {
    unsafe {
        if G_SVC_HANDLERS.is_empty() {
            create_svc_handlers();
        }

        G_SVC_HANDLERS.get(key)
    }
}