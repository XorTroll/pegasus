use std::sync::atomic::AtomicI32;
use std::mem;
use scopeguard::{guard, ScopeGuard};
use super::KAutoObject;
use super::KSynchronizationObject;
use super::proc::KProcess;
use super::thread::KThread;
use super::thread::ThreadState;
use super::thread::get_current_thread;
use super::thread::make_critical_section_guard;
use super::proc::get_current_process;
use crate::ipc::BufferDescriptor;
use crate::ipc::CommandHeader;
use crate::ipc::CommandSpecialHeader;
use crate::ipc::SendStaticDescriptor;
use crate::kern::svc::CURRENT_PROCESS_PSEUDO_HANDLE;
use crate::kern::svc::CURRENT_THREAD_PSEUDO_HANDLE;
use crate::kern::svc::Handle;
use crate::util::Shared;
use crate::util::SharedAny;
use super::svc;
use super::result;
use crate::result::*;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum ChannelState {
    NotInitialized,
    Open,
    ClientDisconnected,
    ServerDisconnected
}

// KPort

pub struct KPort {
    refcount: AtomicI32,
    pub server_port: Shared<KServerPort>,
    pub client_port: Shared<KClientPort>,
    name_addr: u64,
    pub is_light: bool
}

impl KAutoObject for KPort {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

impl KPort {
    pub fn new(max_sessions: u32, is_light: bool, name_addr: u64) -> Shared<Self> {
        let server_port = KServerPort::new(None, is_light);
        let client_port = KClientPort::new(None, max_sessions);

        let port = Shared::new(Self {
            refcount: AtomicI32::new(1),
            server_port: server_port.clone(),
            client_port: client_port.clone(),
            name_addr: name_addr,
            is_light: is_light
        });

        server_port.get().parent = Some(port.clone());
        client_port.get().parent = Some(port.clone());
        port
    }

    pub fn ready_for_drop(&mut self) {
        // Need to do this for the Shareds to actually drop
        self.server_port.get().parent = None;
        self.client_port.get().parent = None;
    }

    #[inline]
    pub fn enqueue_incoming_session(&mut self, session: Shared<KServerSession>) {
        KServerPort::enqueue_incoming_session(&mut self.server_port, session)
    }

    #[inline]
    pub fn enqueue_incoming_light_session(&mut self, session: Shared<KLightServerSession>) {
        KServerPort::enqueue_incoming_light_session(&mut self.server_port, session)
    }
}

impl Drop for KPort {
    fn drop(&mut self) {
        println!("Dropping KPort!");
    }
}

// ---

// KServerPort

pub struct KServerPort {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    pub parent: Option<Shared<KPort>>,
    pub is_light: bool,
    incoming_connections: Vec<Shared<KServerSession>>,
    incoming_light_connections: Vec<Shared<KLightServerSession>>
}

impl KAutoObject for KServerPort {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

impl KSynchronizationObject for KServerPort {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }

    fn is_signaled(&self) -> bool {
        match self.is_light {
            true => !self.incoming_light_connections.is_empty(),
            false => !self.incoming_connections.is_empty()
        }
    }
}

impl KServerPort {
    pub fn new(parent: Option<Shared<KPort>>, is_light: bool) -> Shared<Self> {
        Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            parent: parent,
            is_light: is_light,
            incoming_connections: Vec::new(),
            incoming_light_connections: Vec::new()
        })
    }

    pub fn enqueue_incoming_session(server_port: &mut Shared<KServerPort>, session: Shared<KServerSession>) {
        let _guard = make_critical_section_guard();

        let is_first_session = server_port.get().incoming_connections.is_empty();
        server_port.get().incoming_connections.push(session);

        if is_first_session {
            KSynchronizationObject::signal(server_port);
        }
    }

    pub fn enqueue_incoming_light_session(server_port: &mut Shared<KServerPort>, session: Shared<KLightServerSession>) {
        let _guard = make_critical_section_guard();

        let is_first_light_session = server_port.get().incoming_light_connections.is_empty();
        server_port.get().incoming_light_connections.push(session);

        if is_first_light_session {
            KSynchronizationObject::signal(server_port);
        }
    }

    pub fn accept_incoming_connection(&mut self) -> Option<Shared<KServerSession>> {
        let _guard = make_critical_section_guard();

        let session = match self.incoming_connections.first() {
            Some(session_ref) => Some(session_ref.clone()),
            None => None
        };

        if session.is_some() {
            self.incoming_connections.remove(0);
        }

        session
    }

    pub fn accept_incoming_light_connection(&mut self) -> Option<Shared<KLightServerSession>> {
        let _guard = make_critical_section_guard();

        let session = match self.incoming_light_connections.first() {
            Some(session_ref) => Some(session_ref.clone()),
            None => None
        };

        if session.is_some() {
            self.incoming_light_connections.remove(0);
        }

        session
    }
}

impl Drop for KServerPort {
    fn drop(&mut self) {
        println!("Dropping KServerPort!");
    }
}

// ---

// KClientPort

pub struct KClientPort {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    max_sessions: u32,
    session_count: u32,
    pub parent: Option<Shared<KPort>>
}

impl KAutoObject for KClientPort {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

impl KSynchronizationObject for KClientPort {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }
}

impl KClientPort {
    pub fn new(parent: Option<Shared<KPort>>, max_sessions: u32) -> Shared<Self> {
        Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            max_sessions: max_sessions,
            session_count: 0,
            parent: parent
        })
    }

    pub fn connect(client_port: &mut Shared<KClientPort>) -> Result<Shared<KClientSession>> {
        result_return_unless!(client_port.get().parent.is_some(), result::ResultInvalidState);
        get_current_process().get().resource_limit.get().reserve(svc::LimitableResource::Session, 1, None)?;

        let connect_fail_guard = guard((), |()| {
            get_current_process().get().resource_limit.get().release(svc::LimitableResource::Session, 1, 1);
        });

        let port_session_count = client_port.get().session_count;
        let port_max_sessions = client_port.get().max_sessions;
        result_return_unless!(port_session_count < port_max_sessions, result::ResultOutOfSessions);
        client_port.get().session_count += 1;

        let session = KSession::new(Some(client_port.clone()));
        client_port.get().parent.as_ref().unwrap().get().enqueue_incoming_session(session.get().server_session.clone());

        ScopeGuard::into_inner(connect_fail_guard);
        let client_session = session.get().client_session.clone();
        Ok(client_session)
    }
}

impl Drop for KClientPort {
    fn drop(&mut self) {
        println!("Dropping KClientPort!");
    }
}

// ---

// KSession

pub struct KSession {
    refcount: AtomicI32,
    pub server_session: Shared<KServerSession>,
    pub client_session: Shared<KClientSession>,
    state: ChannelState
}

impl KAutoObject for KSession {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

impl KSession {
    pub fn new(parent_port: Option<Shared<KClientPort>>) -> Shared<Self> {
        let server_session = KServerSession::new(None);
        let client_session = KClientSession::new(None, parent_port);

        let session = Shared::new(Self {
            refcount: AtomicI32::new(1),
            server_session: server_session.clone(),
            client_session: client_session.clone(),
            state: ChannelState::Open
        });

        server_session.get().parent = Some(session.clone());
        client_session.get().parent = Some(session.clone());
        session
    }

    pub fn disconnect_client(&mut self) {
        if self.state == ChannelState::Open {
            self.state = ChannelState::ClientDisconnected;

            self.server_session.get().cancel_all_requests_due_to_client_disconnect();
        }
    }
}

// ---

// KServerSession

struct Message {
    pub buf: *mut u8,
    pub size: usize,
    pub is_custom: bool
}

impl Message {
    pub fn new(thread: &Shared<KThread>, custom_cmd_buf: Option<(u64, usize)>) -> Self {
        let (buf, size) = match custom_cmd_buf {
            Some((custom_addr, custom_size)) => {
                // TODO: get actual ptr through unicorn?
                // (custom_addr as *mut u8, custom_size)
                todo!("Custom UserBuffer IPC requests")
            },
            None => (thread.get().get_tlr_ptr(), 0x100)
        };

        Self {
            buf: buf,
            size: size,
            is_custom: custom_cmd_buf.is_some()
        }
    }

    #[inline]
    pub fn from_request(request: &KSessionRequest) -> Self {
        Self::new(&request.client_thread, request.custom_cmd_buf)
    }

    fn do_write<T: Copy>(&self, offset: isize, t: T) {
        unsafe {
            *(self.buf.offset(offset) as *mut T) = t;
        }
    }

    fn do_read<T: Copy>(&self, offset: isize) -> T {
        unsafe {
            *(self.buf.offset(offset) as *mut T)
        }
    }

    pub fn get_header(&self) -> CommandHeader {
        self.do_read(0)
    }

    pub fn set_header(&self, header: CommandHeader) {
        self.do_write(0, header);
    }

    pub fn get_special_header_offset(&self) -> usize {
        mem::size_of::<CommandHeader>()
    }

    pub fn get_special_header(&self) -> CommandSpecialHeader {
        self.do_read(self.get_special_header_offset() as isize)
    }

    pub fn set_special_header(&self, special_header: CommandSpecialHeader) {
        self.do_write(mem::size_of::<CommandHeader>() as isize, special_header);
    }

    pub fn get_process_id_offset(&self) -> usize {
        self.get_special_header_offset() + mem::size_of::<CommandSpecialHeader>()
    }

    pub fn set_process_id(&self, process_id: u64) {
        self.do_write(self.get_process_id_offset() as isize, process_id);
    }

    fn do_get_array<T: Copy>(&self, base_offset: isize, count: u32) -> Vec<T> {
        let mut ts: Vec<T> = Vec::with_capacity(count as usize);

        for i in 0..count as usize {
            ts.push(self.do_read(base_offset + (i * mem::size_of::<T>()) as isize));
        }

        ts
    }

    fn do_set_array<T: Copy>(&self, base_offset: isize, ts: &Vec<T>) {
        for i in 0..ts.len() {
            self.do_write(base_offset + (i * mem::size_of::<T>()) as isize, ts[i]);
        }
    }

    pub fn get_copy_handles_offset(&self) -> usize {
        let special_header = self.get_special_header();
        self.get_process_id_offset() + match special_header.get_send_process_id() {
            true => mem::size_of::<u64>(),
            false => 0usize
        }
    }
    
    pub fn get_copy_handles(&self) -> Vec<Handle> {
        let special_header = self.get_special_header();

        self.do_get_array(self.get_copy_handles_offset() as isize, special_header.get_copy_handle_count())
    }

    pub fn set_copy_handles(&self, handles: &Vec<Handle>) {
        self.do_set_array(self.get_copy_handles_offset() as isize, handles);
    }

    pub fn get_move_handles_offset(&self) -> usize {
        let special_header = self.get_special_header();
        self.get_copy_handles_offset() + special_header.get_copy_handle_count() as usize * mem::size_of::<Handle>()
    }

    pub fn get_move_handles(&self) -> Vec<Handle> {
        let special_header = self.get_special_header();

        self.do_get_array(self.get_move_handles_offset() as isize, special_header.get_move_handle_count())
    }

    pub fn set_move_handles(&self, handles: &Vec<Handle>) {
        self.do_set_array(self.get_move_handles_offset() as isize, handles);
    }

    pub fn get_send_statics_offset(&self) -> usize {
        let header = self.get_header();
        match header.get_has_special_header() {
            true => {
                let special_header = self.get_special_header();
                self.get_move_handles_offset() + special_header.get_move_handle_count() as usize * mem::size_of::<Handle>()
            },
            false => mem::size_of::<CommandHeader>()
        }
    }

    pub fn get_send_statics(&self) -> Vec<SendStaticDescriptor> {
        let header = self.get_header();

        self.do_get_array(self.get_send_statics_offset() as isize, header.get_send_static_count())
    }

    pub fn get_send_buffers_offset(&self) -> usize {
        let header = self.get_header();
        self.get_send_statics_offset() + header.get_send_static_count() as usize * mem::size_of::<SendStaticDescriptor>()
    }

    pub fn get_receive_buffers_offset(&self) -> usize {
        let header = self.get_header();
        self.get_send_buffers_offset() + header.get_send_buffer_count() as usize * mem::size_of::<BufferDescriptor>()
    }

    pub fn get_exchange_buffers_offset(&self) -> usize {
        let header = self.get_header();
        self.get_receive_buffers_offset() + header.get_receive_buffer_count() as usize * mem::size_of::<BufferDescriptor>()
    }

    pub fn get_raw_data_offset(&self) -> usize {
        let header = self.get_header();
        self.get_exchange_buffers_offset() + header.get_exchange_buffer_count() as usize * mem::size_of::<BufferDescriptor>()
    }

    pub fn get_raw_data(&self) -> Vec<u32> {
        let header = self.get_header();
        self.do_get_array(self.get_raw_data_offset() as isize, header.get_data_word_count())
    }

    pub fn set_raw_data(&self, data: &Vec<u32>) {
        self.do_set_array(self.get_raw_data_offset() as isize, data)
    }

    pub fn get_size(&self) -> usize {
        let header = self.get_header();
        let special_header = self.get_special_header();

        mem::size_of::<CommandHeader>() + header.get_total_size() as usize + match header.get_has_special_header() {
            true => mem::size_of::<CommandSpecialHeader>() + special_header.get_total_size() as usize,
            false => 0usize
        }
    }

    pub fn get_receive_statics(&self) -> Vec<u64> {
        let count = match self.get_header().get_receive_static_count() {
            0xFF => 1,
            c => c
        } as usize;

        let offset = match self.get_header().get_receive_static_offset() {
            0 => self.get_size(),
            o => o as usize
        };

        let mut statics = vec![0u64; count];

        let mut read_ptr = unsafe {
            self.buf.offset(offset as isize) as *mut u64
        };
        for static_v in statics.iter_mut() {
            unsafe {
                *static_v = *read_ptr;
                read_ptr = read_ptr.offset(1);
            }
        }

        statics
    }
}

pub struct KServerSession {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    parent: Option<Shared<KSession>>,
    requests: Vec<KSessionRequest>,
    active_request: Option<KSessionRequest>
}

impl KAutoObject for KServerSession {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }

    fn destroy(&mut self) {
        // _parent.DisconnectServer();
        // CancelAllRequestsServerDisconnected();
        if let Some(session) = self.parent.as_ref() {
            session.get().decrement_refcount();
        }
    }
}

impl KSynchronizationObject for KServerSession {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }

    fn is_signaled(&self) -> bool {
        if let Some(session) = self.parent.as_ref() {
            let client_session_state = session.get().state;
            if client_session_state != ChannelState::Open {
                return true;
            }

            !self.requests.is_empty() && self.active_request.is_none()
        }
        else {
            false
        }
    }
}

impl KServerSession {
    pub fn new(parent: Option<Shared<KSession>>) -> Shared<Self> {
        Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            parent: parent,
            requests: Vec::new(),
            active_request: None
        })
    }

    pub fn cancel_all_requests_due_to_client_disconnect(&self) {
        todo!("cancel_all_requests_due_to_client_disconnect");
    }

    pub fn enqueue_request(server_session: &mut Shared<KServerSession>, mut request: KSessionRequest) -> Result<()> {
        // TODO: check client session state

        /* if async event = None: */
        {
            result_return_if!(request.client_thread.get().is_termination_requested(), result::ResultTerminationRequested);
            KThread::reschedule(&mut request.client_thread, ThreadState::Waiting);
        }
        /* Else, do nothing */

        let is_first_request = server_session.get().requests.is_empty();
        server_session.get().requests.push(request);

        if is_first_request {
            KSynchronizationObject::signal(server_session);
        }

        Ok(())
    }

    fn dequeue_request(&mut self) -> Result<KSessionRequest> {
        let _guard = make_critical_section_guard();

        result_return_if!(self.requests.is_empty(), result::ResultNotFound);

        Ok(self.requests.remove(0))
    }

    fn translate_obj_handle(src_process: &Shared<KProcess>, src_thread: &Shared<KThread>, dst_process: &Shared<KProcess>, src_handle: Handle, is_copy: bool) -> Result<Handle> {
        let obj: SharedAny = match is_copy {
            true => match src_handle {
                CURRENT_PROCESS_PSEUDO_HANDLE => src_process.as_any(),
                CURRENT_THREAD_PSEUDO_HANDLE => src_thread.as_any(),
                _ => src_process.get().handle_table.get_handle_obj_any(src_handle)?
            },
            false => src_process.get().handle_table.get_handle_obj_any(src_handle)?
        };

        let dst_handle = dst_process.get().handle_table.allocate_handle_set_any(obj)?;

        if !is_copy {
            src_process.get().handle_table.close_handle(src_handle)?;
        }

        Ok(dst_handle)
    }

    fn wake_client_thread(request: &mut KSessionRequest, result: ResultCode) {
        /* if async event { ... } */
        /* else */
        {
            let _guard = make_critical_section_guard();

            let state = request.client_thread.get().state.get_low_flags();
            if state == ThreadState::Waiting {
                request.client_thread.get().signaled_obj = None;
                request.client_thread.get().sync_result = result;

                KThread::reschedule(&mut request.client_thread, ThreadState::Runnable);
            }
        }
    }

    fn finish_request(request: &mut KSessionRequest, result: ResultCode) {
        // TODO: unmap buffers

        Self::wake_client_thread(request, result);
    }

    fn do_reply(server_session: &mut Shared<KServerSession>, custom_cmd_buf: Option<(u64, usize)>) -> Result<()> {
        let server_thread = get_current_thread();
        let server_process = get_current_process();

        let (request, client_thread, client_process) = {
            let _guard = make_critical_section_guard();

            result_return_unless!(server_session.get().active_request.is_some(), result::ResultInvalidState);

            let request = server_session.get().active_request.take().unwrap();
            let client_thread = request.client_thread.clone();
            let client_process = client_thread.get().owner_process.as_ref().unwrap().clone();

            let has_any_requests = !server_session.get().requests.is_empty();
            if has_any_requests {
                KSynchronizationObject::signal(server_session);
            }

            (request, client_thread, client_process)
        };

        let client_msg = Message::from_request(&request);
        let server_msg = Message::new(&server_thread, custom_cmd_buf);

        let server_header = server_msg.get_header();
        let server_special_header = server_msg.get_special_header();
        let client_header = client_msg.get_header();
        let client_special_header = client_msg.get_special_header();

        // TODO: check bounds in receive count, etc.

        let server_msg_size = server_msg.get_size();
        let client_msg_size = client_msg.get_size();

        let receive_static_list = server_msg.get_receive_statics();
        client_msg.set_header(server_header);

        if server_header.get_has_special_header() {
            // clientHeader.MoveHandlesCount == 0 here? (...)

            client_msg.set_special_header(client_special_header);

            if server_special_header.get_send_process_id() {
                // TODO
                client_msg.set_process_id(server_process.get().id);
            }

            let mut copy_handles = server_msg.get_copy_handles();
            for handle in copy_handles.iter_mut() {
                let src_handle = *handle;
                *handle = Self::translate_obj_handle(&server_process, &server_thread, &client_process, src_handle, true)?;
            }
            client_msg.set_copy_handles(&copy_handles);

            let mut move_handles = server_msg.get_move_handles();
            for handle in move_handles.iter_mut() {
                let src_handle = *handle;
                *handle = Self::translate_obj_handle(&server_process, &server_thread, &client_process, src_handle, false)?;
            }
            client_msg.set_move_handles(&move_handles);
        }

        // Send statics
        let send_statics = server_msg.get_send_statics();
        for send_static in &send_statics {
            todo!("Send static support");
        }

        // Buffers
        let dummy_count = server_header.get_send_buffer_count() + server_header.get_receive_buffer_count() + server_header.get_exchange_buffer_count();
        if dummy_count > 0 {
            todo!("Buffer support");
        }

        // Raw data
        let raw_data = server_msg.get_raw_data();
        client_msg.set_raw_data(&raw_data);

        // Store again here so that reply(...) can access the request again, dropping it later
        server_session.get().active_request = Some(request);
        Ok(())
    }

    pub fn reply(server_session: &mut Shared<KServerSession>, custom_cmd_buf: Option<(u64, usize)>) -> Result<()> {
        let rc = ResultCode::from(Self::do_reply(server_session, custom_cmd_buf));
        let mut request = server_session.get().active_request.take().unwrap();

        Self::finish_request(&mut request, rc);
        Ok(())
    }

    pub fn receive(&mut self, custom_cmd_buf: Option<(u64, usize)>) -> Result<()> {
        let server_thread = get_current_thread();
        let server_process = get_current_process();

        let (request, client_thread, client_process) = {
            let _guard = make_critical_section_guard();

            result_return_unless!(self.active_request.is_none(), result::ResultNotFound);

            let request = self.dequeue_request()?;
            let client_thread = request.client_thread.clone();
            let client_process = client_thread.get().owner_process.as_ref().unwrap().clone();

            (request, client_thread, client_process)
        };

        let client_msg = Message::from_request(&request);
        let server_msg = Message::new(&server_thread, custom_cmd_buf);

        let server_header = server_msg.get_header();
        let server_special_header = server_msg.get_special_header();
        let client_header = client_msg.get_header();
        let client_special_header = client_msg.get_special_header();

        // TODO: check bounds in receive count, etc.

        let server_msg_size = server_msg.get_size();
        let client_msg_size = client_msg.get_size();

        let receive_static_list = server_msg.get_receive_statics();
        server_msg.set_header(client_header);

        if client_header.get_has_special_header() {
            // clientHeader.MoveHandlesCount == 0 here? (...)

            server_msg.set_special_header(client_special_header);

            if client_special_header.get_send_process_id() {
                // TODO
                server_msg.set_process_id(client_process.get().id);
            }

            let mut copy_handles = client_msg.get_copy_handles();
            for handle in copy_handles.iter_mut() {
                let src_handle = *handle;
                *handle = Self::translate_obj_handle(&client_process, &client_thread, &server_process, src_handle, true)?;
            }
            server_msg.set_copy_handles(&copy_handles);

            let mut move_handles = client_msg.get_move_handles();
            for handle in move_handles.iter_mut() {
                let src_handle = *handle;
                *handle = Self::translate_obj_handle(&client_process, &client_thread, &server_process, src_handle, false)?;
            }
            server_msg.set_move_handles(&move_handles);
        }

        // Send statics
        let send_statics = client_msg.get_send_statics();
        for send_static in &send_statics {
            todo!("Send static support");
        }

        // Buffers
        let dummy_count = client_header.get_send_buffer_count() + client_header.get_receive_buffer_count() + client_header.get_exchange_buffer_count();
        if dummy_count > 0 {
            todo!("Buffer support");
        }

        // Raw data
        let raw_data = client_msg.get_raw_data();
        server_msg.set_raw_data(&raw_data);

        // TODO: unmap buffers?

        self.active_request = Some(request);
        Ok(())
    }
}

// ---

// KClientSession

pub struct KClientSession {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    parent: Option<Shared<KSession>>,
    parent_port: Option<Shared<KClientPort>>
}

impl KAutoObject for KClientSession {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }

    fn destroy(&mut self) {
        if let Some(session) = self.parent.as_ref() {
            session.get().disconnect_client();
            session.get().decrement_refcount();
        }
    }
}

impl KSynchronizationObject for KClientSession {
    fn get_waiting_threads(&mut self) -> &mut Vec<Shared<KThread>> {
        &mut self.waiting_threads
    }
}

impl KClientSession {
    pub fn new(parent: Option<Shared<KSession>>, parent_port: Option<Shared<KClientPort>>) -> Shared<Self> {
        if let Some(port) = parent_port.as_ref() {
            port.get().increment_refcount();
        }

        get_current_process().get().increment_refcount();

        Shared::new(Self {
            refcount: AtomicI32::new(1),
            waiting_threads: Vec::new(),
            parent: parent,
            parent_port: parent_port
        })
    }

    pub fn send_sync_request(&mut self, custom_cmd_buf: Option<(u64, usize)>) -> Result<()> {
        let request = KSessionRequest::new(get_current_thread(), custom_cmd_buf);

        {
            let _guard = make_critical_section_guard();

            get_current_thread().get().signaled_obj = None;
            get_current_thread().get().sync_result = ResultSuccess::make();

            let mut server_session = self.parent.as_ref().unwrap().get().server_session.clone();
            KServerSession::enqueue_request(&mut server_session, request)?;
        }

        get_current_thread().get().sync_result.to(())
    }
}

// ---

// KLightSession

pub struct KLightSession {
    refcount: AtomicI32
}

impl KAutoObject for KLightSession {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

// ---

// KLightServerSession

pub struct KLightServerSession {
    refcount: AtomicI32
}

impl KAutoObject for KLightServerSession {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

// ---

// KLightClientSession

pub struct KLightClientSession {
    refcount: AtomicI32
}

impl KAutoObject for KLightClientSession {
    fn get_refcount(&mut self) -> &mut AtomicI32 {
        &mut self.refcount
    }
}

// ---

// KSessionRequest

pub struct KSessionRequest {
    pub client_thread: Shared<KThread>,
    pub custom_cmd_buf: Option<(u64, usize)>
}

impl KSessionRequest {
    pub fn new(client_thread: Shared<KThread>, custom_cmd_buf: Option<(u64, usize)>) -> Self {
        Self {
            client_thread: client_thread,
            custom_cmd_buf: custom_cmd_buf
        }
    }
}

// ---