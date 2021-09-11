use scopeguard::{guard, ScopeGuard};
use crate::kern::KAutoObject;
use crate::kern::KSynchronizationObject;
use crate::kern::find_named_object;
use crate::kern::ipc::KClientPort;
use crate::kern::ipc::KServerPort;
use crate::kern::ipc::KPort;
use crate::kern::ipc::KClientSession;
use crate::kern::ipc::KServerSession;
use crate::kern::proc::get_current_process;
use crate::kern::register_named_object;
use crate::kern::result;
use crate::kern::wait_for_sync_objects;
use crate::result::*;
use crate::util::Shared;
use crate::util;
use core::mem;
use std::time::Duration;

pub type Handle = u32;

pub const INVALID_HANDLE: Handle = 0;
pub const CURRENT_THREAD_PSEUDO_HANDLE: Handle = 0xFFFF8000;
pub const CURRENT_PROCESS_PSEUDO_HANDLE: Handle = 0xFFFF8001;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum LimitableResource {
    PhysicalMemory = 0,
    Thread = 1,
    Event = 2,
    TransferMemory = 3,
    Session = 4
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[repr(u8)]
pub enum SvcId {
    SetHeapSize = 0x01,
    SetMemoryPermission = 0x02,
    SetMemoryAttribute = 0x03,
    MapMemory = 0x04,
    UnmapMemory = 0x05,
    QueryMemory = 0x06,
    ExitProcess = 0x07,
    CreateThread = 0x08,
    StartThread = 0x09,
    ExitThread = 0x0A,
    SleepThread = 0x0B,
    GetThreadPriority = 0x0C,
    SetThreadPriority = 0x0D,
    GetThreadCoreMask = 0x0E,
    SetThreadCoreMask = 0x0F,
    GetCurrentProcessorNumber = 0x10,
    SignalEvent = 0x11,
    ClearEvent = 0x12,
    MapSharedMemory = 0x13,
    UnmapSharedMemory = 0x14,
    CreateTransferMemory = 0x15,
    CloseHandle = 0x16,
    ResetSignal = 0x17,
    WaitSynchronization = 0x18,
    CancelSynchronization = 0x19,
    ArbitrateLock = 0x1A,
    ArbitrateUnlock = 0x1B,
    WaitProcessWideKeyAtomic = 0x1C,
    SignalProcessWideKey = 0x1D,
    GetSystemTick = 0x1E,
    ConnectToNamedPort = 0x1F,
    SendSyncRequestLight = 0x20,
    SendSyncRequest = 0x21,
    SendSyncRequestWithUserBuffer = 0x22,
    SendAsyncRequestWithUserBuffer = 0x23,
    GetProcessId = 0x24,
    GetThreadId = 0x25,
    Break = 0x26,
    OutputDebugString = 0x27,
    ReturnFromException = 0x28,
    GetInfo = 0x29,
    FlushEntireDataCache = 0x2A,
    FlushDataCache = 0x2B,
    MapPhysicalMemory = 0x2C,
    UnmapPhysicalMemory = 0x2D,
    GetFutureThreadInfo = 0x2E,
    GetLastThreadInfo = 0x2F,
    GetResourceLimitLimitValue = 0x30,
    GetResourceLimitCurrentValue = 0x31,
    SetThreadActivity = 0x32,
    GetThreadContext3 = 0x33,
    WaitForAddress = 0x34,
    SignalToAddress = 0x35,
    SynchronizePreemptionState = 0x36,
    GetResourceLimitPeakValue = 0x37,
    Unknown0x38 = 0x38,
    Unknown0x39 = 0x39,
    Unknown0x3A = 0x3A,
    Unknown0x3B = 0x3B,
    DumpInfoKernelDebug = 0x3C,
    ChangeKernelTraceState = 0x3D,
    Unknown0x3E = 0x3E,
    Unknown0x3F = 0x3F,
    CreateSession = 0x40,
    AcceptSession = 0x41,
    ReplyAndReceiveLight = 0x42,
    ReplyAndReceive = 0x43,
    ReplyAndReceiveWithUserBuffer = 0x44,
    CreateEvent = 0x45,
    Unknown0x46 = 0x46,
    Unknown0x47 = 0x47,
    MapPhysicalMemoryUnsafe = 0x48,
    UnmapPhysicalMemoryUnsafe = 0x49,
    SetUnsafeLimit = 0x4A,
    CreateCodeMemory = 0x4B,
    ControlCodeMemory = 0x4C,
    SleepSystem = 0x4D,
    ReadWriteRegister = 0x4E,
    SetProcessActivity = 0x4F,
    CreateSharedMemory = 0x50,
    MapTransferMemory = 0x51,
    UnmapTransferMemory = 0x52,
    CreateInterruptEvent = 0x53,
    QueryPhysicalAddress = 0x54,
    QueryIoMapping = 0x55,
    CreateDeviceAddressSpace = 0x56,
    AttachDeviceAddressSpace = 0x57,
    DetachDeviceAddressSpace = 0x58,
    MapDeviceAddressSpaceByForce = 0x59,
    MapDeviceAddressSpaceAligned = 0x5A,
    MapDeviceAddressSpace = 0x5B,
    UnmapDeviceAddressSpace = 0x5C,
    InvalidateProcessDataCache = 0x5D,
    StoreProcessDataCache = 0x5E,
    FlushProcessDataCache = 0x5F,
    DebugActiveProcess = 0x60,
    BreakDebugProcess = 0x61,
    TerminateDebugProcess = 0x62,
    GetDebugEvent = 0x63,
    ContinueDebugEvent = 0x64,
    GetProcessList = 0x65,
    GetThreadList = 0x66,
    GetDebugThreadContext = 0x67,
    SetDebugThreadContext = 0x68,
    QueryDebugProcessMemory = 0x69,
    ReadDebugProcessMemory = 0x6A,
    WriteDebugProcessMemory = 0x6B,
    SetHardwareBreakPoint = 0x6C,
    GetDebugThreadParam = 0x6D,
    Unknown0x6E = 0x6E,
    GetSystemInfo = 0x6F,
    CreatePort = 0x70,
    ManageNamedPort = 0x71,
    ConnectToPort = 0x72,
    SetProcessMemoryPermission = 0x73,
    MapProcessMemory = 0x74,
    UnmapProcessMemory = 0x75,
    QueryProcessMemory = 0x76,
    MapProcessCodeMemory = 0x77,
    UnmapProcessCodeMemory = 0x78,
    CreateProcess = 0x79,
    StartProcess = 0x7A,
    TerminateProcess = 0x7B,
    GetProcessInfo = 0x7C,
    CreateResourceLimit = 0x7D,
    SetResourceLimitLimitValue = 0x7E,
    CallSecureMonitor = 0x7F
}

impl SvcId {
    pub const fn from(raw: u8) -> Option<Self> {
        if (raw == 0) || (raw >= 0x80) {
            return None;
        }

        unsafe {
            Some(core::mem::transmute(raw))
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum BreakReason {
    Panic = 0,
    Assert = 1,
    User = 2,
    PreLoadDll = 3,
    PostLoadDll = 4,
    PreUnloadDll = 5,
    PostUnloadDll = 6,
    CppException = 7,
    NotificationOnlyFlag = 0x80000000
}

// Special impl for the only enum which is a normal enum + flag enum at the same time :P

impl BreakReason {
    pub fn is_notification_only(self) -> bool {
        ((self as u32) & (BreakReason::NotificationOnlyFlag as u32)) != 0
    }

    pub fn without_notification_flag(self) -> Self {
        unsafe {
            core::mem::transmute((self as u32) & !(BreakReason::NotificationOnlyFlag as u32))
        }
    }
}

// Note: the actual impl of SVCs would have (ptr, size) for args/bufs/strings, but Rust's slice, &str, etc. makes my life way easier here ;)

pub fn sleep_thread(timeout: i64) -> Result<()> {
    match timeout {
        0 => todo!("Yield"),
        -1 => todo!("YieldWithLoadBalancing"),
        -2 => todo!("YieldToAnyThread"),
        timeout => {
            let duration = Duration::from_nanos(timeout as u64);
            todo!("SleepThread with timeout = {}ns", duration.as_nanos());
        }
    }
}

pub fn close_handle(handle: Handle) -> Result<()> {
    get_current_process().get().handle_table.close_handle(handle)
}

pub fn wait_synchronization(handles: &[Handle], timeout: i64) -> Result<usize> {
    result_return_unless!(handles.len() <= 64, result::ResultOutOfRange);

    let mut sync_objs: Vec<Shared<dyn KSynchronizationObject>> = Vec::with_capacity(handles.len());
    for handle in handles {
        let sync_obj = get_current_process().get().handle_table.get_handle_sync_obj(*handle)?;
        // sync_obj.get().increment_refcount();

        sync_objs.push(sync_obj);
    }

    wait_for_sync_objects(&mut sync_objs, timeout)
}

pub fn connect_to_named_port(name: &str) -> Result<Handle> {
    result_return_unless!(name.len() <= 11, result::ResultOutOfRange);

    log_line!("[ConnectToNamedPort] connecting to port: '{}'", name);
    let mut client_port = find_named_object::<KClientPort>(name)?;
    let client_session_handle = get_current_process().get().handle_table.allocate_handle()?;

    let connect_fail_guard = guard((), |()| {
        let _ = get_current_process().get().handle_table.deallocate_handle(client_session_handle);
    });
    let client_session = KClientPort::connect(&mut client_port)?;
    get_current_process().get().handle_table.set_allocated_handle(client_session_handle, client_session.clone())?;

    ScopeGuard::into_inner(connect_fail_guard);
    client_session.get().decrement_refcount();
    Ok(client_session_handle)
}

pub fn send_sync_request(client_session_handle: Handle) -> Result<()> {
    // log_line!("SendSyncRequest with handle {:#X}", client_session_handle);
    let client_session = get_current_process().get().handle_table.get_handle_obj::<KClientSession>(client_session_handle)?;
    
    let rc = client_session.get().send_sync_request(None);
    rc
}

pub fn break_(reason: BreakReason, arg: &[u8]) -> Result<()> {
    if reason.is_notification_only() {
        let actual_reason = reason.without_notification_flag();
        log_line!("[Break] Notified, reason: {:?}", actual_reason);
    }
    else {
        if arg.len() == mem::size_of::<ResultCode>() {
            let rc: ResultCode = util::slice_read_val(arg, None)?;
            panic!("[Break] Reason: {:?}, with result code {1} ({1:?})", reason, rc);
        }
        else {
            panic!("[Break] Reason: {:?}, with arg size {}", reason, arg.len());
        }
    }

    Ok(())
}

pub fn output_debug_string(msg: &str) -> Result<()> {
    log_line!("[OutputDebugString] {}", msg);
    Ok(())
}

pub fn create_session(is_light: bool, name_addr: u64) -> Result<(Handle, Handle)> {
    todo!("CreateSession");
}

pub fn accept_session(server_port_handle: Handle) -> Result<Handle> {
    let server_port = get_current_process().get().handle_table.get_handle_obj::<KServerPort>(server_port_handle)?;

    let server_session_handle = get_current_process().get().handle_table.allocate_handle()?;

    let accept_fail_guard = guard((), |()| {
        let _ = get_current_process().get().handle_table.close_handle(server_session_handle);
    });

    let is_light = server_port.get().is_light;
    match is_light {
        true => match server_port.get().accept_incoming_light_connection() {
            Some(light_server_session) => get_current_process().get().handle_table.set_allocated_handle(server_session_handle, light_server_session)?,
            None => return result::ResultNotFound::make_err()
        },
        false => match server_port.get().accept_incoming_connection() {
            Some(server_session) => get_current_process().get().handle_table.set_allocated_handle(server_session_handle, server_session)?,
            None => return result::ResultNotFound::make_err()
        }
    };

    ScopeGuard::into_inner(accept_fail_guard);
    Ok(server_session_handle)
}

pub fn reply_and_receive(handles: &[Handle], reply_target_session_handle: Handle, timeout: i64) -> Result<usize> {
    result_return_unless!(handles.len() <= 64, result::ResultOutOfRange);

    let mut sync_objs: Vec<Shared<dyn KSynchronizationObject>> = Vec::with_capacity(handles.len());
    for handle in handles {
        let sync_obj = get_current_process().get().handle_table.get_handle_sync_obj(*handle)?;
        // sync_obj.get().increment_refcount();

        sync_objs.push(sync_obj);
    }

    if reply_target_session_handle != INVALID_HANDLE {
        let mut reply_target_session = get_current_process().get().handle_table.get_handle_obj::<KServerSession>(reply_target_session_handle)?;

        KServerSession::reply(&mut reply_target_session, None)?;
    }

    'w: loop {
        let idx = wait_for_sync_objects(&mut sync_objs, timeout)?;
        let server_session = get_current_process().get().handle_table.get_handle_obj::<KServerSession>(handles[idx])?;

        match server_session.get().receive(None) {
            Ok(()) => return Ok(idx),
            Err(rc) => {
                if result::ResultNotFound::matches(rc) {
                    continue 'w;
                }

                return Err(rc);
            }
        };
    }
}

pub fn create_port(max_sessions: u32, is_light: bool, name_addr: u64) -> Result<(Handle, Handle)> {
    let port = KPort::new(max_sessions, is_light, name_addr);

    let server_port_handle = get_current_process().get().handle_table.allocate_handle_set(port.get().server_port.clone())?;

    let alloc_client_handle_fail_guard = guard((), |()| {
        let _ = get_current_process().get().handle_table.close_handle(server_port_handle);
    });

    let client_port_handle = get_current_process().get().handle_table.allocate_handle_set(port.get().client_port.clone())?;

    ScopeGuard::into_inner(alloc_client_handle_fail_guard);
    Ok((server_port_handle, client_port_handle))
}

pub fn manage_named_port(name: &str, max_sessions: u32) -> Result<Handle> {
    result_return_unless!(name.len() <= 11, result::ResultOutOfRange);

    let port = KPort::new(max_sessions, false, 0);

    let server_port_handle = get_current_process().get().handle_table.allocate_handle_set(port.get().server_port.clone())?;
    
    let register_name_fail_guard = guard((), |()| {
        let _ = get_current_process().get().handle_table.close_handle(server_port_handle);
    });

    register_named_object(port.get().client_port.clone(), name)?;

    ScopeGuard::into_inner(register_name_fail_guard);
    Ok(server_port_handle)
}

pub fn connect_to_port(client_port_handle: Handle) -> Result<Handle> {
    let mut client_port = get_current_process().get().handle_table.get_handle_obj::<KClientPort>(client_port_handle)?;
    let client_session_handle = get_current_process().get().handle_table.allocate_handle()?;

    let connect_fail_guard = guard((), |()| {
        let _ = get_current_process().get().handle_table.deallocate_handle(client_session_handle);
    });
    let client_session = KClientPort::connect(&mut client_port)?;
    get_current_process().get().handle_table.set_allocated_handle(client_session_handle, client_session.clone())?;

    ScopeGuard::into_inner(connect_fail_guard);
    client_session.get().decrement_refcount();
    Ok(client_session_handle)
}