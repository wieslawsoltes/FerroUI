//! UI-thread coordination for the reusable room service. Workers own all transport, credentials
//! and disk I/O; the session remains the only authority that mutates authored/preview state.
use crate::DesignerSession;
use ferroui_designer_collaboration::{Cursor, DocumentReplica, SharedEdit, VersionVector};
use ferroui_designer_collaboration_host as host;
use host::{
    Credentials, Exchange, ExchangeReply, HostedServer, Invitation, JoinedRoom, Member,
    PeerCheckpoint, PeerTransport, Request, Role, RoomState, Secret,
};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct SharingOptions {
    pub directory: PathBuf,
    pub display_name: String,
    pub bind: Option<SocketAddr>,
    /// HTTPS endpoint advertised in invitations when a TLS proxy fronts the local service.
    pub public_url: Option<String>,
}
impl Default for SharingOptions {
    fn default() -> Self {
        let root = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("XDG_STATE_HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir)
                    .join(".local/state")
            });
        Self {
            directory: root.join("FerroUI/Designer/Sharing"),
            display_name: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "Designer".into()),
            bind: None,
            public_url: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharingPhase {
    Connecting,
    AwaitingReview,
    Connected,
    Reconnecting,
    Conflict,
    Disconnected,
}
#[derive(Clone, Debug)]
pub struct SharingStatus {
    pub phase: SharingPhase,
    pub message: String,
    pub role: Option<Role>,
    pub members: Vec<Member>,
    pub pending_operations: usize,
    pub invitation: Option<String>,
    pub backup_path: Option<PathBuf>,
}
#[derive(Clone, Serialize, Deserialize)]
struct NativeCheckpoint {
    peer: PeerCheckpoint,
    source: String,
    uri: String,
    hosted_address: Option<SocketAddr>,
    hosted_directory: Option<PathBuf>,
}
#[derive(Clone)]
struct Pending {
    joined: JoinedRoom,
    /// The exact pre-review replica and source draft, archived before accepting a new baseline.
    checkpoint: NativeCheckpoint,
    /// Server-acknowledged operations for the target snapshot, independent of the old outbox.
    acknowledged_version: VersionVector,
    has_saved_draft: bool,
}
#[derive(Serialize, Deserialize)]
struct PendingJoin {
    endpoint: host::Endpoint,
    request: host::JoinRequest,
}
enum Start {
    Host,
    Join(Invitation),
    Resume,
}
enum Command {
    Sync {
        exchange: Exchange,
        checkpoint: NativeCheckpoint,
    },
    Invite(Role),
    Member {
        actor: String,
        role: Role,
        revoked: bool,
    },
    Checkpoint {
        epoch: u64,
        version: VersionVector,
        resolved_source: String,
    },
    Refresh,
    Accept {
        pending: Pending,
        revision: u64,
        local_source: String,
        resolved: Option<String>,
        uri: String,
    },
    CommitAccept(NativeCheckpoint),
    Stop(Option<NativeCheckpoint>),
}
enum Event {
    Ready {
        pending: Box<Pending>,
        automatic: bool,
    },
    Synced(ExchangeReply),
    Invited(String),
    State(RoomState),
    Accepted {
        pending: Box<Pending>,
        reviewed: Box<Pending>,
        revision: u64,
    },
    AcceptedCommitted,
    Error {
        error: host::HostError,
        state: Option<RoomState>,
    },
    Stopped(host::Result<()>),
}

/// Retain this controller while sharing. Call `poll` on the UI thread (the stock workspace does
/// so from its existing timer). A document/tab change detaches the old room before any reply is
/// applied. All operations are revision-checked and leave source drafts intact on failure.
pub struct SharingController {
    session: Weak<RefCell<DesignerSession>>,
    commands: mpsc::Sender<Command>,
    events: RefCell<mpsc::Receiver<Event>>,
    status: RefCell<SharingStatus>,
    pending: RefCell<Option<Pending>>,
    active: RefCell<Option<NativeCheckpoint>>,
    busy: Cell<bool>,
    next_sync: Cell<Instant>,
    failures: Cell<u32>,
    authorization_blocked: Cell<bool>,
    durability_blocked: Cell<bool>,
    disconnect_requested: Cell<bool>,
    display_name: String,
    bound_uri: String,
    starting_revision: u64,
    directory: PathBuf,
    cursor: RefCell<Option<Cursor>>,
}
impl SharingController {
    pub fn host(
        session: &Rc<RefCell<DesignerSession>>,
        options: SharingOptions,
    ) -> Result<Rc<Self>, String> {
        Self::start(session, options, Start::Host)
    }
    pub fn join(
        session: &Rc<RefCell<DesignerSession>>,
        invitation: String,
        options: SharingOptions,
    ) -> Result<Rc<Self>, String> {
        let invitation = Invitation::decode(&invitation).map_err(|e| e.to_string())?;
        Self::start(session, options, Start::Join(invitation))
    }
    pub fn resume(
        session: &Rc<RefCell<DesignerSession>>,
        options: SharingOptions,
    ) -> Result<Rc<Self>, String> {
        Self::start(session, options, Start::Resume)
    }
    fn start(
        session: &Rc<RefCell<DesignerSession>>,
        options: SharingOptions,
        start: Start,
    ) -> Result<Rc<Self>, String> {
        if let Some(url) = &options.public_url {
            host::validate_public_endpoint_url(url).map_err(|error| error.to_string())?;
        }
        if options
            .bind
            .is_some_and(|address| !address.ip().is_loopback())
        {
            return Err("The sharing service must bind to a loopback address; use a TLS proxy for public access".into());
        }
        let current = session
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        if current.collaboration_version().is_some() {
            return Err("Leave the current room before starting another sharing session".into());
        }
        if matches!(start, Start::Host) && !current.document().is_valid() {
            return Err("Repair the XAML before hosting a room".into());
        }
        if options.display_name.is_empty()
            || options.display_name.len() > 256
            || options.display_name.chars().any(char::is_control)
        {
            return Err("Display name must be 1..256 non-control UTF-8 bytes".into());
        }
        let source = current.document().source().to_owned();
        let uri = current.document_uri().to_owned();
        let revision = current.document().revision();
        drop(current);
        let (commands, receiver) = mpsc::channel();
        let (sender, events) = mpsc::channel();
        let worker_options = options.clone();
        let worker_uri = uri.clone();
        std::thread::Builder::new()
            .name("FerroUI designer sharing".into())
            .spawn(move || worker(start, worker_options, source, worker_uri, receiver, sender))
            .map_err(|e| e.to_string())?;
        Ok(Rc::new(Self {
            session: Rc::downgrade(session),
            commands,
            events: RefCell::new(events),
            status: RefCell::new(SharingStatus {
                phase: SharingPhase::Connecting,
                message: "Connecting…".into(),
                role: None,
                members: vec![],
                pending_operations: 0,
                invitation: None,
                backup_path: None,
            }),
            pending: RefCell::new(None),
            active: RefCell::new(None),
            busy: Cell::new(true),
            next_sync: Cell::new(Instant::now()),
            failures: Cell::new(0),
            authorization_blocked: Cell::new(false),
            durability_blocked: Cell::new(false),
            disconnect_requested: Cell::new(false),
            display_name: options.display_name,
            bound_uri: uri,
            starting_revision: revision,
            directory: options.directory,
            cursor: RefCell::new(None),
        }))
    }
    pub fn status(&self) -> SharingStatus {
        self.status.borrow().clone()
    }
    /// Cursor coordinates are artboard-local logical units, independent of viewport zoom/scroll.
    pub fn set_cursor(&self, cursor: Option<Cursor>) -> Result<(), String> {
        if cursor
            .as_ref()
            .is_some_and(|position| !position.x.is_finite() || !position.y.is_finite())
        {
            return Err("Cursor coordinates must be finite".into());
        }
        *self.cursor.borrow_mut() = cursor;
        Ok(())
    }
    pub fn pending_source(&self) -> Option<String> {
        let pending = self.pending.borrow();
        let pending = pending.as_ref()?;
        DocumentReplica::from_snapshot(
            pending.joined.credentials.actor.clone(),
            pending.joined.state.snapshot.clone(),
        )
        .ok()
        .map(|r| r.source().to_owned())
    }
    /// The saved offline draft offered as a resolution when resuming after a restart.
    pub fn pending_local_source(&self) -> Option<String> {
        self.pending
            .borrow()
            .as_ref()
            .filter(|pending| pending.has_saved_draft)
            .map(|pending| pending.checkpoint.source.clone())
    }
    pub fn invite(&self, role: Role) -> Result<(), String> {
        if role == Role::Owner {
            return Err("Invite an editor or viewer".into());
        }
        self.owner_command(Command::Invite(role))
    }
    pub fn set_member(&self, actor: String, role: Role, revoked: bool) -> Result<(), String> {
        self.owner_command(Command::Member {
            actor,
            role,
            revoked,
        })
    }
    pub fn checkpoint(&self) -> Result<(), String> {
        let session = self.session.upgrade().ok_or("Designer session closed")?;
        let session = session
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        if !self.owns_session(&session) {
            return Err("This controller no longer owns the collaboration session".into());
        }
        let version = session.collaboration_version().ok_or("No active room")?;
        let active = self.active.borrow();
        let active = active.as_ref().ok_or("No active room")?;
        if version != active.peer.server_version {
            return Err("Synchronize all pending edits before checkpointing".into());
        }
        let resolved_source = session.document().source().to_owned();
        self.owner_command(Command::Checkpoint {
            epoch: active.peer.epoch,
            version,
            resolved_source,
        })
    }
    pub fn refresh_room(&self) -> Result<(), String> {
        self.send(Command::Refresh)
    }
    fn owner_command(&self, command: Command) -> Result<(), String> {
        if self.status.borrow().role != Some(Role::Owner) {
            return Err("Only the room owner can manage sharing".into());
        }
        let session = self.session.upgrade().ok_or("Designer session closed")?;
        let session = session
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        if !self.owns_session(&session) {
            return Err("This controller no longer owns the collaboration session".into());
        }
        self.send(command)
    }
    fn send(&self, command: Command) -> Result<(), String> {
        if self.durability_blocked.get() {
            return Err("Sharing durability is uncertain; stop and resume the saved session before continuing".into());
        }
        if self.busy.get() {
            return Err("A sharing operation is already in progress".into());
        }
        self.commands
            .send(command)
            .map_err(|_| "Sharing worker stopped".to_owned())?;
        self.busy.set(true);
        Ok(())
    }
    /// The stock UI presents the exact remote source and a resolution field before this call.
    /// `None` restores the saved session draft; pass `Some(pending_source())` to explicitly adopt
    /// the remote document instead. A revision pin prevents applying that decision if the local
    /// document changes during I/O.
    pub fn accept_snapshot(
        &self,
        expected_revision: u64,
        resolved_source: Option<String>,
    ) -> Result<(), String> {
        let session = self.session.upgrade().ok_or("Designer session closed")?;
        let session = session
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        if session.document().revision() != expected_revision || !self.may_attach(&session) {
            return Err("The current document changed; review the room source again".into());
        }
        let mut pending = self
            .pending
            .borrow()
            .clone()
            .ok_or("No room snapshot awaiting review")?;
        if self.owns_session(&session) {
            pending.checkpoint = self
                .checkpoint_for_session(&session)
                .ok_or("Sharing detached")?;
            pending.has_saved_draft = true;
        }
        if let Some(source) = &resolved_source {
            ferroui_designer_core::Document::new(source.clone()).map_err(|e| e.to_string())?;
        }
        self.send(Command::Accept {
            pending,
            revision: expected_revision,
            local_source: session.document().source().into(),
            resolved: resolved_source,
            uri: self.bound_uri.clone(),
        })?;
        self.status.borrow_mut().message =
            "Saving the local backup and reviewed resolution…".into();
        Ok(())
    }
    pub fn disconnect(&self) {
        let owner = self.session.upgrade();
        let mut session = match owner.as_ref().map(|owner| owner.try_borrow_mut()) {
            Some(Ok(session)) => Some(session),
            Some(Err(_)) => {
                // A preview callback may request disconnection while a document transaction
                // owns the session. Capture its completed outbox on the next poll, not now.
                self.disconnect_requested.set(true);
                return;
            }
            None => None,
        };
        let checkpoint = session
            .as_deref()
            .and_then(|session| self.checkpoint_for_session(session));
        let queued = self.commands.send(Command::Stop(checkpoint)).is_ok();
        if let Some(session) = session.as_mut() {
            if self.owns_session(session) {
                session.leave_collaboration();
            }
        }
        self.active.borrow_mut().take();
        self.pending.borrow_mut().take();
        self.busy.set(false);
        self.disconnect_requested.set(false);
        let mut status = self.status.borrow_mut();
        status.phase = SharingPhase::Disconnected;
        status.message = if queued{"Sharing stopped; saving the final offline checkpoint."}else{"Sharing stopped. The worker was unavailable; save the current document to retain the latest edits."}.into();
        status.role = None;
    }
    fn owns_session(&self, session: &DesignerSession) -> bool {
        self.active.borrow().as_ref().is_some_and(|active| {
            session.collaboration_actor() == Some(active.peer.credentials.actor.as_str())
                && session.collaboration_document_id()
                    == Some(active.peer.snapshot.document_id.as_str())
        })
    }
    fn may_attach(&self, session: &DesignerSession) -> bool {
        session.document_uri() == self.bound_uri
            && if self.active.borrow().is_some() {
                self.owns_session(session)
            } else {
                session.collaboration_actor().is_none()
            }
    }
    fn checkpoint_for_session(&self, session: &DesignerSession) -> Option<NativeCheckpoint> {
        if self.durability_blocked.get() {
            return None;
        }
        let mut checkpoint = self.active.borrow().clone()?;
        if !self.owns_session(session) {
            return None;
        }
        checkpoint.peer.snapshot = session.collaboration_snapshot().ok()?;
        checkpoint.source = session.document().source().into();
        checkpoint.uri = session.document_uri().into();
        Some(checkpoint)
    }
    fn capture_checkpoint(&self) -> Option<NativeCheckpoint> {
        let session = self.session.upgrade()?;
        let session = session.try_borrow().ok()?;
        self.checkpoint_for_session(&session)
    }
    fn attach(
        &self,
        pending: Pending,
        expected_revision: u64,
        replace: bool,
    ) -> Result<(), String> {
        let session = self.session.upgrade().ok_or("Designer session closed")?;
        let mut session = session
            .try_borrow_mut()
            .map_err(|_| "Designer is already processing a transaction")?;
        if !self.may_attach(&session) {
            drop(session);
            self.disconnect();
            return Err("The collaboration session changed while sharing connected".into());
        }
        if session.document().revision() != expected_revision
            || session.document_uri() != self.bound_uri
        {
            // An Accepted candidate may already contain the proposed resolution. Keep the
            // original server snapshot as the review's remote side when its local CAS fails.
            if self.pending.borrow().is_none() {
                *self.pending.borrow_mut() = Some(pending);
            }
            self.status.borrow_mut().phase = SharingPhase::AwaitingReview;
            return Err("The local document changed while sharing connected; review before opening the shared source".into());
        }
        let replica = DocumentReplica::from_snapshot(
            pending.joined.credentials.actor.clone(),
            pending.joined.state.snapshot.clone(),
        )
        .map_err(|e| e.to_string())?;
        if !replace && session.document().source() != replica.source() {
            *self.pending.borrow_mut() = Some(pending);
            self.status.borrow_mut().phase = SharingPhase::AwaitingReview;
            return Ok(());
        }
        session.leave_collaboration();
        if session.document().source() != replica.source() {
            session.open_document(replica.source().into(), self.bound_uri.clone())?;
        }
        session.join_collaboration(
            pending.joined.credentials.actor.clone(),
            pending.joined.state.snapshot.clone(),
        )?;
        // A restored invalid/source-conflict draft is kept separate from the replicated graph.
        if pending.checkpoint.source != replica.source()
            && pending.joined.state.role != Role::Viewer
        {
            session.update_source(pending.checkpoint.source.clone());
        }
        session.set_read_only(pending.joined.state.role == Role::Viewer);
        let mut status = self.status.borrow_mut();
        status.phase = SharingPhase::Connected;
        status.message = "Connected. Changes are saved and synchronized automatically.".into();
        status.role = Some(pending.joined.state.role);
        status.members = pending.joined.state.members.clone();
        status.backup_path = Some(self.directory.join("previous-document.json"));
        *self.active.borrow_mut() = Some(pending.checkpoint);
        self.pending.borrow_mut().take();
        self.next_sync.set(Instant::now());
        Ok(())
    }
    fn receive_room_state(&self, state: RoomState) -> Result<(), String> {
        let session = self.session.upgrade().ok_or("Designer session closed")?;
        let mut session = session
            .try_borrow_mut()
            .map_err(|_| "Designer is already processing a transaction")?;
        if !self.may_attach(&session) {
            return Err("This controller no longer owns the collaboration session".into());
        }
        let active = self.active.borrow().clone();
        let Some(active) = active else {
            let mut pending = self.pending.borrow_mut();
            if let Some(pending) = pending.as_mut() {
                if state.room != pending.joined.credentials.room {
                    return Err("Room state belongs to another room".into());
                }
                let replica = DocumentReplica::from_snapshot(
                    pending.joined.credentials.actor.clone(),
                    state.snapshot.clone(),
                )
                .map_err(|error| error.to_string())?;
                pending.acknowledged_version = replica.version();
                pending.joined.state = state.clone();
                let mut status = self.status.borrow_mut();
                status.role = Some(state.role);
                status.members = state.members;
                status.phase = SharingPhase::AwaitingReview;
                self.authorization_blocked.set(false);
            }
            return Ok(());
        };
        if state.room != active.peer.credentials.room
            || state.snapshot.document_id != active.peer.snapshot.document_id
        {
            return Err("Room state belongs to another collaboration document".into());
        }
        let remote = DocumentReplica::from_snapshot(
            active.peer.credentials.actor.clone(),
            state.snapshot.clone(),
        )
        .map_err(|error| error.to_string())?;
        let checkpoint = self
            .checkpoint_for_session(&session)
            .ok_or("Sharing detached")?;
        let local = DocumentReplica::from_snapshot(
            active.peer.credentials.actor.clone(),
            checkpoint.peer.snapshot.clone(),
        )
        .map_err(|error| error.to_string())?;
        let unsent = local
            .pending_operation_count(&active.peer.server_version, &active.peer.credentials.actor)
            > 0;
        let draft = checkpoint.source != local.source();
        let review = state.epoch != active.peer.epoch
            || self.pending.borrow().is_some()
            || (state.role == Role::Viewer && (unsent || draft));
        // The role fence applies immediately, even while the old authored draft is retained
        // for review. A viewer never resubmits a rejected editor outbox in a retry loop.
        session.set_read_only(state.role == Role::Viewer);
        self.authorization_blocked.set(false);
        let mut status = self.status.borrow_mut();
        status.role = Some(state.role);
        status.members = state.members.clone();
        if review {
            *self.pending.borrow_mut() = Some(Pending {
                joined: JoinedRoom {
                    credentials: active.peer.credentials,
                    state,
                },
                checkpoint,
                acknowledged_version: remote.version(),
                has_saved_draft: true,
            });
            status.phase = SharingPhase::AwaitingReview;
            status.message = if status.role == Some(Role::Viewer) {
                "Your role is now viewer. Your unsent operations and source draft are retained; review and adopt the remote snapshot to continue."
            } else {
                "The room has a new checkpoint. Review the remote source and resolve your retained local edits before rejoining."
            }.into();
        } else {
            if let Some(active) = self.active.borrow_mut().as_mut() {
                active.peer.role = state.role;
            }
            self.next_sync.set(Instant::now());
        }
        Ok(())
    }
    fn receive_error(&self, error: host::HostError) {
        if error.code == host::ErrorCode::Indeterminate {
            self.durability_blocked.set(true);
        }
        self.failures.set(self.failures.get().saturating_add(1));
        self.next_sync.set(
            Instant::now()
                + Duration::from_millis((250u64 << self.failures.get().min(7)).min(30_000)),
        );
        if matches!(
            error.code,
            host::ErrorCode::Unauthorized
                | host::ErrorCode::Forbidden
                | host::ErrorCode::EpochChanged
                | host::ErrorCode::Indeterminate
        ) {
            self.authorization_blocked.set(true);
            if matches!(
                error.code,
                host::ErrorCode::Unauthorized
                    | host::ErrorCode::Forbidden
                    | host::ErrorCode::Indeterminate
            ) {
                if let Some(session) = self.session.upgrade() {
                    if let Ok(mut session) = session.try_borrow_mut() {
                        if self.owns_session(&session) {
                            session.set_read_only(true);
                        }
                    };
                }
            }
        }
        let mut status = self.status.borrow_mut();
        status.phase = if self.active.borrow().is_none() && self.pending.borrow().is_none() {
            SharingPhase::Disconnected
        } else if matches!(
            error.code,
            host::ErrorCode::Unauthorized
                | host::ErrorCode::Forbidden
                | host::ErrorCode::EpochChanged
                | host::ErrorCode::Conflict
                | host::ErrorCode::Indeterminate
        ) {
            SharingPhase::Conflict
        } else {
            SharingPhase::Reconnecting
        };
        status.message = error.to_string();
    }
    pub fn poll(&self) -> Result<bool, String> {
        if self.disconnect_requested.get() {
            self.disconnect();
            if self.disconnect_requested.get() {
                return Ok(false);
            }
        }
        if self.status.borrow().phase == SharingPhase::Disconnected {
            let mut changed = false;
            for event in self.events.borrow().try_iter() {
                if let Event::Stopped(result) = event {
                    self.status.borrow_mut().message = match result {
                        Ok(()) => "Sharing stopped. The final offline checkpoint is saved.".into(),
                        Err(error) => format!(
                            "Sharing stopped, but saving the final checkpoint failed: {error}. Save the current document."
                        ),
                    };
                    changed = true;
                }
            }
            return Ok(changed);
        }
        let Some(session) = self.session.upgrade() else {
            self.disconnect();
            return Ok(true);
        };
        let attached = match session.try_borrow_mut() {
            Ok(session) => self.may_attach(&session),
            // Reentrant preview callbacks leave queued replies untouched until the current
            // transaction completes; a RefCell panic must not discard the drained events.
            Err(_) => return Ok(false),
        };
        if !attached {
            self.disconnect();
            return Ok(true);
        }
        let events: Vec<_> = self.events.borrow().try_iter().collect();
        let mut changed = !events.is_empty();
        for event in events {
            self.busy.set(false);
            match event {
                Event::Ready { pending, automatic } => {
                    if automatic && session.borrow().document().revision() == self.starting_revision
                    {
                        self.attach(*pending, self.starting_revision, false)?;
                    } else {
                        let mut status = self.status.borrow_mut();
                        status.phase = SharingPhase::AwaitingReview;
                        status.role = Some(pending.joined.state.role);
                        status.members = pending.joined.state.members.clone();
                        status.message="Review the room source before opening it. Your local document has been backed up.".into();
                        status.backup_path = Some(self.directory.join("previous-document.json"));
                        *self.pending.borrow_mut() = Some(*pending);
                    }
                }
                Event::Accepted {
                    pending,
                    reviewed,
                    revision,
                } => {
                    *self.pending.borrow_mut() = Some(*reviewed);
                    self.attach(*pending, revision, true)?;
                    let checkpoint = self.active.borrow().clone().ok_or("Sharing detached")?;
                    self.send(Command::CommitAccept(checkpoint))?;
                    let mut status = self.status.borrow_mut();
                    status.phase = SharingPhase::Connecting;
                    status.message =
                        "Opening the reviewed source; saving its accepted checkpoint…".into();
                }
                Event::AcceptedCommitted => {
                    self.status.borrow_mut().phase = SharingPhase::Connected;
                    self.status.borrow_mut().message =
                        "Connected. The reviewed checkpoint is saved.".into();
                    self.next_sync.set(Instant::now());
                }
                Event::Synced(reply) => {
                    let mut active = self.active.borrow_mut();
                    let active = active.as_mut().ok_or("Room closed while syncing")?;
                    if reply.epoch != active.peer.epoch {
                        return Err("Room epoch changed unexpectedly".into());
                    }
                    let mut session = session.borrow_mut();
                    session
                        .merge_collaboration(reply.message)
                        .map_err(|e| e.to_string())?;
                    for presence in reply.presence {
                        if presence.actor != active.peer.credentials.actor {
                            session.receive_presence_now(presence)?;
                        }
                    }
                    active.peer.server_version = reply.version;
                    active.peer.role = reply.role;
                    session.set_read_only(reply.role == Role::Viewer);
                    let mut status = self.status.borrow_mut();
                    status.role = Some(reply.role);
                    status.phase =
                        if session.collaboration_error().is_some() || !reply.conflicts.is_empty() {
                            SharingPhase::Conflict
                        } else {
                            SharingPhase::Connected
                        };
                    status.message = session
                        .collaboration_error()
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            if reply.conflicts.is_empty() {
                                "Connected. Changes are durably synchronized.".into()
                            } else {
                                format!(
                                    "{} operation conflicts need review; local source is retained.",
                                    reply.conflicts.len()
                                )
                            }
                        });
                    self.failures.set(0);
                    self.authorization_blocked.set(false);
                    self.next_sync
                        .set(Instant::now() + Duration::from_millis(350));
                }
                Event::Invited(invitation) => {
                    self.status.borrow_mut().invitation = Some(invitation);
                    self.status.borrow_mut().message =
                        "Invitation ready. Copy it to the people you want to join.".into();
                }
                Event::State(state) => self.receive_room_state(state)?,
                Event::Stopped(result) => {
                    self.status.borrow_mut().phase = SharingPhase::Disconnected;
                    if let Err(error) = result {
                        self.status.borrow_mut().message = error.to_string();
                    }
                }
                Event::Error { error, state } => {
                    self.receive_error(error);
                    if let Some(state) = state {
                        self.receive_room_state(state)?;
                    }
                }
            }
            if self.disconnect_requested.get() {
                self.disconnect();
                return Ok(true);
            }
        }
        if !self.durability_blocked.get()
            && !self.busy.get()
            && self.pending.borrow().is_none()
            && self.active.borrow().is_some()
            && Instant::now() >= self.next_sync.get()
        {
            if self.authorization_blocked.get() {
                self.send(Command::Refresh)?;
                return Ok(true);
            }
            let mut session = session.borrow_mut();
            let mut checkpoint = self.active.borrow().clone().unwrap();
            let message = session.collaboration_outbox_page(&checkpoint.peer.server_version)?;
            self.status.borrow_mut().pending_operations =
                session.collaboration_pending_operations(&checkpoint.peer.server_version)?;
            checkpoint.peer.snapshot = session.collaboration_snapshot()?;
            checkpoint.source = session.document().source().into();
            let known = session.collaboration_version().ok_or("Sharing detached")?;
            let presence =
                session.local_presence(self.display_name.clone(), self.cursor.borrow().clone())?;
            let exchange = Exchange {
                epoch: checkpoint.peer.epoch,
                known,
                message,
                presence: Some(presence),
            };
            self.send(Command::Sync {
                exchange,
                checkpoint,
            })?;
            changed = true;
        }
        Ok(changed)
    }
}
impl Drop for SharingController {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop(self.capture_checkpoint()));
    }
}

struct Worker {
    peer: host::McpPeer,
    credentials: Credentials,
    checkpoint: NativeCheckpoint,
    invitation_endpoint: host::Endpoint,
    _server: Option<HostedServer>,
}
fn private_directory(path: &Path) -> host::Result<()> {
    std::fs::create_dir_all(path)
        .map_err(|e| host::HostError::new(host::ErrorCode::Storage, e.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| host::HostError::new(host::ErrorCode::Storage, e.to_string()))?;
    }
    Ok(())
}
fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> host::Result<T> {
    serde_json::from_value(value)
        .map_err(|e| host::HostError::new(host::ErrorCode::Invalid, e.to_string()))
}
fn prepare_accept(
    mut pending: Pending,
    local_source: String,
    resolved: Option<String>,
    uri: String,
    directory: &Path,
) -> host::Result<Pending> {
    host::save_private(
        &directory.join("previous-sharing-checkpoint.json"),
        &pending.checkpoint,
    )?;
    host::save_private(
        &directory.join("previous-document.json"),
        &serde_json::json!({"uri":uri,"source":local_source}),
    )?;
    let mut replica = DocumentReplica::from_snapshot(
        pending.joined.credentials.actor.clone(),
        pending.joined.state.snapshot.clone(),
    )?;
    let preserve_draft = resolved.is_none();
    let retained_draft = pending.checkpoint.source.clone();
    if pending.joined.state.role == Role::Viewer && pending.has_saved_draft && preserve_draft {
        return Err(host::HostError::new(
            host::ErrorCode::Forbidden,
            "Viewers must explicitly adopt the remote snapshot; the saved draft and outbox remain in the review backup",
        ));
    }
    if let Some(resolved) = resolved {
        if resolved != replica.source() {
            if pending.joined.state.role == Role::Viewer {
                return Err(host::HostError::new(
                    host::ErrorCode::Forbidden,
                    "Viewers cannot publish a source resolution",
                ));
            }
            replica.submit(
                "Resolve shared snapshot",
                vec![SharedEdit::ReplaceSource {
                    expected_source: replica.source().into(),
                    source: resolved,
                }],
            )?;
        }
    }
    pending.joined.state.snapshot = replica.snapshot();
    pending.checkpoint.peer.snapshot = replica.snapshot();
    pending.checkpoint.peer.epoch = pending.joined.state.epoch;
    pending.checkpoint.peer.role = pending.joined.state.role;
    // A resumed snapshot can include offline operations, while a fetched target snapshot has
    // its own authoritative vector. Neither inherits acknowledgements from a prepared edit.
    pending.checkpoint.peer.server_version = pending.acknowledged_version.clone();
    pending.checkpoint.source = if preserve_draft {
        retained_draft
    } else {
        replica.source().into()
    };
    pending.checkpoint.uri = uri;
    // Preparation is durable but not yet accepted. The UI's revision/ownership CAS must
    // succeed before CommitAccept replaces peer.json. A rejected proposal remains recoverable
    // without being mistaken for the remote snapshot on a later review or restart.
    host::save_private(
        &directory.join("reviewed-resolution.json"),
        &pending.checkpoint,
    )?;
    Ok(pending)
}
fn setup(
    start: Start,
    options: &SharingOptions,
    source: String,
    uri: String,
) -> host::Result<(Worker, Pending, bool)> {
    private_directory(&options.directory)?;
    host::save_private(
        &options.directory.join("previous-document.json"),
        &serde_json::json!({"uri":uri,"source":source}),
    )?;
    let mut server = None;
    let mut hosted_address = None;
    let mut hosted_directory = None;
    let mut bootstrap_path = None;
    let (peer, joined, restore, automatic) = match start {
        Start::Host => {
            let admin_path = options.directory.join("administrator.json");
            let admin: Secret = if admin_path.exists() {
                host::load_private(&admin_path)?
            } else {
                let secret = Secret::generate()?;
                host::save_private(&admin_path, &secret)?;
                secret
            };
            let directory = options.directory.join("rooms");
            let hosted = HostedServer::start(&directory, &admin, options.bind)?;
            let endpoint = hosted.endpoint();
            hosted_address = endpoint
                .url
                .strip_prefix("http://")
                .and_then(|url| url.strip_suffix("/mcp"))
                .and_then(|value| value.parse().ok());
            hosted_directory = Some(directory);
            let mut peer = host::McpPeer::new(endpoint)?;
            let path = options.directory.join("pending-create.json");
            let existing = if path.exists() {
                Some(host::load_private::<host::CreateRequest>(&path)?)
            } else {
                None
            };
            let request = existing
                .filter(|request| {
                    request.document_id == uri && request.source == source && request.admin == admin
                })
                .unwrap_or(host::CreateRequest {
                    admin,
                    owner_token: Secret::generate()?,
                    title: uri
                        .rsplit('/')
                        .next()
                        .unwrap_or("Shared design")
                        .chars()
                        .take(100)
                        .collect(),
                    document_id: uri.clone(),
                    source: source.clone(),
                    display_name: options.display_name.clone(),
                });
            host::save_private(&path, &request)?;
            bootstrap_path = Some(path);
            let joined = decode(peer.request(Request::Create { request })?)?;
            server = Some(hosted);
            (peer, joined, None, true)
        }
        Start::Join(invitation) => {
            let path = options.directory.join("pending-join.json");
            let existing = if path.exists() {
                Some(host::load_private::<PendingJoin>(&path)?)
            } else {
                None
            };
            let request = existing
                .filter(|value| {
                    value.endpoint.url == invitation.endpoint.url
                        && value.endpoint.bearer == invitation.endpoint.bearer
                        && value.request.room == invitation.room
                        && value.request.invite == invitation.invite
                })
                .map(|value| value.request)
                .unwrap_or(host::JoinRequest {
                    room: invitation.room,
                    invite: invitation.invite,
                    member_token: Secret::generate()?,
                    display_name: options.display_name.clone(),
                });
            host::save_private(
                &path,
                &PendingJoin {
                    endpoint: invitation.endpoint.clone(),
                    request: request.clone(),
                },
            )?;
            bootstrap_path = Some(path);
            let mut peer = host::McpPeer::new(invitation.endpoint)?;
            let joined = decode(peer.request(Request::Join { request })?)?;
            (peer, joined, None, false)
        }
        Start::Resume => {
            let checkpoint: NativeCheckpoint =
                host::load_private(&options.directory.join("peer.json"))?;
            if let Some(directory) = &checkpoint.hosted_directory {
                let admin: Secret =
                    host::load_private(&options.directory.join("administrator.json"))?;
                server = Some(HostedServer::start(
                    directory,
                    &admin,
                    checkpoint.hosted_address,
                )?);
                hosted_address = checkpoint.hosted_address;
                hosted_directory = checkpoint.hosted_directory.clone();
            }
            let endpoint = server
                .as_ref()
                .map(HostedServer::endpoint)
                .unwrap_or_else(|| checkpoint.peer.endpoint.clone());
            let peer = host::McpPeer::new(endpoint)?;
            let joined = JoinedRoom {
                credentials: checkpoint.peer.credentials.clone(),
                state: RoomState {
                    room: checkpoint.peer.credentials.room.clone(),
                    title: "Resumed room".into(),
                    epoch: checkpoint.peer.epoch,
                    snapshot: checkpoint.peer.snapshot.clone(),
                    members: vec![],
                    role: checkpoint.peer.role,
                },
            };
            (peer, joined, Some(checkpoint), false)
        }
    };
    let replica = DocumentReplica::from_snapshot(
        joined.credentials.actor.clone(),
        joined.state.snapshot.clone(),
    )?;
    let has_saved_draft = restore.is_some();
    let checkpoint = restore.unwrap_or_else(|| NativeCheckpoint {
        peer: PeerCheckpoint {
            protocol: host::HOST_PROTOCOL,
            endpoint: peer.endpoint().clone(),
            credentials: joined.credentials.clone(),
            epoch: joined.state.epoch,
            snapshot: joined.state.snapshot.clone(),
            server_version: replica.version(),
            role: joined.state.role,
        },
        source: replica.source().into(),
        uri,
        hosted_address,
        hosted_directory,
    });
    host::save_private(&options.directory.join("peer.json"), &checkpoint)?;
    if let Some(path) = bootstrap_path {
        std::fs::remove_file(path)
            .map_err(|error| host::HostError::new(host::ErrorCode::Storage, error.to_string()))?;
    }
    let pending = Pending {
        joined: joined.clone(),
        checkpoint: checkpoint.clone(),
        acknowledged_version: checkpoint.peer.server_version.clone(),
        has_saved_draft,
    };
    let mut invitation_endpoint = peer.endpoint().clone();
    if let Some(url) = &options.public_url {
        invitation_endpoint.url = url.clone();
    }
    let worker = Worker {
        peer,
        credentials: joined.credentials,
        checkpoint,
        invitation_endpoint,
        _server: server,
    };
    Ok((worker, pending, automatic))
}
fn worker(
    start: Start,
    options: SharingOptions,
    source: String,
    uri: String,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
) {
    let _directory_lock = match client_lock(&options.directory) {
        Ok(lock) => lock,
        Err(error) => {
            let _ = events.send(Event::Error { error, state: None });
            return;
        }
    };
    let (mut worker, pending, automatic) = match setup(start, &options, source, uri) {
        Ok(value) => value,
        Err(error) => {
            let _ = events.send(Event::Error { error, state: None });
            return;
        }
    };
    if events
        .send(Event::Ready {
            pending: Box::new(pending),
            automatic,
        })
        .is_err()
    {
        return;
    }
    while let Ok(command) = commands.recv() {
        if let Command::Stop(checkpoint) = command {
            let result = checkpoint.as_ref().map_or(Ok(()), |checkpoint| {
                host::save_private(&options.directory.join("peer.json"), checkpoint)
            });
            let _ = events.send(Event::Stopped(result));
            break;
        }
        let result = (|| -> host::Result<Event> {
            match command {
                Command::Sync {
                    exchange,
                    checkpoint,
                } => {
                    host::save_private(&options.directory.join("peer.json"), &checkpoint)?;
                    worker.checkpoint = checkpoint;
                    let reply: ExchangeReply = decode(worker.peer.request(Request::Exchange {
                        credentials: worker.credentials.clone(),
                        exchange,
                    })?)?;
                    Ok(Event::Synced(reply))
                }
                Command::Invite(role) => {
                    let token = Secret::generate()?;
                    worker.peer.request(Request::Invite {
                        credentials: worker.credentials.clone(),
                        token: token.clone(),
                        role,
                        expires_in: 86400,
                        uses: 32,
                    })?;
                    Ok(Event::Invited(
                        Invitation {
                            protocol: host::HOST_PROTOCOL,
                            endpoint: worker.invitation_endpoint.clone(),
                            room: worker.credentials.room.clone(),
                            invite: token,
                        }
                        .encode()?,
                    ))
                }
                Command::Member {
                    actor,
                    role,
                    revoked,
                } => {
                    worker.peer.request(Request::SetMember {
                        credentials: worker.credentials.clone(),
                        actor,
                        role,
                        revoked,
                    })?;
                    Ok(Event::State(decode(worker.peer.request(
                        Request::State {
                            credentials: worker.credentials.clone(),
                        },
                    )?)?))
                }
                Command::Checkpoint {
                    epoch,
                    version,
                    resolved_source,
                } => Ok(Event::State(decode(worker.peer.request(
                    Request::Checkpoint {
                        credentials: worker.credentials.clone(),
                        epoch,
                        expected: version,
                        resolved_source: Some(resolved_source),
                    },
                )?)?)),
                Command::Refresh => Ok(Event::State(decode(worker.peer.request(
                    Request::State {
                        credentials: worker.credentials.clone(),
                    },
                )?)?)),
                Command::Accept {
                    pending,
                    revision,
                    local_source,
                    resolved,
                    uri,
                } => {
                    let reviewed = pending.clone();
                    let pending =
                        prepare_accept(pending, local_source, resolved, uri, &options.directory)?;
                    Ok(Event::Accepted {
                        pending: Box::new(pending),
                        reviewed: Box::new(reviewed),
                        revision,
                    })
                }
                Command::CommitAccept(checkpoint) => {
                    host::save_private(&options.directory.join("peer.json"), &checkpoint)?;
                    worker.checkpoint = checkpoint;
                    Ok(Event::AcceptedCommitted)
                }
                Command::Stop(_) => unreachable!(),
            }
        })();
        match result {
            Ok(event) => {
                if events.send(event).is_err() {
                    break;
                }
            }
            Err(error) => {
                // A rename may already have replaced peer.json. Do not let a queued sync,
                // acceptance or final-stop checkpoint overwrite that uncertain durable state.
                let terminal = error.code == host::ErrorCode::Indeterminate;
                // Keep the operation busy until its authority refresh finishes. One event
                // prevents an intervening poll from queuing another rejected editor outbox.
                let state = if matches!(
                    error.code,
                    host::ErrorCode::EpochChanged
                        | host::ErrorCode::Forbidden
                        | host::ErrorCode::Unauthorized
                ) {
                    worker
                        .peer
                        .request(Request::State {
                            credentials: worker.credentials.clone(),
                        })
                        .and_then(decode)
                        .ok()
                } else {
                    None
                };
                if events.send(Event::Error { error, state }).is_err() || terminal {
                    break;
                }
            }
        }
    }
}

fn client_lock(directory: &Path) -> host::Result<std::fs::File> {
    private_directory(directory)?;
    let path = directory.join("client.lock");
    if path.exists()
        && std::fs::symlink_metadata(&path)
            .map_err(|error| host::HostError::new(host::ErrorCode::Storage, error.to_string()))?
            .file_type()
            .is_symlink()
    {
        return Err(host::HostError::new(
            host::ErrorCode::Storage,
            "Client lock must not be a symlink",
        ));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|error| host::HostError::new(host::ErrorCode::Storage, error.to_string()))?;
    file.try_lock().map_err(|_|host::HostError::new(host::ErrorCode::Storage,"This sharing directory is still in use; wait for disconnect to finish or choose another directory"))?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        ContentControl,
    };
    use host::Endpoint;
    const SOURCE: &str = r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Width="100" /></Canvas>"#;
    struct Fixture {
        session: Rc<RefCell<DesignerSession>>,
        controller: Rc<SharingController>,
        commands: mpsc::Receiver<Command>,
        events: mpsc::Sender<Event>,
    }
    fn fixture() -> Fixture {
        let session = Rc::new(RefCell::new(
            DesignerSession::with_uri(SOURCE, ContentControl::new(), "file:///Main.xaml".into())
                .unwrap(),
        ));
        session
            .borrow_mut()
            .enable_collaboration("actor-a", "document-a")
            .unwrap();
        let snapshot = session.borrow().collaboration_snapshot().unwrap();
        let credentials = Credentials {
            room: "room-a".into(),
            actor: "actor-a".into(),
            token: Secret::generate().unwrap(),
        };
        let checkpoint = NativeCheckpoint {
            peer: PeerCheckpoint {
                protocol: host::HOST_PROTOCOL,
                endpoint: Endpoint {
                    url: "http://127.0.0.1:1/mcp".into(),
                    bearer: Secret::generate().unwrap(),
                },
                credentials,
                epoch: 1,
                snapshot,
                server_version: VersionVector::new(),
                role: Role::Owner,
            },
            source: SOURCE.into(),
            uri: "file:///Main.xaml".into(),
            hosted_address: None,
            hosted_directory: None,
        };
        let (tx, commands) = mpsc::channel();
        let (events, rx) = mpsc::channel();
        let revision = session.borrow().document().revision();
        let controller = Rc::new(SharingController {
            session: Rc::downgrade(&session),
            commands: tx,
            events: RefCell::new(rx),
            status: RefCell::new(SharingStatus {
                phase: SharingPhase::Connected,
                message: String::new(),
                role: Some(Role::Owner),
                members: vec![],
                pending_operations: 0,
                invitation: None,
                backup_path: None,
            }),
            pending: RefCell::new(None),
            active: RefCell::new(Some(checkpoint)),
            busy: Cell::new(false),
            next_sync: Cell::new(Instant::now() + Duration::from_secs(60)),
            failures: Cell::new(0),
            authorization_blocked: Cell::new(false),
            durability_blocked: Cell::new(false),
            disconnect_requested: Cell::new(false),
            display_name: "Owner".into(),
            bound_uri: "file:///Main.xaml".into(),
            starting_revision: revision,
            directory: PathBuf::from("unused"),
            cursor: RefCell::new(None),
        });
        Fixture {
            session,
            controller,
            commands,
            events,
        }
    }
    fn review_pending(checkpoint: NativeCheckpoint) -> Pending {
        Pending {
            joined: JoinedRoom {
                credentials: checkpoint.peer.credentials.clone(),
                state: RoomState {
                    room: checkpoint.peer.credentials.room.clone(),
                    title: "Review".into(),
                    epoch: checkpoint.peer.epoch,
                    snapshot: checkpoint.peer.snapshot.clone(),
                    members: vec![],
                    role: checkpoint.peer.role,
                },
            },
            acknowledged_version: checkpoint.peer.server_version.clone(),
            checkpoint,
            has_saved_draft: true,
        }
    }
    fn temporary_directory() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("ferroui-sharing-{}", Secret::generate().unwrap().0));
        private_directory(&path).unwrap();
        path
    }
    #[test]
    fn indeterminate_durability_blocks_edits_retries_and_final_checkpoint_replacement() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        fixture.controller.next_sync.set(Instant::now());
        fixture
            .events
            .send(Event::Error {
                error: host::HostError::new(
                    host::ErrorCode::Indeterminate,
                    "Directory sync failed after replacing peer.json",
                ),
                state: None,
            })
            .unwrap();
        assert!(fixture.controller.poll().unwrap());
        assert_eq!(fixture.controller.status().phase, SharingPhase::Conflict);
        assert!(fixture.session.borrow().is_read_only());
        let id = fixture
            .session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        assert!(fixture
            .session
            .borrow_mut()
            .set_property(&[id], "Width", "999")
            .is_err());
        assert!(fixture.controller.invite(Role::Editor).is_err());
        assert!(fixture.commands.try_recv().is_err());
        assert!(fixture.controller.capture_checkpoint().is_none());
        assert!(!fixture.controller.poll().unwrap());
        fixture.controller.disconnect();
        assert!(matches!(
            fixture.commands.try_recv().unwrap(),
            Command::Stop(None)
        ));
        assert!(!fixture.session.borrow().is_read_only());
    }
    #[test]
    fn save_as_disconnect_captures_latest_outbox_and_clears_owned_viewer_guard() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        let id = fixture
            .session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        fixture
            .session
            .borrow_mut()
            .set_property(&[id], "Width", "175")
            .unwrap();
        fixture
            .session
            .borrow_mut()
            .set_document_uri("file:///Renamed.xaml".into())
            .unwrap();
        fixture.session.borrow_mut().set_read_only(true);
        fixture.controller.disconnect();
        let Command::Stop(Some(checkpoint)) = fixture.commands.try_recv().unwrap() else {
            panic!("final checkpoint missing")
        };
        assert!(checkpoint.source.contains("175"));
        assert_eq!(checkpoint.peer.snapshot.operations.len(), 1);
        assert!(checkpoint.peer.server_version.is_empty());
        assert_eq!(checkpoint.uri, "file:///Renamed.xaml");
        assert!(fixture.session.borrow().collaboration_actor().is_none());
        assert!(!fixture.session.borrow().is_read_only());
    }
    #[test]
    fn old_controller_cannot_detach_a_new_tab_or_new_room() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        fixture
            .session
            .borrow_mut()
            .open_document(SOURCE.into(), "file:///Other.xaml".into())
            .unwrap();
        fixture
            .session
            .borrow_mut()
            .enable_collaboration("actor-b", "document-b")
            .unwrap();
        fixture.session.borrow_mut().set_read_only(true);
        fixture.controller.poll().unwrap();
        assert_eq!(
            fixture.session.borrow().collaboration_actor(),
            Some("actor-b")
        );
        assert!(fixture.session.borrow().is_read_only());
        assert!(matches!(
            fixture.commands.try_recv().unwrap(),
            Command::Stop(None)
        ));
    }
    #[test]
    fn same_uri_replacement_rejects_old_replies_and_competing_pending_attachment() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let old = fixture();
        let checkpoint = old.controller.active.borrow().clone().unwrap();
        let state = review_pending(checkpoint.clone()).joined.state;
        let mut remote =
            DocumentReplica::from_snapshot("actor-a", checkpoint.peer.snapshot).unwrap();
        remote.set_attribute(remote.root(), "Width", "777").unwrap();
        {
            let mut session = old.session.borrow_mut();
            session.leave_collaboration();
            session
                .enable_collaboration("actor-b", "document-a")
                .unwrap();
            session.set_read_only(true);
        }
        old.events.send(Event::State(state)).unwrap();
        old.events
            .send(Event::Synced(ExchangeReply {
                epoch: 1,
                version: remote.version(),
                message: remote.changes_since(&VersionVector::new()),
                conflicts: vec![],
                presence: vec![],
                role: Role::Owner,
            }))
            .unwrap();
        old.controller.poll().unwrap();
        assert_eq!(old.session.borrow().collaboration_actor(), Some("actor-b"));
        assert!(old.session.borrow().is_read_only());
        assert_eq!(old.session.borrow().document().source(), SOURCE);
        assert!(matches!(
            old.commands.try_recv().unwrap(),
            Command::Stop(None)
        ));

        // These are independent ownership scenarios. Release the first preview's hot-reload
        // registration before the next fixture opens the same document URI.
        drop(old);
        let pending = fixture();
        let reviewed = review_pending(pending.controller.active.borrow_mut().take().unwrap());
        let revision = pending.session.borrow().document().revision();
        {
            let mut session = pending.session.borrow_mut();
            session.leave_collaboration();
            session
                .enable_collaboration("actor-c", "document-a")
                .unwrap();
            session.set_read_only(true);
        }
        assert!(pending
            .controller
            .attach(reviewed, revision, false)
            .is_err());
        assert_eq!(
            pending.session.borrow().collaboration_actor(),
            Some("actor-c")
        );
        assert!(pending.session.borrow().is_read_only());
        assert!(matches!(
            pending.commands.try_recv().unwrap(),
            Command::Stop(None)
        ));
    }
    #[test]
    fn reentrant_disconnect_waits_for_the_transaction_and_captures_its_final_outbox() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        let mut session = fixture.session.borrow_mut();
        fixture.controller.disconnect();
        assert!(fixture.controller.disconnect_requested.get());
        assert!(!fixture.controller.poll().unwrap());
        let target = session.document().ast().find_name("Target").unwrap();
        session.set_property(&[target], "Width", "230").unwrap();
        drop(session);
        fixture.controller.poll().unwrap();
        let Command::Stop(Some(checkpoint)) = fixture.commands.try_recv().unwrap() else {
            panic!("completed transaction checkpoint missing")
        };
        assert!(checkpoint.source.contains("230"));
        assert_eq!(checkpoint.peer.snapshot.operations.len(), 1);
        assert!(fixture.session.borrow().collaboration_actor().is_none());
    }
    #[test]
    fn invalid_public_endpoint_and_bind_are_rejected_before_creating_state() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        fixture.session.borrow_mut().leave_collaboration();
        let directory = std::env::temp_dir().join(format!(
            "ferroui-sharing-invalid-{}",
            Secret::generate().unwrap().0
        ));
        for (url, bind) in [
            (Some("https://user:secret@example.org/mcp"), None),
            (Some("https://example.org/mcp?secret=value"), None),
            (None, Some("0.0.0.0:0".parse().unwrap())),
        ] {
            let options = SharingOptions {
                directory: directory.clone(),
                display_name: "Owner".into(),
                bind,
                public_url: url.map(str::to_owned),
            };
            assert!(SharingController::host(&fixture.session, options).is_err());
            assert!(!directory.exists());
        }
    }
    #[test]
    fn resume_accept_preserves_unsent_operations_and_invalid_source_draft() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        let mut checkpoint = fixture.controller.active.borrow().clone().unwrap();
        let mut replica =
            DocumentReplica::from_snapshot("actor-a", checkpoint.peer.snapshot.clone()).unwrap();
        replica
            .set_attribute(replica.root(), "Width", "42")
            .unwrap();
        checkpoint.peer.snapshot = replica.snapshot();
        checkpoint.source = "<Canvas><Button".into();
        let pending = Pending {
            joined: JoinedRoom {
                credentials: checkpoint.peer.credentials.clone(),
                state: RoomState {
                    room: "room-a".into(),
                    title: "Resume".into(),
                    epoch: 1,
                    snapshot: replica.snapshot(),
                    members: vec![],
                    role: Role::Owner,
                },
            },
            checkpoint,
            acknowledged_version: VersionVector::new(),
            has_saved_draft: true,
        };
        let path =
            std::env::temp_dir().join(format!("ferroui-sharing-{}", Secret::generate().unwrap().0));
        private_directory(&path).unwrap();
        let restored = prepare_accept(
            pending,
            "local document".into(),
            None,
            "file:///Main.xaml".into(),
            &path,
        )
        .unwrap();
        assert!(restored.checkpoint.peer.server_version.is_empty());
        assert_eq!(restored.checkpoint.peer.snapshot.operations.len(), 1);
        assert_eq!(restored.checkpoint.source, "<Canvas><Button");
        let archive: NativeCheckpoint =
            host::load_private(&path.join("previous-sharing-checkpoint.json")).unwrap();
        assert_eq!(archive.source, restored.checkpoint.source);
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn delayed_acceptance_cannot_overwrite_new_local_edits() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        let checkpoint = fixture.controller.active.borrow().clone().unwrap();
        let path = temporary_directory();
        host::save_private(&path.join("peer.json"), &checkpoint).unwrap();
        let reviewed = review_pending(checkpoint.clone());
        let proposed = SOURCE.replace("100", "150");
        let pending = prepare_accept(
            reviewed.clone(),
            SOURCE.into(),
            Some(proposed.clone()),
            "file:///Main.xaml".into(),
            &path,
        )
        .unwrap();
        let saved: NativeCheckpoint = host::load_private(&path.join("peer.json")).unwrap();
        assert_eq!(saved.source, SOURCE);
        assert!(saved.peer.snapshot.operations.is_empty());
        let prepared: NativeCheckpoint =
            host::load_private(&path.join("reviewed-resolution.json")).unwrap();
        assert_eq!(prepared.source, proposed);
        let revision = fixture.session.borrow().document().revision();
        let id = fixture
            .session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        fixture
            .session
            .borrow_mut()
            .set_property(&[id], "Width", "210")
            .unwrap();
        fixture
            .events
            .send(Event::Accepted {
                reviewed: Box::new(reviewed),
                pending: Box::new(pending),
                revision,
            })
            .unwrap();
        assert!(fixture.controller.poll().is_err());
        assert!(fixture.session.borrow().document().source().contains("210"));
        assert_eq!(
            fixture.controller.status().phase,
            SharingPhase::AwaitingReview
        );
        assert_eq!(fixture.controller.pending_source().as_deref(), Some(SOURCE));
        assert!(fixture.commands.try_recv().is_err());
        let saved: NativeCheckpoint = host::load_private(&path.join("peer.json")).unwrap();
        assert_eq!(saved.source, SOURCE);

        // A second decision to use the real remote source must not publish the rejected
        // proposal. The newest old-room outbox is archived, including edits made in review.
        fixture
            .controller
            .accept_snapshot(
                fixture.session.borrow().document().revision(),
                Some(SOURCE.into()),
            )
            .unwrap();
        let Command::Accept {
            pending,
            revision,
            local_source,
            resolved,
            uri,
        } = fixture.commands.try_recv().unwrap()
        else {
            panic!("review acceptance missing")
        };
        assert!(pending.checkpoint.source.contains("210"));
        assert_eq!(pending.checkpoint.peer.snapshot.operations.len(), 1);
        let reviewed = pending.clone();
        let prepared = prepare_accept(pending, local_source, resolved, uri, &path).unwrap();
        let archive: NativeCheckpoint =
            host::load_private(&path.join("previous-sharing-checkpoint.json")).unwrap();
        assert!(archive.source.contains("210"));
        assert_eq!(archive.peer.snapshot.operations.len(), 1);
        fixture
            .events
            .send(Event::Accepted {
                pending: Box::new(prepared),
                reviewed: Box::new(reviewed),
                revision,
            })
            .unwrap();
        fixture.controller.poll().unwrap();
        let Command::CommitAccept(accepted) = fixture.commands.try_recv().unwrap() else {
            panic!("accepted checkpoint must be committed before synchronization")
        };
        assert_eq!(accepted.source, SOURCE);
        assert!(accepted.peer.snapshot.operations.is_empty());
        assert!(fixture.commands.try_recv().is_err());
        assert!(fixture.controller.busy.get());
        assert_eq!(fixture.controller.status().phase, SharingPhase::Connecting);
        // Failed commit cannot claim durability or discard the visible accepted source and
        // prepared recovery file. A later sync retries persistence before sending anything.
        fixture
            .events
            .send(Event::Error {
                error: host::HostError::new(host::ErrorCode::Storage, "disk full"),
                state: None,
            })
            .unwrap();
        fixture.controller.poll().unwrap();
        assert_eq!(
            fixture.controller.status().phase,
            SharingPhase::Reconnecting
        );
        assert_eq!(fixture.session.borrow().document().source(), SOURCE);
        assert!(path.join("reviewed-resolution.json").exists());
        let saved: NativeCheckpoint = host::load_private(&path.join("peer.json")).unwrap();
        assert_eq!(saved.source, SOURCE);
        std::fs::remove_dir_all(path).unwrap();
    }
    #[test]
    fn viewer_downgrade_retains_exact_unacknowledged_replica_and_invalid_draft_for_review() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let fixture = fixture();
        let remote_checkpoint = fixture.controller.active.borrow().clone().unwrap();
        let mut state = review_pending(remote_checkpoint).joined.state;
        state.role = Role::Viewer;
        let target = fixture
            .session
            .borrow()
            .document()
            .ast()
            .find_name("Target")
            .unwrap();
        fixture
            .session
            .borrow_mut()
            .set_property(&[target], "Width", "225")
            .unwrap();
        fixture
            .session
            .borrow_mut()
            .update_source("<Canvas><Button");
        let previous = fixture.controller.capture_checkpoint().unwrap();
        fixture
            .events
            .send(Event::Error {
                error: host::HostError::new(
                    host::ErrorCode::Forbidden,
                    "Viewers cannot change the document",
                ),
                state: Some(state),
            })
            .unwrap();
        fixture.controller.poll().unwrap();
        assert!(fixture.session.borrow().is_read_only());
        assert_eq!(
            fixture.controller.status().phase,
            SharingPhase::AwaitingReview
        );
        assert_eq!(fixture.controller.pending_source().as_deref(), Some(SOURCE));
        assert_eq!(
            fixture.controller.pending_local_source().as_deref(),
            Some("<Canvas><Button")
        );
        fixture.controller.next_sync.set(Instant::now());
        fixture.controller.poll().unwrap();
        assert!(fixture.commands.try_recv().is_err());
        let pending = fixture.controller.pending.borrow().clone().unwrap();
        assert_eq!(
            serde_json::to_value(&pending.checkpoint).unwrap(),
            serde_json::to_value(&previous).unwrap()
        );
        let path = temporary_directory();
        host::save_private(&path.join("peer.json"), &previous).unwrap();
        assert!(prepare_accept(
            pending.clone(),
            previous.source.clone(),
            None,
            previous.uri.clone(),
            &path
        )
        .is_err());
        let accepted = prepare_accept(
            pending,
            previous.source.clone(),
            Some(SOURCE.into()),
            previous.uri.clone(),
            &path,
        )
        .unwrap();
        let archive: NativeCheckpoint =
            host::load_private(&path.join("previous-sharing-checkpoint.json")).unwrap();
        assert_eq!(
            serde_json::to_value(archive).unwrap(),
            serde_json::to_value(&previous).unwrap()
        );
        assert_eq!(accepted.checkpoint.peer.role, Role::Viewer);
        assert_eq!(accepted.checkpoint.source, SOURCE);
        assert!(accepted.checkpoint.peer.snapshot.operations.is_empty());
        assert!(accepted.checkpoint.peer.server_version.is_empty());
        let saved: NativeCheckpoint = host::load_private(&path.join("peer.json")).unwrap();
        assert_eq!(
            serde_json::to_value(saved).unwrap(),
            serde_json::to_value(previous).unwrap()
        );
        std::fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn native_workers_host_review_join_converge_and_enforce_live_role_changes() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let directory = std::env::temp_dir().join(format!(
            "ferroui-native-sharing-{}",
            Secret::generate().unwrap().0
        ));
        let make_session = |source: &str, name: &str| {
            Rc::new(RefCell::new(
                DesignerSession::with_uri(
                    source,
                    ContentControl::new(),
                    format!("file:///{name}.xaml"),
                )
                .unwrap(),
            ))
        };
        let options = |name: &str| SharingOptions {
            directory: directory.join(name),
            display_name: name.into(),
            bind: None,
            public_url: None,
        };
        let owner_session = make_session(SOURCE, "Owner");
        let guest_source = r#"<Panel xmlns="https://github.com/ferroui" />"#;
        let guest_session = make_session(guest_source, "Guest");
        let owner = SharingController::host(&owner_session, options("Owner")).unwrap();
        let pump = |controllers: &[&Rc<SharingController>], ready: &dyn Fn() -> bool| {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                for controller in controllers {
                    controller.poll().unwrap();
                }
                if ready() {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "sharing timed out: {:?}",
                    controllers
                        .iter()
                        .map(|controller| controller.status())
                        .collect::<Vec<_>>()
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        pump(&[&owner], &|| {
            owner.status().phase == SharingPhase::Connected && !owner.busy.get()
        });
        owner.invite(Role::Editor).unwrap();
        pump(&[&owner], &|| {
            owner.status().invitation.is_some() && !owner.busy.get()
        });
        let guest = SharingController::join(
            &guest_session,
            owner.status().invitation.unwrap(),
            options("Guest"),
        )
        .unwrap();
        pump(&[&owner, &guest], &|| {
            guest.status().phase == SharingPhase::AwaitingReview
        });
        assert_eq!(guest_session.borrow().document().source(), guest_source);
        assert_eq!(guest.pending_source().as_deref(), Some(SOURCE));
        assert!(guest.pending_local_source().is_none());
        let revision = guest_session.borrow().document().revision();
        guest
            .accept_snapshot(revision, guest.pending_source())
            .unwrap();
        pump(&[&owner, &guest], &|| {
            guest.status().phase == SharingPhase::Connected
        });
        let target = |session: &Rc<RefCell<DesignerSession>>| {
            session
                .borrow()
                .document()
                .ast()
                .find_name("Target")
                .unwrap()
        };
        let owner_target = target(&owner_session);
        let guest_target = target(&guest_session);
        owner_session
            .borrow_mut()
            .set_property(&[owner_target], "Width", "175")
            .unwrap();
        guest_session
            .borrow_mut()
            .set_property(&[guest_target], "Height", "55")
            .unwrap();
        owner.set_cursor(Some(Cursor { x: 42.0, y: 24.0 })).unwrap();
        pump(&[&owner, &guest], &|| {
            let one = owner_session.borrow();
            let two = guest_session.borrow();
            one.document().source() == two.document().source()
                && one.document().source().contains("175")
                && one.document().source().contains("55")
                && two
                    .current_presence(30_000)
                    .iter()
                    .any(|peer| peer.cursor == Some(Cursor { x: 42.0, y: 24.0 }))
                && !owner.busy.get()
                && !guest.busy.get()
        });
        let guest_actor = guest_session
            .borrow()
            .collaboration_actor()
            .unwrap()
            .to_owned();
        // Downgrade before the editor's newest outbox reaches the server. The refusal must
        // fetch the new role, preserve the invalid source draft and expose a review, rather
        // than repeatedly resending operations the viewer is no longer allowed to publish.
        guest
            .next_sync
            .set(Instant::now() + Duration::from_secs(60));
        guest_session
            .borrow_mut()
            .set_property(&[guest_target], "Width", "225")
            .unwrap();
        guest_session.borrow_mut().update_source("<Canvas><Button");
        owner
            .set_member(guest_actor.clone(), Role::Viewer, false)
            .unwrap();
        pump(&[&owner], &|| {
            !owner.busy.get()
                && owner
                    .status()
                    .members
                    .iter()
                    .any(|member| member.actor == guest_actor && member.role == Role::Viewer)
        });
        guest.next_sync.set(Instant::now());
        pump(&[&owner, &guest], &|| {
            guest.status().role == Some(Role::Viewer)
                && guest.status().phase == SharingPhase::AwaitingReview
        });
        assert!(guest_session.borrow().is_read_only());
        assert_eq!(
            guest_session.borrow().document().source(),
            "<Canvas><Button"
        );
        assert_eq!(
            guest.pending_local_source().as_deref(),
            Some("<Canvas><Button")
        );
        assert!(guest_session
            .borrow_mut()
            .set_property(&[guest_target], "Width", "999")
            .is_err());
        guest
            .accept_snapshot(
                guest_session.borrow().document().revision(),
                guest.pending_source(),
            )
            .unwrap();
        pump(&[&owner, &guest], &|| {
            guest.status().phase == SharingPhase::Connected && !guest.busy.get()
        });
        let archived: NativeCheckpoint =
            host::load_private(&directory.join("Guest/previous-sharing-checkpoint.json")).unwrap();
        assert_eq!(archived.source, "<Canvas><Button");
        let archived_replica = DocumentReplica::from_snapshot(
            archived.peer.credentials.actor.clone(),
            archived.peer.snapshot,
        )
        .unwrap();
        assert!(archived_replica.source().contains("225"));
        assert_eq!(
            archived_replica
                .changes_since(&archived.peer.server_version)
                .operations
                .iter()
                .filter(|operation| operation.id.actor == archived.peer.credentials.actor)
                .count(),
            1
        );
        assert!(guest_session.borrow().is_read_only());
        assert!(guest_session.borrow().document().source().contains("175"));
        owner_session
            .borrow_mut()
            .set_property(&[owner_target], "Height", "66")
            .unwrap();
        pump(&[&owner, &guest], &|| {
            guest_session.borrow().document().source().contains("66")
        });
        guest.disconnect();
        owner.disconnect();
        pump(&[&owner, &guest], &|| {
            [&owner, &guest].iter().all(|controller| {
                controller
                    .status()
                    .message
                    .contains("final offline checkpoint is saved")
            })
        });
        assert!(!guest_session.borrow().is_read_only());
        let saved: NativeCheckpoint =
            host::load_private(&directory.join("Guest/peer.json")).unwrap();
        assert!(saved.source.contains("175") && saved.source.contains("66"));
        assert!(!directory.join("Owner/pending-create.json").exists());
        assert!(!directory.join("Guest/pending-join.json").exists());
        drop(guest);
        drop(owner);
        let _ = std::fs::remove_dir_all(directory);
    }
}
