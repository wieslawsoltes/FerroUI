use crate::animation_session::AnimationSession;
use crate::collaboration::SessionCollaboration;
use crate::runtime::{PreviewStatus, RuntimeNode, RuntimePreview};
use ferroui_base::{Rect, Ref, Visual};
use ferroui_controls::ContentControl;
use ferroui_designer_collaboration::{
    Conflict, Cursor, DocumentReplica, MergeReport, Operation, Presence, SharedEdit, Snapshot,
    SyncMessage, VersionVector,
};
use ferroui_designer_core::{ChangeSet, DesignerError, Document, Edit, NodeId, SourceUpdate};
use ferroui_inspect::TreeKind;
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// Shared UI-thread state. All authored changes pass through the same document history;
/// runtime load errors preserve both source and the last successfully rendered tree.
pub struct DesignerSession {
    pub(crate) document: Document,
    pub(crate) selection: Vec<NodeId>,
    pub(crate) preview: RuntimePreview,
    pub(crate) animation: AnimationSession,
    pub(crate) interactions: crate::interaction_session::InteractionSession,
    pub(crate) generation: u64,
    pub(crate) definition_scope_epoch: u64,
    pub(crate) layers: crate::layer_state::LayerView,
    collaboration: Option<SessionCollaboration>,
    read_only: bool,
    layout_tools: ferroui_designer_tools::LayoutRegistry,
    property_tools: ferroui_designer_tools::PropertyRegistry,
    presence_started: std::time::Instant,
}

impl DesignerSession {
    pub fn new(
        source: impl Into<String>,
        preview_host: Ref<ContentControl>,
    ) -> Result<Self, String> {
        let id = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);
        Self::with_uri(
            source,
            preview_host,
            format!("ferres://FerroUI.Designer/Document{id}.xaml"),
        )
    }

    /// Supply the original document URI when includes and relative resource references matter.
    pub fn with_uri(
        source: impl Into<String>,
        preview_host: Ref<ContentControl>,
        uri: String,
    ) -> Result<Self, String> {
        let source = source.into();
        let (document, invalid_source) = match Document::new(source.clone()) {
            Ok(document) => (document, None),
            Err(_) => (
                Document::new(r#"<Canvas xmlns="https://github.com/ferroui" />"#)
                    .expect("valid neutral document"),
                Some(source),
            ),
        };
        ferroui_xaml_interactivity::register_types();
        ferroui_xaml_interactions::register_types();
        ferroui_xaml_interactions_custom::register_types();
        let preview = RuntimePreview::new(preview_host, uri, &document)?;
        let mut result = Self {
            document,
            preview,
            selection: Vec::new(),
            animation: AnimationSession::new()?,
            interactions: crate::interaction_session::InteractionSession::default(),
            generation: 0,
            definition_scope_epoch: 0,
            layers: crate::layer_state::LayerView::default(),
            collaboration: None,
            read_only: false,
            layout_tools: crate::definition_manipulation::layout_registry(),
            property_tools: ferroui_designer_tools::PropertyRegistry::built_in(),
            presence_started: std::time::Instant::now(),
        };
        result.import_animations();
        if let Some(source) = invalid_source {
            result.update_source(source);
        }
        Ok(result)
    }

    /// Register custom panel planners without replacing the native manipulation surface.
    pub fn layout_tools(&self) -> &ferroui_designer_tools::LayoutRegistry {
        &self.layout_tools
    }
    pub fn layout_tools_mut(&mut self) -> &mut ferroui_designer_tools::LayoutRegistry {
        &mut self.layout_tools
    }
    pub fn property_tools(&self) -> &ferroui_designer_tools::PropertyRegistry {
        &self.property_tools
    }
    pub fn property_tools_mut(&mut self) -> &mut ferroui_designer_tools::PropertyRegistry {
        &mut self.property_tools
    }
    pub fn align_selection(
        &mut self,
        alignment: ferroui_designer_tools::Alignment,
    ) -> Result<(), String> {
        crate::workspace_tools::align(self, alignment)
    }
    pub fn distribute_selection(
        &mut self,
        axis: ferroui_designer_tools::Axis,
    ) -> Result<(), String> {
        crate::workspace_tools::distribute(self, axis)
    }

    pub fn document_uri(&self) -> &str {
        self.preview.uri()
    }
    /// Record a successful write of the current authored buffer.
    pub fn mark_saved(&mut self) {
        self.document.mark_saved();
        self.generation += 1;
    }
    /// Change the base URI after Save As without replacing source history or collaboration.
    pub fn set_document_uri(&mut self, uri: String) -> Result<(), String> {
        if uri == self.preview.uri() {
            return Ok(());
        }
        self.preview.clear_animation();
        self.animation.native.clear();
        self.preview.replace_document(uri, &self.document)?;
        self.animation.player.stop();
        self.forget_interaction_preview();
        self.import_animations();
        self.refresh_layer_visuals();
        self.generation += 1;
        Ok(())
    }
    /// Open a different document, clearing selection/history/sharing. Revision stays monotonic
    /// so automation holding a revision from the previous document cannot edit the new one.
    /// Hosts must resolve unsaved work before calling this explicit replacement command.
    pub fn open_document(&mut self, source: String, uri: String) -> Result<(), String> {
        let mut candidate = self.document.clone();
        candidate.update_source("Open document", source);
        candidate.set_history_limit(0);
        candidate.set_history_limit(256);
        candidate.mark_saved();
        self.preview.clear_animation();
        self.animation.native.clear();
        self.preview.replace_document(uri, &candidate)?;
        self.layers.reset();
        self.document = candidate;
        self.selection.clear();
        self.collaboration = None;
        self.read_only = false;
        self.animation.recording = false;
        self.animation.player.stop();
        self.animation.clear_program_state();
        self.forget_interaction_preview();
        self.import_animations();
        self.generation += 1;
        Ok(())
    }

    /// Restore a tab buffer, including its undo history, onto this host's revision sequence.
    pub fn restore_document(&mut self, mut document: Document, uri: String) -> Result<(), String> {
        let minimum = self
            .document
            .revision()
            .checked_add(1)
            .ok_or("Document revision exhausted")?;
        document
            .advance_revision_to(minimum)
            .map_err(|error| error.to_string())?;
        self.preview.clear_animation();
        self.animation.native.clear();
        self.preview.replace_document(uri, &document)?;
        self.layers.reset();
        self.document = document;
        self.selection.clear();
        self.collaboration = None;
        self.read_only = false;
        self.animation.recording = false;
        self.animation.player.stop();
        self.animation.clear_program_state();
        self.forget_interaction_preview();
        self.import_animations();
        self.generation += 1;
        Ok(())
    }

    /// Viewer sessions may inspect and play previews while all authored edits are rejected.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
    pub fn set_read_only(&mut self, value: bool) {
        self.read_only = value;
        if value {
            self.animation.recording = false;
        }
        self.generation += 1;
    }
    pub(crate) fn ensure_writable(&self) -> Result<(), DesignerError> {
        if self.read_only {
            Err(DesignerError::InvalidEdit(
                "This collaboration role has read-only access".into(),
            ))
        } else {
            Ok(())
        }
    }
    fn rejected_source_update(&self) -> SourceUpdate {
        SourceUpdate {
            change: ChangeSet {
                label: "Rejected source edit".into(),
                previous_revision: self.document.revision(),
                revision: self.document.revision(),
                inserted: Vec::new(),
                removed: Vec::new(),
                changed: Vec::new(),
                patches: Vec::new(),
                valid: self.document.is_valid(),
            },
            valid: false,
            diagnostics: vec![ferroui_designer_core::Diagnostic {
                message: "This collaboration role has read-only access".into(),
                line: 1,
                column: 1,
                span: ferroui_designer_core::SourceSpan::new(0, 0),
            }],
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn selection(&self) -> &[NodeId] {
        &self.selection
    }
    /// Changes on selection and timeline edits as well as document revisions.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn can_manipulate(&self) -> bool {
        !self.read_only
            && self.document.is_valid()
            && matches!(self.preview.status, PreviewStatus::Ready { revision } if revision == self.document.revision())
    }
    pub fn preview_notices(&self) -> &[String] {
        &self.preview.notices
    }
    pub fn preview_status(&self) -> &PreviewStatus {
        &self.preview.status
    }
    pub fn preview_host(&self) -> Ref<ContentControl> {
        self.preview.host.clone()
    }
    pub fn select(&mut self, ids: impl IntoIterator<Item = NodeId>) {
        let mut seen = BTreeSet::new();
        self.selection = ids
            .into_iter()
            .filter(|id| {
                self.document
                    .ast()
                    .node(*id)
                    .is_some_and(|node| node.is_element())
                    && seen.insert(*id)
            })
            .collect();
        self.generation += 1;
    }

    pub fn apply(
        &mut self,
        label: impl Into<String>,
        edits: Vec<Edit>,
    ) -> Result<ChangeSet, DesignerError> {
        let change = self.commit_document(label.into(), edits)?;
        self.changed(true);
        Ok(change)
    }

    pub fn apply_at(
        &mut self,
        expected_revision: u64,
        label: impl Into<String>,
        edits: Vec<Edit>,
    ) -> Result<ChangeSet, DesignerError> {
        if self.document.revision() != expected_revision {
            return Err(DesignerError::RevisionConflict {
                expected: expected_revision,
                actual: self.document.revision(),
            });
        }
        let change = self.commit_document(label.into(), edits)?;
        self.changed(true);
        Ok(change)
    }

    pub fn update_source(&mut self, source: impl Into<String>) -> SourceUpdate {
        if self.read_only {
            return self.rejected_source_update();
        }
        let update = self.stage_source(source);
        let _ = self.apply_staged_preview();
        update
    }

    /// Accept source-buffer text immediately; a host can debounce only expensive runtime reload.
    pub fn stage_source(&mut self, source: impl Into<String>) -> SourceUpdate {
        if self.read_only {
            return self.rejected_source_update();
        }
        if let Some(collaboration) = self.collaboration.as_mut() {
            if collaboration.draft_base.is_none() {
                collaboration.draft_base = Some(collaboration.replica.source().to_owned());
            }
        }
        let update = self.document.update_source("Edit XAML", source);
        self.animation.player.pause();
        self.animation.native.clear();
        self.selection
            .retain(|id| self.document.ast().node(*id).is_some());
        if !update.valid {
            self.preview.status = PreviewStatus::InvalidSource {
                revision: self.document.revision(),
            };
        }
        self.generation += 1;
        update
    }
    pub fn apply_staged_preview(&mut self) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let result = self.publish_source_draft();
        self.changed(true);
        result
    }

    pub fn undo(&mut self) -> Result<ChangeSet, DesignerError> {
        self.ensure_writable()?;
        if self.collaboration.is_some() {
            return self.collaborative_history(false);
        }
        let change = self.document.undo()?;
        self.changed(true);
        Ok(change)
    }
    pub fn redo(&mut self) -> Result<ChangeSet, DesignerError> {
        self.ensure_writable()?;
        if self.collaboration.is_some() {
            return self.collaborative_history(true);
        }
        let change = self.document.redo()?;
        self.changed(true);
        Ok(change)
    }

    pub fn can_undo(&self) -> bool {
        if self.read_only {
            return false;
        }
        self.collaboration.as_ref().map_or_else(
            || self.document.can_undo(),
            |state| state.draft_base.is_none() && !state.undo.is_empty(),
        )
    }
    pub fn can_redo(&self) -> bool {
        if self.read_only {
            return false;
        }
        self.collaboration.as_ref().map_or_else(
            || self.document.can_redo(),
            |state| state.draft_base.is_none() && !state.redo.is_empty(),
        )
    }

    pub(crate) fn commit_document(
        &mut self,
        label: String,
        edits: Vec<Edit>,
    ) -> Result<ChangeSet, DesignerError> {
        self.ensure_writable()?;
        self.publish_source_draft()
            .map_err(DesignerError::InvalidEdit)?;
        let mut document = self.document.clone();
        let change = document.apply(label.clone(), edits.clone())?;
        if change.is_empty() {
            return Ok(change);
        }
        let mut collaboration = self.collaboration.clone();
        if let Some(state) = collaboration.as_mut() {
            let before = self.document.source().to_owned();
            let operation = state
                .replica
                .submit_document_edits(&self.document, label, edits)
                .map_err(|error| DesignerError::InvalidEdit(error.to_string()))?;
            if state.replica.source() != document.source() {
                return Err(DesignerError::InvalidEdit(
                    "Collaboration projection differs from the staged edit".into(),
                ));
            }
            state.local(before, operation);
        }
        self.document = document;
        self.collaboration = collaboration;
        Ok(change)
    }

    /// Starts a transport-independent collaboration session. Hosts authenticate peers and carry
    /// SyncMessage envelopes over their existing transport; constructing a session opens no socket.
    pub fn enable_collaboration(
        &mut self,
        actor: impl Into<String>,
        document_id: impl Into<String>,
    ) -> Result<(), String> {
        if !self.document.is_valid() {
            return Err("Repair the XAML source before creating a collaboration baseline".into());
        }
        if self.collaboration.is_some() {
            return Err("Collaboration is already enabled".into());
        }
        self.collaboration = Some(SessionCollaboration::new(
            DocumentReplica::new(actor, document_id, self.document.source())
                .map_err(|error| error.to_string())?,
        ));
        self.generation += 1;
        Ok(())
    }
    pub fn join_collaboration(
        &mut self,
        actor: impl Into<String>,
        snapshot: Snapshot,
    ) -> Result<(), String> {
        if self.collaboration.is_some() {
            return Err("Collaboration is already enabled".into());
        }
        let replica =
            DocumentReplica::from_snapshot(actor, snapshot).map_err(|error| error.to_string())?;
        if replica.source() != self.document.source() {
            return Err("Open the snapshot projection before joining; local source is never discarded implicitly".into());
        }
        self.collaboration = Some(SessionCollaboration::new(replica));
        self.generation += 1;
        Ok(())
    }
    /// Detach transport/sharing while preserving the authored document and its history.
    pub fn leave_collaboration(&mut self) {
        self.collaboration = None;
        self.read_only = false;
        self.generation += 1;
    }

    /// Identity of the currently attached replica, independent of document URI changes.
    pub fn collaboration_actor(&self) -> Option<&str> {
        self.collaboration
            .as_ref()
            .map(|state| state.replica.actor())
    }
    pub fn collaboration_document_id(&self) -> Option<&str> {
        self.collaboration
            .as_ref()
            .map(|state| state.replica.document_id())
    }

    pub fn collaboration_version(&self) -> Option<VersionVector> {
        self.collaboration
            .as_ref()
            .map(|state| state.replica.version())
    }
    pub fn collaboration_snapshot(&self) -> Result<Snapshot, String> {
        Ok(self
            .collaboration
            .as_ref()
            .ok_or("Collaboration is not enabled")?
            .replica
            .snapshot())
    }
    pub fn collaboration_message(&self, operations: Vec<Operation>) -> Result<SyncMessage, String> {
        Ok(self
            .collaboration
            .as_ref()
            .ok_or("Collaboration is not enabled")?
            .replica
            .message(operations))
    }
    pub fn collaboration_changes_since(
        &self,
        version: &VersionVector,
    ) -> Result<SyncMessage, String> {
        Ok(self
            .collaboration
            .as_ref()
            .ok_or("Collaboration is not enabled")?
            .replica
            .changes_since(version))
    }
    /// Page only this writer's outbox, keeping offline uploads within the collaboration wire
    /// budget without allowing other actors' operation payloads to consume the page.
    pub fn collaboration_outbox_page(
        &self,
        version: &VersionVector,
    ) -> Result<SyncMessage, String> {
        let replica = &self
            .collaboration
            .as_ref()
            .ok_or("Collaboration is not enabled")?
            .replica;
        replica
            .changes_since_bounded(
                version,
                Some(replica.actor()),
                ferroui_designer_collaboration::MAX_MESSAGE_BYTES,
            )
            .map_err(|error| error.to_string())
    }
    pub fn collaboration_pending_operations(
        &self,
        version: &VersionVector,
    ) -> Result<usize, String> {
        let replica = &self
            .collaboration
            .as_ref()
            .ok_or("Collaboration is not enabled")?
            .replica;
        Ok(replica.pending_operation_count(version, replica.actor()))
    }
    pub fn take_collaboration_changes(&mut self) -> Result<SyncMessage, String> {
        let state = self
            .collaboration
            .as_mut()
            .ok_or("Collaboration is not enabled")?;
        let operations = std::mem::take(&mut state.outgoing);
        Ok(state.replica.message(operations))
    }
    pub fn collaboration_conflicts(&self) -> Vec<Conflict> {
        self.collaboration
            .as_ref()
            .map(|state| state.replica.conflicts().to_vec())
            .unwrap_or_default()
    }
    pub fn collaboration_error(&self) -> Option<&str> {
        self.collaboration
            .as_ref()
            .and_then(|state| state.error.as_deref())
    }

    fn publish_source_draft(&mut self) -> Result<(), String> {
        let Some(state) = self.collaboration.as_mut() else {
            return Ok(());
        };
        let Some(expected) = state.draft_base.clone() else {
            return Ok(());
        };
        if !self.document.is_valid() {
            return Ok(());
        }
        if self.document.source() == state.replica.source() {
            state.draft_base = None;
            state.error = None;
            return Ok(());
        }
        let mut candidate = state.clone();
        let result = candidate.replica.submit(
            "Edit XAML",
            vec![SharedEdit::ReplaceSource {
                expected_source: expected.clone(),
                source: self.document.source().to_owned(),
            }],
        );
        match result {
            Ok(operation) => {
                candidate.local(expected, operation);
                *state = candidate;
                Ok(())
            }
            Err(error) => {
                let message = format!(
                    "Source collaboration conflict: {error}. Merge the source explicitly or discard the local draft."
                );
                state.error = Some(message.clone());
                Err(message)
            }
        }
    }

    /// Remote operations are merged atomically. A local source draft remains visible and its
    /// original compare-and-set base is retained; remote changes cannot silently erase it.
    pub fn merge_collaboration(&mut self, message: SyncMessage) -> Result<MergeReport, String> {
        let state = self
            .collaboration
            .as_ref()
            .ok_or("Collaboration is not enabled")?;
        let mut candidate = state.clone();
        let report = candidate
            .replica
            .merge(message)
            .map_err(|error| error.to_string())?;
        if candidate.draft_base.is_some() {
            if candidate.draft_base.as_deref() != Some(candidate.replica.source()) {
                candidate.error=Some("Remote changes arrived while a source draft was being edited; merge or discard the draft explicitly".into());
            }
        } else {
            let mut document = self.document.clone();
            document
                .apply(
                    "Merge collaboration",
                    vec![Edit::ReplaceSource {
                        source: candidate.replica.source().into(),
                    }],
                )
                .map_err(|error| error.to_string())?;
            self.document = document;
        }
        self.collaboration = Some(candidate);
        self.changed(true);
        Ok(report)
    }

    /// Explicitly discard the local draft in favor of the merged collaboration projection.
    pub fn discard_collaboration_draft(&mut self) -> Result<ChangeSet, String> {
        let state = self
            .collaboration
            .as_mut()
            .ok_or("Collaboration is not enabled")?;
        let source = state.replica.source().to_owned();
        state.draft_base = None;
        state.error = None;
        let update = self
            .document
            .update_source("Discard local collaboration draft", source);
        self.changed(true);
        Ok(update.change)
    }
    /// The host presents both sources and obtains an explicit merged result. A version guard
    /// prevents applying that resolution after another peer has advanced the shared document.
    pub fn resolve_collaboration_draft(
        &mut self,
        expected_version: &VersionVector,
        source: String,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let state = self
            .collaboration
            .as_mut()
            .ok_or("Collaboration is not enabled")?;
        if state.replica.version() != *expected_version {
            return Err("Collaboration advanced while resolving the source draft".into());
        }
        Document::new(source.clone()).map_err(|error| error.to_string())?;
        state.draft_base = Some(state.replica.source().to_owned());
        state.error = None;
        self.document
            .update_source("Resolve collaboration source", source);
        self.publish_source_draft()?;
        self.changed(true);
        Ok(())
    }

    fn collaborative_history(&mut self, redo: bool) -> Result<ChangeSet, DesignerError> {
        let state = self.collaboration.as_ref().expect("collaboration");
        if state.draft_base.is_some() {
            return Err(DesignerError::InvalidEdit(
                "Apply or discard the local source draft before shared undo/redo".into(),
            ));
        }
        let mut candidate = state.clone();
        let entry = if redo {
            candidate.redo.last()
        } else {
            candidate.undo.last()
        }
        .cloned()
        .ok_or(if redo {
            DesignerError::NothingToRedo
        } else {
            DesignerError::NothingToUndo
        })?;
        if candidate.replica.version() != entry.expected_version {
            return Err(DesignerError::InvalidEdit(
                "Another collaboration operation arrived; shared undo/redo refuses to overwrite it"
                    .into(),
            ));
        }
        let expected = if redo { &entry.before } else { &entry.after };
        let source = if redo { &entry.after } else { &entry.before };
        if candidate.replica.source() != expected {
            return Err(DesignerError::InvalidEdit(
                "Shared document changed since this history entry".into(),
            ));
        }
        let label = if redo {
            "Redo shared edit"
        } else {
            "Undo shared edit"
        };
        let operation = candidate
            .replica
            .submit(
                label,
                vec![SharedEdit::ReplaceSource {
                    expected_source: expected.clone(),
                    source: source.clone(),
                }],
            )
            .map_err(|error| DesignerError::InvalidEdit(error.to_string()))?;
        let mut document = self.document.clone();
        let change = document.apply(
            label,
            vec![Edit::ReplaceSource {
                source: candidate.replica.source().into(),
            }],
        )?;
        let mut entry = entry;
        entry.expected_version = candidate.replica.version();
        if redo {
            candidate.redo.pop();
            candidate.undo.push(entry);
        } else {
            candidate.undo.pop();
            candidate.redo.push(entry);
        }
        // The next adjacent local history entry remains undoable after our own compensation.
        if let Some(next) = candidate.undo.last_mut() {
            next.expected_version = candidate.replica.version();
        }
        if let Some(next) = candidate.redo.last_mut() {
            next.expected_version = candidate.replica.version();
        }
        candidate.outgoing.push(operation);
        self.document = document;
        self.collaboration = Some(candidate);
        self.changed(true);
        Ok(change)
    }

    /// Receiver-owned monotonic time shared by native overlays and all MCP providers.
    pub fn presence_now(&self) -> u64 {
        self.presence_started
            .elapsed()
            .as_millis()
            .min(u64::MAX as u128) as u64
    }
    pub fn current_presence(&self, ttl_millis: u64) -> Vec<Presence> {
        self.presence(self.presence_now(), ttl_millis)
    }
    pub fn receive_presence_now(&mut self, presence: Presence) -> Result<bool, String> {
        self.receive_presence(presence, self.presence_now())
    }
    /// Map a shared identity into the current authored document without conflating the replica's
    /// node IDs with this host's IDs. Conflicting source drafts deliberately have no peer mapping.
    pub fn presence_source_node(
        &self,
        shared: &ferroui_designer_collaboration::SharedNodeId,
    ) -> Option<NodeId> {
        let state = self.collaboration.as_ref()?;
        if state.replica.source() != self.document.source() {
            return None;
        }
        let node = state
            .replica
            .document()
            .ast()
            .node(state.replica.resolve(shared)?)?;
        self.document
            .ast()
            .elements()
            .find(|candidate| candidate.span == node.span && candidate.name == node.name)
            .map(|node| node.id)
    }

    pub fn local_presence(
        &mut self,
        display_name: String,
        cursor: Option<Cursor>,
    ) -> Result<Presence, String> {
        let state = self
            .collaboration
            .as_mut()
            .ok_or("Collaboration is not enabled")?;
        let selection = self
            .selection
            .iter()
            .map(|id| state.replica.shared_id_in(&self.document, *id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        state.presence_sequence = state
            .presence_sequence
            .checked_add(1)
            .ok_or("Presence sequence exhausted")?;
        let presence = Presence {
            actor: state.replica.actor().into(),
            sequence: state.presence_sequence,
            display_name,
            selection,
            cursor,
        };
        if presence.display_name.len() > 256 || presence.selection.len() > 1024 {
            return Err("Invalid presence update".into());
        }
        if presence
            .cursor
            .as_ref()
            .is_some_and(|cursor| !cursor.x.is_finite() || !cursor.y.is_finite())
        {
            return Err("Presence coordinates must be finite".into());
        }
        Ok(presence)
    }
    pub fn receive_presence(
        &mut self,
        presence: Presence,
        now_millis: u64,
    ) -> Result<bool, String> {
        let state = self
            .collaboration
            .as_mut()
            .ok_or("Collaboration is not enabled")?;
        let changed = state
            .replica
            .receive_presence(presence, now_millis)
            .map_err(|error| error.to_string())?;
        if changed {
            self.generation += 1;
        }
        Ok(changed)
    }
    pub fn presence(&self, now_millis: u64, ttl_millis: u64) -> Vec<Presence> {
        self.collaboration
            .as_ref()
            .map(|state| {
                state
                    .replica
                    .presence(now_millis, ttl_millis)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn changed(&mut self, import_animation: bool) {
        self.animation.native.clear();
        self.forget_interaction_preview();
        self.animation.player.pause();
        self.selection
            .retain(|id| self.document.ast().node(*id).is_some());
        self.preview.reload(&self.document);
        if import_animation {
            self.import_animations();
        }
        self.refresh_layer_visuals();
        self.generation += 1;
    }

    /// A property edit records into the timeline when recording is enabled; ordinary edits
    /// write an attribute. Explicit resource/binding expressions remain ordinary XAML edits.
    pub fn set_property(
        &mut self,
        nodes: &[NodeId],
        name: &str,
        value: &str,
    ) -> Result<(), String> {
        let edits = self
            .property_tools
            .plan(
                &self.document,
                nodes,
                name,
                Some(value),
                ferroui_designer_tools::ExpressionPolicy::ReplaceExpressions,
            )
            .map_err(|e| e.to_string())?;
        if self.animation.recording && !value.starts_with('{') {
            return self.apply_visual("Record animation property", edits);
        }
        self.apply(format!("Set {name}"), edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Explicit reset removes either an attribute or a property element in one undoable edit.
    pub fn reset_property(&mut self, nodes: &[NodeId], name: &str) -> Result<(), String> {
        let mut edits = vec![];
        for id in nodes {
            let element = self
                .document
                .ast()
                .node(*id)
                .ok_or("Unknown property target")?;
            edits.push(Edit::RemoveAttribute {
                node: *id,
                name: name.into(),
            });
            for child in element
                .children
                .iter()
                .filter_map(|id| self.document.ast().node(*id))
            {
                if child.local_name() == name || child.local_name().ends_with(&format!(".{name}")) {
                    edits.push(Edit::RemoveNode { node: child.id });
                }
            }
        }
        self.apply(format!("Reset {name}"), edits)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Visual manipulators use the same recording path as property editors. All keyframes from
    /// one gesture and any required anchor cleanup become a single source/history transaction.
    pub fn apply_visual(&mut self, label: &str, edits: Vec<Edit>) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.animation.recording {
            self.apply(label, edits)
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        if !self.can_manipulate() {
            return Err(
                "Animation recording requires the current document to render successfully".into(),
            );
        }
        let mut candidate = self.animation.timeline.clone();
        let mut ordinary = Vec::new();
        for edit in edits {
            if let Edit::SetAttribute { node, name, value } = &edit {
                if value.starts_with('{') {
                    ordinary.push(edit);
                    continue;
                }
                let value = self.animation_value_for_target(*node, name, value)?;
                let track_target = self.recording_target(*node);
                if candidate.track(track_target, name).is_none() {
                    let baseline = self
                        .document
                        .ast()
                        .node(*node)
                        .and_then(|node| node.attribute(name))
                        .and_then(|text| self.animation_value_for_target(*node, name, text).ok())
                        .or_else(|| {
                            self.preview.visual(*node).and_then(|visual| {
                                let property =
                                    ferroui_inspect::Inspector::property(&visual, name).ok()?;
                                let value = visual.get_value_untyped(property);
                                self.animation_value_for_target(
                                    *node,
                                    name,
                                    &ferroui_inspect::display_value(Some(&value)),
                                )
                                .ok()
                            })
                        })
                        .filter(|base| base.kind() == value.kind())
                        .unwrap_or_else(|| value.clone());
                    candidate
                        .record(track_target, name, 0.0, baseline)
                        .map_err(|error| error.to_string())?;
                    candidate
                        .record(track_target, name, candidate.duration, value.clone())
                        .map_err(|error| error.to_string())?;
                }
                candidate
                    .record(
                        track_target,
                        name,
                        self.animation.player.time().min(candidate.duration),
                        value,
                    )
                    .map_err(|error| error.to_string())?;
            } else {
                ordinary.push(edit);
            }
        }
        self.write_timeline_edits(candidate, label, ordinary)?;
        self.seek(self.animation.player.time())
    }

    pub fn runtime_tree(&self, kind: TreeKind) -> Vec<RuntimeNode> {
        self.preview.tree(kind)
    }
    pub fn selection_bounds(&self, relative_to: &Ref<Visual>) -> Vec<(NodeId, Rect)> {
        self.selection
            .iter()
            .filter_map(|id| {
                self.preview
                    .bounds(*id, relative_to)
                    .map(|rect| (*id, rect))
            })
            .collect()
    }
    pub fn refresh_runtime_mapping(&mut self) {
        self.preview.rebuild_map(&self.document);
        self.refresh_layer_visuals();
    }
}
