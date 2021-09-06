use std::collections::BTreeMap;
use crate::emu::cpu;
use crate::kern::svc::{self, BreakReason};
use crate::result::*;

static mut G_SVC_HANDLERS: BTreeMap<svc::SvcId, cpu::HookedInstructionHandlerFn> = BTreeMap::new();

fn do_sleep_thread(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let timeout: i64 = ctx_h.read_register(cpu::Register::X0)?;

    let rc = ResultCode::from(svc::sleep_thread(timeout));
    ctx_h.write_register(cpu::Register::W0, rc)?;
    Ok(())
}

fn do_connect_to_named_port(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let port_name_addr: u64 = ctx_h.read_register(cpu::Register::X1)?;

    let mut port_name_buf: Vec<u8> = Vec::new();
    let mut read_offset = port_name_addr;
    loop {
        let byte: u8 = ctx_h.read_memory_val(read_offset)?;
        if byte == 0 {
            break;
        }
        port_name_buf.push(byte);
        read_offset += 1;
    }

    let port_name = std::str::from_utf8(&port_name_buf).unwrap();

    match svc::connect_to_named_port(port_name) {
        Ok(handle) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, handle)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    };

    Ok(())
}

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
    if str_len > 0 {
        ctx_h.read_memory(str_addr, &mut str_buf)?;
    }
    let msg = std::str::from_utf8(&str_buf).unwrap();

    let rc = ResultCode::from(svc::output_debug_string(msg));
    ctx_h.write_register(cpu::Register::W0, rc)?;
    Ok(())
}

fn do_manage_named_port(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let port_name_addr: u64 = ctx_h.read_register(cpu::Register::X1)?;
    let max_sessions: u32 = ctx_h.read_register(cpu::Register::W2)?;

    let mut port_name_buf: Vec<u8> = Vec::new();
    let mut read_offset = port_name_addr;
    loop {
        let byte: u8 = ctx_h.read_memory_val(read_offset)?;
        if byte == 0 {
            break;
        }
        port_name_buf.push(byte);
        read_offset += 1;
    }

    let port_name = std::str::from_utf8(&port_name_buf).unwrap();
    
    match svc::manage_named_port(port_name, max_sessions) {
        Ok(handle) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, handle)?;
        },
        Err(rc) => ctx_h.write_register(cpu::Register::W0, rc)?
    };

    Ok(())
}

unsafe fn create_svc_handlers() {
    G_SVC_HANDLERS.insert(svc::SvcId::SleepThread, Box::new(do_sleep_thread));
    G_SVC_HANDLERS.insert(svc::SvcId::ConnectToNamedPort, Box::new(do_connect_to_named_port));
    G_SVC_HANDLERS.insert(svc::SvcId::Break, Box::new(do_break));
    G_SVC_HANDLERS.insert(svc::SvcId::OutputDebugString, Box::new(do_output_debug_string));
    G_SVC_HANDLERS.insert(svc::SvcId::ManageNamedPort, Box::new(do_manage_named_port));
}

pub fn try_find_svc_handler(key: &svc::SvcId) -> Option<&cpu::HookedInstructionHandlerFn> {
    unsafe {
        if G_SVC_HANDLERS.is_empty() {
            create_svc_handlers();
        }

        G_SVC_HANDLERS.get(key)
    }
}