use std::sync::atomic::AtomicI32;
use scopeguard::{guard, ScopeGuard};
use super::KAutoObject;
use super::KSynchronizationObject;
use super::thread::KThread;
use super::thread::ThreadState;
use super::thread::get_current_thread;
use super::thread::make_critical_section_guard;
use super::proc::get_current_process;
use crate::util::Shared;
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
    server_session: Shared<KServerSession>,
    client_session: Shared<KClientSession>,
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

pub struct KServerSession {
    refcount: AtomicI32,
    waiting_threads: Vec<Shared<KThread>>,
    parent: Option<Shared<KSession>>,
    requests: Vec<KSessionRequest>,
    active_request: Option<Shared<KSessionRequest>>
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

    pub fn reply(&mut self, custom_cmd_buf: Option<(u64, usize)>) -> Result<()> {
        todo!("reply");
    }

    pub fn receive(&mut self, custom_cmd_buf: Option<(u64, usize)>) -> Result<()> {
        todo!("receive");
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

    pub fn send_sync_request(&mut self, custom_cmd_buf_addr: Option<u64>, custom_cmd_buf_size: Option<usize>) -> Result<()> {
        let request = KSessionRequest::new(get_current_thread(), custom_cmd_buf_addr, custom_cmd_buf_size);

        {
            let _guard = make_critical_section_guard();

            get_current_thread().get().signaled_obj = None;
            get_current_thread().get().sync_result = ResultSuccess::make();

            let mut server_session = self.parent.as_ref().unwrap().get().server_session.clone();
            KServerSession::enqueue_request(&mut server_session, request)?;

            log_line!("Leaving KCriticalSection!");
        }
        log_line!("Left KCriticalSection!");

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
    pub custom_cmd_buf_addr: Option<u64>,
    pub custom_cmd_buf_size: Option<usize>
}

impl KSessionRequest {
    pub fn new(client_thread: Shared<KThread>, custom_cmd_buf_addr: Option<u64>, custom_cmd_buf_size: Option<usize>) -> Self {
        Self {
            client_thread: client_thread,
            custom_cmd_buf_addr: custom_cmd_buf_addr,
            custom_cmd_buf_size: custom_cmd_buf_size
        }
    }
}

// ---