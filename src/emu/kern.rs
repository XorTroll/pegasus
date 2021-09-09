use std::collections::BTreeMap;
use std::mem;
use crate::emu::cpu;
use crate::kern::svc::{self, BreakReason, Handle};
use crate::result::*;

static mut G_SVC_HANDLERS: BTreeMap<svc::SvcId, cpu::HookedInstructionHandlerFn> = BTreeMap::new();

fn do_sleep_thread(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let timeout: i64 = ctx_h.read_register(cpu::Register::X0)?;

    let rc = ResultCode::from(svc::sleep_thread(timeout));
    ctx_h.write_register(cpu::Register::W0, rc)?;
    Ok(())
}

fn do_close_handle(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let handle: Handle = ctx_h.read_register(cpu::Register::W0)?;

    let rc = ResultCode::from(svc::close_handle(handle));
    ctx_h.write_register(cpu::Register::W0, rc)?;
    Ok(())
}

fn do_wait_synchronization(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let handles_addr: u64 = ctx_h.read_register(cpu::Register::X1)?;
    let handles_count: u32 = ctx_h.read_register(cpu::Register::W2)?;
    let timeout: i64 = ctx_h.read_register(cpu::Register::X3)?;

    let mut handles: Vec<Handle> = Vec::with_capacity(handles_count as usize);
    let mut read_offset = handles_addr;
    for _ in 0..handles_count {
        let handle: Handle = ctx_h.read_memory_val(read_offset)?;
        handles.push(handle);
        read_offset += mem::size_of_val(&handle) as u64;
    }

    match svc::wait_synchronization(&handles, timeout) {
        Ok(idx) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, idx)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    }

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
        read_offset += mem::size_of_val(&byte) as u64;
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

fn do_send_sync_request(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let client_session_handle: Handle = ctx_h.read_register(cpu::Register::W0)?;

    let rc = ResultCode::from(svc::send_sync_request(client_session_handle));
    ctx_h.write_register(cpu::Register::W0, rc)?;
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

fn do_create_session(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let is_light: bool = ctx_h.read_register(cpu::Register::W2)?;
    let name_addr: u64 = ctx_h.read_register(cpu::Register::X3)?;

    match svc::create_session(is_light, name_addr) {
        Ok((server_session_handle, client_session_handle)) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, server_session_handle)?;
            ctx_h.write_register(cpu::Register::W2, client_session_handle)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    }

    Ok(())
}

fn do_accept_session(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let server_port_handle: Handle = ctx_h.read_register(cpu::Register::W1)?;

    match svc::accept_session(server_port_handle) {
        Ok(server_session_handle) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, server_session_handle)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    }

    Ok(())
}

fn do_reply_and_receive(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let handles_addr: u64 = ctx_h.read_register(cpu::Register::X1)?;
    let handles_count: u32 = ctx_h.read_register(cpu::Register::W2)?;
    let reply_target_session_handle: Handle = ctx_h.read_register(cpu::Register::W3)?;
    let timeout: i64 = ctx_h.read_register(cpu::Register::X4)?;

    let mut handles: Vec<Handle> = Vec::with_capacity(handles_count as usize);
    let mut read_offset = handles_addr;
    for _ in 0..handles_count {
        let handle: Handle = ctx_h.read_memory_val(read_offset)?;
        handles.push(handle);
        read_offset += mem::size_of_val(&handle) as u64;
    }

    match svc::reply_and_receive(&handles, reply_target_session_handle, timeout) {
        Ok(idx) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, idx)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    }

    Ok(())
}

fn do_create_port(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let max_sessions: u32 = ctx_h.read_register(cpu::Register::W2)?;
    let is_light: bool = ctx_h.read_register(cpu::Register::W3)?;
    let name_addr: u64 = ctx_h.read_register(cpu::Register::X4)?;

    match svc::create_port(max_sessions, is_light, name_addr) {
        Ok((server_port_handle, client_port_handle)) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, server_port_handle)?;
            ctx_h.write_register(cpu::Register::W2, client_port_handle)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    };

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
        read_offset += mem::size_of_val(&byte) as u64;
    }

    let port_name = std::str::from_utf8(&port_name_buf).unwrap();
    
    match svc::manage_named_port(port_name, max_sessions) {
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

fn do_connect_to_port(mut ctx_h: cpu::ContextHandle) -> Result<()> {
    let client_port_handle: Handle = ctx_h.read_register(cpu::Register::W1)?;

    match svc::connect_to_port(client_port_handle) {
        Ok(session_handle) => {
            ctx_h.write_register(cpu::Register::W0, ResultSuccess::make())?;
            ctx_h.write_register(cpu::Register::W1, session_handle)?;
        },
        Err(rc) => {
            ctx_h.write_register(cpu::Register::W0, rc)?;
        }
    };

    Ok(())
}

unsafe fn create_svc_handlers() {
    G_SVC_HANDLERS.insert(svc::SvcId::SleepThread, Box::new(do_sleep_thread));
    G_SVC_HANDLERS.insert(svc::SvcId::CloseHandle, Box::new(do_close_handle));
    G_SVC_HANDLERS.insert(svc::SvcId::WaitSynchronization, Box::new(do_wait_synchronization));
    G_SVC_HANDLERS.insert(svc::SvcId::ConnectToNamedPort, Box::new(do_connect_to_named_port));
    G_SVC_HANDLERS.insert(svc::SvcId::SendSyncRequest, Box::new(do_send_sync_request));
    G_SVC_HANDLERS.insert(svc::SvcId::Break, Box::new(do_break));
    G_SVC_HANDLERS.insert(svc::SvcId::OutputDebugString, Box::new(do_output_debug_string));
    G_SVC_HANDLERS.insert(svc::SvcId::CreateSession, Box::new(do_create_session));
    G_SVC_HANDLERS.insert(svc::SvcId::AcceptSession, Box::new(do_accept_session));
    G_SVC_HANDLERS.insert(svc::SvcId::ReplyAndReceive, Box::new(do_reply_and_receive));
    G_SVC_HANDLERS.insert(svc::SvcId::CreatePort, Box::new(do_create_port));
    G_SVC_HANDLERS.insert(svc::SvcId::ManageNamedPort, Box::new(do_manage_named_port));
    G_SVC_HANDLERS.insert(svc::SvcId::ConnectToPort, Box::new(do_connect_to_port));
}

pub fn try_find_svc_handler(key: &svc::SvcId) -> Option<&cpu::HookedInstructionHandlerFn> {
    unsafe {
        if G_SVC_HANDLERS.is_empty() {
            create_svc_handlers();
        }

        G_SVC_HANDLERS.get(key)
    }
}