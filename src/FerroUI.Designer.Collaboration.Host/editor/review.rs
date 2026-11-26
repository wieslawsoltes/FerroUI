use super::EditorRoom;
use crate::model::*;
use crate::{load_private, save_private};
use ferroui_designer_collaboration::DocumentReplica;
use serde::Serialize;

#[derive(Serialize)]
pub struct RoomReview<'a> {
    pub revision: String,
    pub source: &'a str,
    pub rebase: bool,
}

pub(super) struct ReviewTarget<'a> {
    pub replica: &'a DocumentReplica,
    pub rebase: bool,
    epoch: u64,
    role: Role,
}

impl ReviewTarget<'_> {
    pub(super) fn revision(&self) -> String {
        // This token is also invalidated by same-source operations, role changes and checkpoints.
        let identity = serde_json::to_vec(&(
            self.replica.document_id(),
            self.replica.actor(),
            self.epoch,
            self.replica.version(),
            hash(self.replica.source().as_bytes()),
            self.role,
            self.rebase,
        ))
        .expect("room identity is serializable");
        hash(&identity)
    }
}

impl EditorRoom {
    pub fn review(&self) -> Result<RoomReview<'_>> {
        let target = self.review_target().ok_or_else(|| {
            HostError::new(
                ErrorCode::Conflict,
                "Wait for the current room source to synchronize",
            )
        })?;
        Ok(RoomReview {
            revision: target.revision(),
            source: target.replica.source(),
            rebase: target.rebase,
        })
    }

    pub(super) fn review_target(&self) -> Option<ReviewTarget<'_>> {
        if self.blocked() {
            return None;
        }
        if let Some(pending) = &self.pending {
            return Some(ReviewTarget {
                replica: &pending.replica,
                rebase: true,
                epoch: pending.state.epoch,
                role: pending.state.role,
            });
        }
        self.peer.is_synchronized().then(|| ReviewTarget {
            replica: self.peer.document(),
            rebase: false,
            epoch: self.peer.epoch(),
            role: self.role(),
        })
    }

    pub(super) fn require_target(&self, expected: &str) -> Result<ReviewTarget<'_>> {
        self.review_target()
            .filter(|target| target.revision() == expected)
            .ok_or_else(|| {
                HostError::new(
                    ErrorCode::Conflict,
                    "The room version changed. Open the current source review.",
                )
            })
    }

    /// Verify the remote part of the review and save the pre-apply buffer for recovery. The UI
    /// must still compare the file identity and editor version before its own Undo transaction.
    pub fn prepare_apply(&mut self, expected: &str, before: &str) -> Result<()> {
        ensure(
            !self.require_target(expected)?.rebase,
            ErrorCode::Conflict,
            "Accept the new room baseline before applying it",
        )?;
        self.confirmed(expected, false)?;
        save_private(
            &self.storage.directory.join("previous-document.json"),
            &before,
        )
    }

    pub fn recovery_source(&self) -> Result<String> {
        load_private(&self.storage.directory.join("previous-document.json"))
    }

    /// Explicitly adopt a new epoch (or a viewer downgrade with an outbox). An editor can provide
    /// a reviewed resolution. The old outbox is archived before the new checkpoint replaces it.
    pub fn resolve(&mut self, expected: &str, local: &str, resolved: Option<&str>) -> Result<()> {
        ensure(
            self.require_target(expected)?.rebase,
            ErrorCode::Conflict,
            "There is no replacement room baseline to review",
        )?;
        let (state, remote) = self.confirmed(expected, true)?;
        let source = resolved.unwrap_or(&remote);
        if source != remote {
            ensure(
                state.role != Role::Viewer,
                ErrorCode::Forbidden,
                "Viewers can adopt the room baseline. An editor or owner can publish a resolution.",
            )?;
            ferroui_designer_core::Document::new(source).map_err(HostError::invalid)?;
        }
        self.archive(local)?;
        self.peer.rebase(state.clone(), &remote, source)?;
        self.observe(state)
    }

    /// The server validates the same vector at its durable checkpoint commit.
    pub fn checkpoint(
        &mut self,
        expected: &str,
        local: &str,
        resolved: Option<String>,
    ) -> Result<()> {
        self.require_owner()?;
        ensure(
            !self.require_target(expected)?.rebase,
            ErrorCode::Conflict,
            "Resolve the pending room baseline first",
        )?;
        let (old, _) = self.confirmed(expected, false)?;
        let version = self.peer.document().version();
        if let Some(source) = &resolved {
            ferroui_designer_core::Document::new(source.clone()).map_err(HostError::invalid)?;
        }
        self.archive(local)?;
        let state: RoomState = self.peer.request_room(|credentials| Request::Checkpoint {
            credentials,
            epoch: old.epoch,
            expected: version,
            resolved_source: resolved,
        })?;
        let replica =
            DocumentReplica::from_snapshot(self.peer.document().actor(), state.snapshot.clone())?;
        // This response is the durable result of the local owner's explicitly reviewed action.
        self.peer
            .rebase(state.clone(), replica.source(), replica.source())?;
        self.observe(state)
    }

    pub(super) fn confirmed(
        &mut self,
        expected: &str,
        rebase: bool,
    ) -> Result<(RoomState, String)> {
        let state = self.remote()?;
        let replica =
            DocumentReplica::from_snapshot(self.peer.document().actor(), state.snapshot.clone())?;
        let target = ReviewTarget {
            replica: &replica,
            rebase,
            epoch: state.epoch,
            role: state.role,
        };
        if target.revision() != expected {
            self.observe(state)?;
            return Err(HostError::new(
                ErrorCode::Conflict,
                "The room changed while reviewing. Refresh the source diff.",
            ));
        }
        Ok((state, replica.source().into()))
    }

    fn archive(&self, local: &str) -> Result<()> {
        save_private(
            &self.storage.directory.join("previous-peer.json"),
            &self.peer.checkpoint(),
        )?;
        save_private(
            &self.storage.directory.join("previous-document.json"),
            &local,
        )
    }
}
