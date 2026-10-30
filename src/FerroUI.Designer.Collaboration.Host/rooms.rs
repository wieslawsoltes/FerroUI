use crate::model::*;
use crate::storage;
use ferroui_designer_collaboration::{DocumentReplica, VersionVector, MAX_MESSAGE_BYTES};
use std::collections::BTreeMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;

struct LiveRoom {
    saved: StoredRoom,
    replica: DocumentReplica,
}
struct State {
    rooms: BTreeMap<String, LiveRoom>,
}
/// One process owns each storage directory (OS file lock). Clones should share an `Arc<RoomStore>`.
/// The mutex covers validation through durable commit, so authenticated concurrent calls cannot
/// acknowledge a lost update or race a role change. Presence uses the receiver's monotonic clock.
pub struct RoomStore {
    root: PathBuf,
    _lock: File,
    admin_hash: String,
    indeterminate: AtomicBool,
    state: Mutex<State>,
    started: Instant,
}
impl RoomStore {
    pub fn open(path: impl AsRef<Path>, admin: &Secret) -> Result<Self> {
        admin.validate()?;
        let (root, lock) = storage::open_directory(path.as_ref())?;
        let identity = root.join("identity.json");
        let admin_hash = if identity.exists() {
            let existing: String = storage::read(&identity)?;
            ensure(
                equal_secret(&existing, &admin.digest()),
                ErrorCode::Unauthorized,
                "Administrator credential does not match this store",
            )?;
            existing
        } else {
            storage::write(&identity, &admin.digest(), &no_cancel)?;
            admin.digest()
        };
        let mut rooms = BTreeMap::new();
        for entry in std::fs::read_dir(&root).map_err(HostError::storage)? {
            let entry = entry.map_err(HostError::storage)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("room-") || !name.ends_with(".json") {
                continue;
            }
            ensure(
                rooms.len() < 1024,
                ErrorCode::Limit,
                "Room count limit reached",
            )?;
            let saved: StoredRoom = storage::read(&entry.path())?;
            ensure(
                saved.protocol == HOST_PROTOCOL
                    && name == format!("room-{}.json", hash(saved.id.as_bytes())),
                ErrorCode::Storage,
                "Room identity does not match storage path",
            )?;
            let replica = DocumentReplica::from_snapshot("@host", saved.snapshot.clone())?;
            rooms.insert(saved.id.clone(), LiveRoom { saved, replica });
        }
        Ok(Self {
            root,
            _lock: lock,
            admin_hash,
            indeterminate: AtomicBool::new(false),
            state: Mutex::new(State { rooms }),
            started: Instant::now(),
        })
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>> {
        let state = self
            .state
            .lock()
            .map_err(|_| HostError::storage("Room lock poisoned"))?;
        ensure(
            !self.indeterminate.load(Ordering::Acquire),
            ErrorCode::Indeterminate,
            "Durability was indeterminate; restart and validate the stored state before continuing",
        )?;
        Ok(state)
    }
    fn persist(&self, id: &str, saved: &StoredRoom, fence: &dyn CommitFence) -> Result<()> {
        let result = storage::write(&self.path(id), saved, fence);
        if result
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::Indeterminate)
        {
            self.indeterminate.store(true, Ordering::Release);
        }
        result
    }
    fn path(&self, id: &str) -> PathBuf {
        self.root.join(format!("room-{}.json", hash(id.as_bytes())))
    }
    fn authenticate<'a>(room: &'a StoredRoom, credentials: &Credentials) -> Result<&'a Member> {
        credentials.token.validate()?;
        let member = room
            .members
            .get(&credentials.actor)
            .filter(|m| {
                !m.member.revoked && equal_secret(&m.token_hash, &credentials.token.digest())
            })
            .ok_or_else(|| {
                HostError::new(
                    ErrorCode::Unauthorized,
                    "Room credential is invalid or revoked",
                )
            })?;
        ensure(
            room.id == credentials.room,
            ErrorCode::Unauthorized,
            "Credential belongs to another room",
        )?;
        Ok(&member.member)
    }
    fn room<'a>(state: &'a State, id: &str) -> Result<&'a LiveRoom> {
        state
            .rooms
            .get(id)
            .ok_or_else(|| HostError::new(ErrorCode::NotFound, "Room not found"))
    }
    fn view(room: &LiveRoom, role: Role) -> RoomState {
        RoomState {
            room: room.saved.id.clone(),
            title: room.saved.title.clone(),
            epoch: room.saved.epoch,
            snapshot: room.replica.snapshot(),
            members: room
                .saved
                .members
                .values()
                .map(|m| m.member.clone())
                .collect(),
            role,
        }
    }
    pub fn authorize(&self, credentials: &Credentials) -> Result<Role> {
        let state = self.lock()?;
        let room = Self::room(&state, &credentials.room)?;
        Ok(Self::authenticate(&room.saved, credentials)?.role)
    }
    pub fn authorize_admin(&self, admin: &Secret) -> Result<()> {
        admin.validate()?;
        ensure(
            equal_secret(&self.admin_hash, &admin.digest()),
            ErrorCode::Unauthorized,
            "Administrator credential required",
        )
    }
    pub fn authorize_scope(&self, authority: &Authority) -> Result<()> {
        match authority {
            Authority::Administrator { token } => self.authorize_admin(token),
            Authority::Member { credentials } => self.authorize(credentials).map(|_| ()),
            Authority::Invitation { room, token } => {
                token.validate()?;
                let state = self.lock()?;
                let room = Self::room(&state, room)?;
                ensure(
                    room.saved
                        .invites
                        .get(&token.digest())
                        .is_some_and(|invite| invite.expires_at >= now_seconds()),
                    ErrorCode::Unauthorized,
                    "Invite is invalid or expired",
                )
            }
        }
    }
    /// `owner_token` is generated by the caller and is the idempotency key. Retrying a lost
    /// response cannot create another room or replace an existing document.
    pub fn create(&self, request: CreateRequest) -> Result<JoinedRoom> {
        self.create_checked(request, &no_cancel)
    }
    pub fn create_checked(
        &self,
        request: CreateRequest,
        fence: &dyn CommitFence,
    ) -> Result<JoinedRoom> {
        self.authorize_admin(&request.admin)?;
        request.owner_token.validate()?;
        bounded_name(&request.title, "Title")?;
        bounded_name(&request.display_name, "Display name")?;
        let id = format!("room-{}", request.owner_token.digest());
        let actor = format!("actor-{}", request.owner_token.digest());
        let credentials = Credentials {
            room: id.clone(),
            actor: actor.clone(),
            token: request.owner_token,
        };
        let mut state = self.lock()?;
        if let Some(room) = state.rooms.get(&id) {
            Self::authenticate(&room.saved, &credentials)?;
            ensure(
                room.saved.snapshot.document_id == request.document_id,
                ErrorCode::Conflict,
                "Creation key already belongs to another document",
            )?;
            return Ok(JoinedRoom {
                credentials,
                state: Self::view(room, Role::Owner),
            });
        }
        ensure(
            state.rooms.len() < 1024,
            ErrorCode::Limit,
            "Room count limit reached",
        )?;
        let replica = DocumentReplica::new("@host", request.document_id, request.source)?;
        let saved = StoredRoom {
            protocol: HOST_PROTOCOL,
            id: id.clone(),
            title: request.title,
            epoch: 1,
            snapshot: replica.snapshot(),
            members: BTreeMap::from([(
                actor.clone(),
                StoredMember {
                    member: Member {
                        actor,
                        display_name: request.display_name,
                        role: Role::Owner,
                        revoked: false,
                    },
                    token_hash: credentials.token.digest(),
                },
            )]),
            invites: BTreeMap::new(),
        };
        self.persist(&id, &saved, fence)?;
        let room = LiveRoom { saved, replica };
        let view = Self::view(&room, Role::Owner);
        state.rooms.insert(id, room);
        Ok(JoinedRoom {
            credentials,
            state: view,
        })
    }
    pub fn state(&self, credentials: &Credentials) -> Result<RoomState> {
        let state = self.lock()?;
        let room = Self::room(&state, &credentials.room)?;
        let role = Self::authenticate(&room.saved, credentials)?.role;
        Ok(Self::view(room, role))
    }
    pub fn invite(
        &self,
        credentials: &Credentials,
        token: Secret,
        role: Role,
        expires_in: u64,
        uses: u32,
    ) -> Result<()> {
        self.invite_checked(credentials, token, role, expires_in, uses, &no_cancel)
    }
    pub fn invite_checked(
        &self,
        credentials: &Credentials,
        token: Secret,
        role: Role,
        expires_in: u64,
        uses: u32,
        fence: &dyn CommitFence,
    ) -> Result<()> {
        token.validate()?;
        ensure(
            role != Role::Owner
                && expires_in > 0
                && expires_in <= 7 * 86400
                && uses > 0
                && uses <= 128,
            ErrorCode::Invalid,
            "Invites grant editor/viewer access for 1..604800 seconds and 1..128 uses",
        )?;
        let mut state = self.lock()?;
        let room = Self::room(&state, &credentials.room)?;
        ensure(
            Self::authenticate(&room.saved, credentials)?.role == Role::Owner,
            ErrorCode::Forbidden,
            "Only the owner can invite members",
        )?;
        let mut saved = room.saved.clone();
        let key = token.digest();
        if saved.invites.contains_key(&key) {
            return Ok(());
        }
        saved
            .invites
            .retain(|_, invite| invite.expires_at >= now_seconds());
        ensure(
            saved.invites.len() < 256,
            ErrorCode::Limit,
            "Invite count limit reached",
        )?;
        saved.invites.insert(
            key,
            Invite {
                role,
                expires_at: now_seconds().saturating_add(expires_in),
                remaining: uses,
                redeemed: BTreeMap::new(),
            },
        );
        self.persist(&saved.id, &saved, fence)?;
        state.rooms.get_mut(&credentials.room).unwrap().saved = saved;
        Ok(())
    }
    pub fn join(&self, request: JoinRequest) -> Result<JoinedRoom> {
        self.join_checked(request, &no_cancel)
    }
    pub fn join_checked(
        &self,
        request: JoinRequest,
        fence: &dyn CommitFence,
    ) -> Result<JoinedRoom> {
        request.invite.validate()?;
        request.member_token.validate()?;
        bounded_name(&request.display_name, "Display name")?;
        let mut state = self.lock()?;
        let room = Self::room(&state, &request.room)?;
        let mut saved = room.saved.clone();
        let token_hash = request.member_token.digest();
        let actor = format!("actor-{token_hash}");
        let credentials = Credentials {
            room: request.room.clone(),
            actor: actor.clone(),
            token: request.member_token,
        };
        let invite = saved
            .invites
            .get_mut(&request.invite.digest())
            .filter(|invite| invite.expires_at >= now_seconds())
            .ok_or_else(|| {
                HostError::new(ErrorCode::Unauthorized, "Invite is invalid or expired")
            })?;
        if invite.redeemed.get(&token_hash) == Some(&actor) {
            let current_role = Self::authenticate(&room.saved, &credentials)?.role;
            return Ok(JoinedRoom {
                credentials,
                state: Self::view(room, current_role),
            });
        }
        ensure(
            invite.remaining > 0,
            ErrorCode::Unauthorized,
            "Invite has no uses remaining",
        )?;
        ensure(
            saved.members.len() < 256,
            ErrorCode::Limit,
            "Member limit reached",
        )?;
        ensure(
            !saved.members.contains_key(&actor),
            ErrorCode::Conflict,
            "Member token already exists",
        )?;
        let role = invite.role;
        invite.remaining -= 1;
        invite.redeemed.insert(token_hash.clone(), actor.clone());
        saved.members.insert(
            actor.clone(),
            StoredMember {
                member: Member {
                    actor,
                    display_name: request.display_name,
                    role,
                    revoked: false,
                },
                token_hash,
            },
        );
        self.persist(&saved.id, &saved, fence)?;
        let room = state.rooms.get_mut(&request.room).unwrap();
        room.saved = saved;
        Ok(JoinedRoom {
            credentials,
            state: Self::view(room, role),
        })
    }
    pub fn set_member(
        &self,
        owner: &Credentials,
        actor: &str,
        role: Role,
        revoked: bool,
    ) -> Result<()> {
        self.set_member_checked(owner, actor, role, revoked, &no_cancel)
    }
    pub fn set_member_checked(
        &self,
        owner: &Credentials,
        actor: &str,
        role: Role,
        revoked: bool,
        fence: &dyn CommitFence,
    ) -> Result<()> {
        let mut state = self.lock()?;
        let room = Self::room(&state, &owner.room)?;
        ensure(
            Self::authenticate(&room.saved, owner)?.role == Role::Owner,
            ErrorCode::Forbidden,
            "Only the owner can change membership",
        )?;
        ensure(
            actor != owner.actor && role != Role::Owner,
            ErrorCode::Forbidden,
            "The owner cannot be revoked or transferred by a member update",
        )?;
        let mut saved = room.saved.clone();
        let member = saved
            .members
            .get_mut(actor)
            .ok_or_else(|| HostError::new(ErrorCode::NotFound, "Member not found"))?;
        member.member.role = role;
        member.member.revoked = revoked;
        self.persist(&saved.id, &saved, fence)?;
        state.rooms.get_mut(&owner.room).unwrap().saved = saved;
        Ok(())
    }
    pub fn exchange(&self, credentials: &Credentials, exchange: Exchange) -> Result<ExchangeReply> {
        self.exchange_checked(credentials, exchange, &no_cancel)
    }
    pub fn exchange_checked(
        &self,
        credentials: &Credentials,
        exchange: Exchange,
        fence: &dyn CommitFence,
    ) -> Result<ExchangeReply> {
        let mut state = self.lock()?;
        let room = Self::room(&state, &credentials.room)?;
        let role = Self::authenticate(&room.saved, credentials)?.role;
        ensure(
            room.saved.epoch == exchange.epoch,
            ErrorCode::EpochChanged,
            "Room was checkpointed; fetch the new snapshot and review unsent edits before rejoining",
        )?;
        ensure(
            exchange.known.len() <= 256,
            ErrorCode::Limit,
            "Version vector is too large",
        )?;
        ensure(
            exchange.message.operations.is_empty() || role != Role::Viewer,
            ErrorCode::Forbidden,
            "Viewers cannot change the document",
        )?;
        ensure(
            exchange
                .message
                .operations
                .iter()
                .all(|operation| operation.id.actor == credentials.actor),
            ErrorCode::Forbidden,
            "Operations must belong to the authenticated actor",
        )?;
        let mut replica = room.replica.clone();
        let prior_count = replica.operations().count();
        replica.merge(exchange.message)?;
        if let Some(presence) = exchange.presence {
            ensure(
                presence.actor == credentials.actor,
                ErrorCode::Forbidden,
                "Presence must belong to the authenticated actor",
            )?;
            replica.receive_presence(presence, self.started.elapsed().as_millis() as u64)?;
        }
        let now = self.started.elapsed().as_millis() as u64;
        replica.prune_presence(now, PRESENCE_TTL_MS);
        if replica.operations().count() != prior_count {
            let mut saved = room.saved.clone();
            saved.snapshot = replica.snapshot();
            self.persist(&saved.id, &saved, fence)?;
            state.rooms.get_mut(&credentials.room).unwrap().saved = saved;
        } else {
            fence.check()?;
        }
        let room = state.rooms.get_mut(&credentials.room).unwrap();
        room.replica = replica;
        Ok(ExchangeReply {
            epoch: room.saved.epoch,
            version: room.replica.version(),
            message: room.replica.changes_since_bounded(
                &exchange.known,
                None,
                MAX_MESSAGE_BYTES,
            )?,
            conflicts: room.replica.conflicts().to_vec(),
            presence: room
                .replica
                .presence(now, PRESENCE_TTL_MS)
                .into_iter()
                .filter(|p| {
                    room.saved
                        .members
                        .get(&p.actor)
                        .is_some_and(|m| !m.member.revoked)
                })
                .cloned()
                .collect(),
            role,
        })
    }
    /// An owner checkpoints only an exact observed vector. Old clients receive EpochChanged;
    /// their offline edits remain intact until they explicitly rebase/discard them.
    pub fn checkpoint(
        &self,
        owner: &Credentials,
        epoch: u64,
        expected: &VersionVector,
    ) -> Result<RoomState> {
        self.checkpoint_checked(owner, epoch, expected, &no_cancel)
    }
    pub fn checkpoint_checked(
        &self,
        owner: &Credentials,
        epoch: u64,
        expected: &VersionVector,
        fence: &dyn CommitFence,
    ) -> Result<RoomState> {
        self.checkpoint_resolved_checked(owner, epoch, expected, None, fence)
    }
    /// Adopt an explicitly reviewed source at an exact room version. Historical losing
    /// operations remain in the old peer checkpoint until each participant reviews the new epoch.
    pub fn checkpoint_resolved(
        &self,
        owner: &Credentials,
        epoch: u64,
        expected: &VersionVector,
        source: &str,
    ) -> Result<RoomState> {
        self.checkpoint_resolved_checked(owner, epoch, expected, Some(source), &no_cancel)
    }
    pub fn checkpoint_resolved_checked(
        &self,
        owner: &Credentials,
        epoch: u64,
        expected: &VersionVector,
        resolved_source: Option<&str>,
        fence: &dyn CommitFence,
    ) -> Result<RoomState> {
        let mut state = self.lock()?;
        let room = Self::room(&state, &owner.room)?;
        ensure(
            Self::authenticate(&room.saved, owner)?.role == Role::Owner,
            ErrorCode::Forbidden,
            "Only the owner can checkpoint",
        )?;
        ensure(
            room.saved.epoch == epoch && room.replica.version() == *expected,
            ErrorCode::Conflict,
            "Room advanced; synchronize before checkpointing",
        )?;
        ensure(
            !room.replica.has_sequence_gaps(),
            ErrorCode::Conflict,
            "Room history has sequence gaps; receive missing operations before checkpointing",
        )?;
        ensure(
            room.replica.conflicts().is_empty() || resolved_source.is_some(),
            ErrorCode::Conflict,
            "Review and supply resolved_source to checkpoint a log containing conflicts",
        )?;
        let mut saved = room.saved.clone();
        saved.epoch = saved
            .epoch
            .checked_add(1)
            .ok_or_else(|| HostError::invalid("Epoch exhausted"))?;
        let replica = DocumentReplica::new(
            "@host",
            saved.snapshot.document_id.clone(),
            resolved_source.unwrap_or_else(|| room.replica.source()),
        )?;
        saved.snapshot = replica.snapshot();
        self.persist(&saved.id, &saved, fence)?;
        let room = state.rooms.get_mut(&owner.room).unwrap();
        *room = LiveRoom { saved, replica };
        Ok(Self::view(room, Role::Owner))
    }
}
