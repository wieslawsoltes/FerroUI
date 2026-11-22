//! Room coordination for hosts that own their editor buffers and undo stacks.
//! The room projects immutable versions. Adopting a version into a buffer is a host transaction.
mod open;
mod review;
#[cfg(test)]
mod tests;

pub use open::{OpenRoom, StartRoom};
pub use review::RoomReview;

use crate::ReconnectingPeer;
use crate::model::*;
use ferroui_designer_collaboration::{Conflict, DocumentReplica, Presence, SharedEdit};
use open::RoomStorage;
use serde::Serialize;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomPhase {
    Connected,
    Synchronizing,
    Reconnecting,
    Review,
    Conflict,
    Blocked,
}

#[derive(Serialize)]
pub struct RoomStatus<'a> {
    title: &'a str,
    actor: &'a str,
    role: Role,
    phase: RoomPhase,
    message: Option<&'a str>,
    members: &'a [Member],
    online: &'a [String],
    conflicts: &'a [Conflict],
    pending: usize,
    revision: Option<String>,
}

struct PendingReview {
    state: RoomState,
    replica: DocumentReplica,
}

pub struct EditorRoom {
    peer: ReconnectingPeer,
    storage: RoomStorage,
    title: String,
    display_name: String,
    members: Vec<Member>,
    current_role: Role,
    online: Vec<String>,
    sequence: u64,
    next_members: Instant,
    pending: Option<PendingReview>,
    problem: Option<HostError>,
    connected: bool,
}

impl EditorRoom {
    fn new(
        peer: ReconnectingPeer,
        storage: RoomStorage,
        title: String,
        display_name: String,
        members: Vec<Member>,
    ) -> Self {
        let current_role = peer.role();
        Self {
            peer,
            storage,
            title,
            display_name,
            members,
            current_role,
            online: Vec::new(),
            sequence: 0,
            next_members: Instant::now(),
            pending: None,
            problem: None,
            connected: false,
        }
    }

    pub fn status(&self) -> RoomStatus<'_> {
        let conflicts = self.peer.document().conflicts();
        let phase = if self.blocked() {
            RoomPhase::Blocked
        } else if self.pending.is_some() {
            RoomPhase::Review
        } else if !self.connected {
            RoomPhase::Reconnecting
        } else if !conflicts.is_empty() {
            RoomPhase::Conflict
        } else if !self.peer.is_synchronized() {
            RoomPhase::Synchronizing
        } else {
            RoomPhase::Connected
        };
        RoomStatus {
            title: &self.title,
            actor: self.peer.document().actor(),
            role: self.role(),
            phase,
            message: self.problem.as_ref().map(|error| error.message.as_str()),
            members: &self.members,
            online: &self.online,
            conflicts,
            pending: self.peer.pending_operations(),
            revision: self.review_target().map(|target| target.revision()),
        }
    }

    /// Poll in the host's background worker. No check writes the editor buffer.
    pub fn tick(&mut self, refresh: bool) {
        if self.blocked()
            && !self
                .problem
                .as_ref()
                .is_some_and(|error| error.code == ErrorCode::Unauthorized)
        {
            return;
        }
        if self.pending.is_some() || self.blocked() {
            if refresh || Instant::now() >= self.next_members {
                self.refresh_members();
            }
            return;
        }
        let presence = Some(self.presence());
        let result = if refresh {
            Some(self.peer.sync(presence))
        } else {
            self.peer.poll(presence)
        };
        match result {
            Some(Ok(reply)) => {
                self.connected = true;
                self.problem = None;
                self.current_role = reply.role;
                self.online = reply
                    .presence
                    .into_iter()
                    .map(|presence| presence.actor)
                    .collect();
                if refresh || Instant::now() >= self.next_members {
                    self.refresh_members();
                }
            }
            Some(Err(error)) => {
                let need_state =
                    matches!(error.code, ErrorCode::EpochChanged | ErrorCode::Forbidden);
                self.record_error(error);
                if need_state {
                    self.refresh_members();
                }
            }
            None => {}
        }
    }

    pub fn publish(&mut self, expected_revision: &str, source: String) -> Result<()> {
        self.require_editor()?;
        // A publish keeps its reviewed base in the operation, including while offline.
        let target = self.require_target(expected_revision)?;
        ensure(
            !target.rebase,
            ErrorCode::Conflict,
            "Review the new room baseline before publishing",
        )?;
        let previous = target.replica.source().to_owned();
        if source != previous {
            ferroui_designer_core::Document::new(source.clone()).map_err(HostError::invalid)?;
            if self.connected {
                match self.confirmed(expected_revision, false) {
                    Ok(_) => {}
                    Err(error) if error.code == ErrorCode::Transport => self.record_error(error),
                    Err(error) => return Err(error),
                }
            }
            self.peer.submit(
                "Publish editor source",
                vec![SharedEdit::ReplaceSource {
                    expected_source: previous,
                    source,
                }],
            )?;
        }
        Ok(())
    }

    pub fn invite(&mut self, role: Role) -> Result<String> {
        self.require_owner()?;
        ensure(
            role != Role::Owner,
            ErrorCode::Invalid,
            "Invite an editor or viewer",
        )?;
        let token = Secret::generate()?;
        let _: serde_json::Value = self.peer.request_room(|credentials| Request::Invite {
            credentials,
            role,
            token: token.clone(),
            expires_in: 86400,
            uses: 32,
        })?;
        let mut endpoint = self.peer.endpoint().clone();
        if let Some(url) = &self.storage.public_url {
            endpoint.url.clone_from(url);
        }
        Invitation {
            protocol: HOST_PROTOCOL,
            endpoint,
            room: self.peer.room_id().into(),
            invite: token,
        }
        .encode()
    }

    pub fn set_member(&mut self, actor: String, role: Role, revoked: bool) -> Result<()> {
        self.require_owner()?;
        let _: serde_json::Value = self.peer.request_room(|credentials| Request::SetMember {
            credentials,
            actor,
            role,
            revoked,
        })?;
        self.refresh_members();
        Ok(())
    }

    fn presence(&mut self) -> Presence {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.sequence = now.max(self.sequence.saturating_add(1));
        Presence {
            actor: self.peer.document().actor().into(),
            sequence: self.sequence,
            display_name: self.display_name.clone(),
            selection: Vec::new(),
            cursor: None,
        }
    }

    fn refresh_members(&mut self) {
        self.next_members = Instant::now() + Duration::from_secs(10);
        let result = self.remote().and_then(|state| self.observe(state));
        if let Err(error) = result {
            self.record_error(error);
        }
    }

    fn remote(&mut self) -> Result<RoomState> {
        self.peer
            .request_room(|credentials| Request::State { credentials })
    }

    fn observe(&mut self, state: RoomState) -> Result<()> {
        self.title.clone_from(&state.title);
        self.members.clone_from(&state.members);
        self.current_role = state.role;
        if state.epoch != self.peer.epoch()
            || (state.role == Role::Viewer && self.peer.pending_operations() != 0)
        {
            let replica = DocumentReplica::from_snapshot(
                self.peer.document().actor(),
                state.snapshot.clone(),
            )?;
            self.pending = Some(PendingReview { state, replica });
        } else {
            self.pending = None;
        }
        self.connected = true;
        self.problem = None;
        Ok(())
    }

    fn record_error(&mut self, error: HostError) {
        self.connected = false;
        self.online.clear();
        self.problem = Some(error);
    }

    fn role(&self) -> Role {
        self.current_role
    }

    fn blocked(&self) -> bool {
        self.problem.as_ref().is_some_and(|error| {
            matches!(
                error.code,
                ErrorCode::Unauthorized | ErrorCode::Indeterminate | ErrorCode::Storage
            )
        })
    }

    fn require_editor(&self) -> Result<()> {
        ensure(
            !self.blocked(),
            ErrorCode::Forbidden,
            "Review the connection status before publishing",
        )?;
        ensure(
            self.role() != Role::Viewer,
            ErrorCode::Forbidden,
            "Viewers can review room source. An editor or owner can publish.",
        )
    }

    fn require_owner(&self) -> Result<()> {
        self.require_editor()?;
        ensure(
            self.role() == Role::Owner,
            ErrorCode::Forbidden,
            "Only the room owner can manage members or checkpoint the room",
        )
    }
}
