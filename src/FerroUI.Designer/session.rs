use crate::collaboration::SessionCollaboration;
use crate::runtime::{PreviewStatus, RuntimeNode, RuntimePreview};
use ferroui_base::{Rect, Ref, Visual};
use ferroui_controls::ContentControl;
use ferroui_designer_animation::{AnimationValue, CompiledTimeline, Player, Timeline, ValueKind};
use ferroui_designer_collaboration::{
    Conflict, Cursor, DocumentReplica, MergeReport, Operation, Presence, SharedEdit, Snapshot,
    SyncMessage, VersionVector,
};
use ferroui_designer_core::{ChangeSet, DesignerError, Document, Edit, NodeId, SourceUpdate};
use ferroui_inspect::TreeKind;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

/// Shared UI-thread state. All authored changes pass through the same document history;
/// runtime load errors preserve both source and the last successfully rendered tree.
pub struct DesignerSession {
    document: Document,
    selection: Vec<NodeId>,
    pub(crate) preview: RuntimePreview,
    timeline: Timeline,
    compiled: CompiledTimeline,
    animation_nodes: BTreeMap<NodeId, NodeId>,
    animation_owners: BTreeSet<NodeId>,
    animation_warnings: Vec<String>,
    player: Player,
    recording: bool,
    generation: u64,
    collaboration: Option<SessionCollaboration>,
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
        let preview = RuntimePreview::new(preview_host, uri, &document)?;
        let mut result = Self {
            document,
            preview,
            selection: Vec::new(),
            timeline: Timeline::new(3.0).map_err(|error| error.to_string())?,
            compiled: Timeline::new(3.0)
                .and_then(|timeline| timeline.compile())
                .map_err(|error| error.to_string())?,
            animation_nodes: BTreeMap::new(),
            animation_owners: BTreeSet::new(),
            animation_warnings: Vec::new(),
            player: Player::new(),
            recording: false,
            generation: 0,
            collaboration: None,
            layout_tools: ferroui_designer_tools::LayoutRegistry::built_in(),
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
        self.preview = RuntimePreview::new(self.preview.host.clone(), uri, &self.document)?;
        self.player.stop();
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
        let preview = RuntimePreview::new(self.preview.host.clone(), uri, &candidate)?;
        self.document = candidate;
        self.preview = preview;
        self.selection.clear();
        self.collaboration = None;
        self.recording = false;
        self.player.stop();
        self.import_animations();
        self.generation += 1;
        Ok(())
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
        self.document.is_valid()
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
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }
    pub fn animation_warnings(&self) -> &[String] {
        &self.animation_warnings
    }
    pub fn time(&self) -> f64 {
        self.player.time()
    }
    pub fn is_playing(&self) -> bool {
        self.player.is_playing()
    }
    pub fn recording(&self) -> bool {
        self.recording
    }
    pub fn set_recording(&mut self, value: bool) {
        self.recording = value;
        self.generation += 1;
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
        let update = self.stage_source(source);
        let _ = self.apply_staged_preview();
        update
    }

    /// Accept source-buffer text immediately; a host can debounce only expensive runtime reload.
    pub fn stage_source(&mut self, source: impl Into<String>) -> SourceUpdate {
        if let Some(collaboration) = self.collaboration.as_mut() {
            if collaboration.draft_base.is_none() {
                collaboration.draft_base = Some(collaboration.replica.source().to_owned());
            }
        }
        let update = self.document.update_source("Edit XAML", source);
        self.player.pause();
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
        let result = self.publish_source_draft();
        self.changed(true);
        result
    }

    pub fn undo(&mut self) -> Result<ChangeSet, DesignerError> {
        if self.collaboration.is_some() {
            return self.collaborative_history(false);
        }
        let change = self.document.undo()?;
        self.changed(true);
        Ok(change)
    }
    pub fn redo(&mut self) -> Result<ChangeSet, DesignerError> {
        if self.collaboration.is_some() {
            return self.collaborative_history(true);
        }
        let change = self.document.redo()?;
        self.changed(true);
        Ok(change)
    }

    pub fn can_undo(&self) -> bool {
        self.collaboration.as_ref().map_or_else(
            || self.document.can_undo(),
            |state| state.draft_base.is_none() && !state.undo.is_empty(),
        )
    }
    pub fn can_redo(&self) -> bool {
        self.collaboration.as_ref().map_or_else(
            || self.document.can_redo(),
            |state| state.draft_base.is_none() && !state.redo.is_empty(),
        )
    }

    fn commit_document(
        &mut self,
        label: String,
        edits: Vec<Edit>,
    ) -> Result<ChangeSet, DesignerError> {
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
                let message=format!("Source collaboration conflict: {error}. Merge the source explicitly or discard the local draft.");
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

    fn changed(&mut self, import_animation: bool) {
        self.player.pause();
        self.selection
            .retain(|id| self.document.ast().node(*id).is_some());
        self.preview.reload(&self.document);
        if import_animation {
            self.import_animations();
        }
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
        if self.recording && !value.starts_with('{') {
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
        if !self.recording {
            self.apply(label, edits)
                .map_err(|error| error.to_string())?;
            return Ok(());
        }
        if !self.can_manipulate() {
            return Err(
                "Animation recording requires the current document to render successfully".into(),
            );
        }
        let mut candidate = self.timeline.clone();
        let mut ordinary = Vec::new();
        for edit in edits {
            if let Edit::SetAttribute { node, name, value } = &edit {
                if value.starts_with('{') {
                    ordinary.push(edit);
                    continue;
                }
                let value = animation_value(name, value)?;
                if candidate.track(*node, name).is_none() {
                    let baseline = self
                        .document
                        .ast()
                        .node(*node)
                        .and_then(|node| node.attribute(name))
                        .and_then(|text| animation_value(name, text).ok())
                        .or_else(|| {
                            self.preview.visual(*node).and_then(|visual| {
                                let property =
                                    ferroui_inspect::Inspector::property(&visual, name).ok()?;
                                let value = visual.get_value_untyped(property);
                                animation_value(name, &ferroui_inspect::display_value(Some(&value)))
                                    .ok()
                            })
                        })
                        .filter(|base| base.kind() == value.kind())
                        .unwrap_or_else(|| value.clone());
                    candidate
                        .record(*node, name, 0.0, baseline)
                        .map_err(|error| error.to_string())?;
                    candidate
                        .record(*node, name, candidate.duration, value.clone())
                        .map_err(|error| error.to_string())?;
                }
                candidate
                    .record(
                        *node,
                        name,
                        self.player.time().min(candidate.duration),
                        value,
                    )
                    .map_err(|error| error.to_string())?;
            } else {
                ordinary.push(edit);
            }
        }
        self.write_timeline_edits(candidate, label, ordinary)?;
        self.seek(self.player.time())
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
    }

    pub fn seek(&mut self, time: f64) -> Result<(), String> {
        if !time.is_finite() || time < 0.0 {
            return Err("Animation time must be finite and nonnegative".into());
        }
        self.player
            .seek(time.min(self.timeline.duration))
            .map_err(|error| error.to_string())?;
        let samples = self
            .compiled
            .scrub(self.player.time())
            .map_err(|error| error.to_string())?;
        self.preview.preview_samples(&samples)?;
        self.generation += 1;
        Ok(())
    }
    pub fn play(&mut self) {
        self.player.play();
        self.generation += 1;
    }
    pub fn set_interactive(&mut self, value: bool) {
        self.player.pause();
        self.preview.set_interactive(value);
        self.generation += 1;
    }
    pub fn pause(&mut self) {
        self.player.pause();
        self.generation += 1;
    }
    pub fn stop(&mut self) {
        self.player.stop();
        self.preview.clear_animation();
        self.generation += 1;
    }
    pub fn advance(&mut self, seconds: f64) -> Result<(), String> {
        if !self.player.is_playing() {
            return Ok(());
        }
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("Playback delta must be finite and nonnegative".into());
        }
        let mut time = self.player.time() + seconds;
        if !time.is_finite() {
            return Err("Playback time overflow".into());
        }
        if let Some(end) = self.timeline.total_duration() {
            if time >= end {
                time = end;
                self.player.pause();
            }
        }
        let samples = self
            .compiled
            .sample(time)
            .map_err(|error| error.to_string())?;
        self.player.seek(time).map_err(|error| error.to_string())?;
        self.preview.preview_samples(&samples)?;
        self.generation += 1;
        Ok(())
    }

    pub fn set_duration(&mut self, duration: f64) -> Result<(), String> {
        let mut candidate = self.timeline.clone();
        if !duration.is_finite() || duration <= 0.0 {
            return Err("Duration must be finite and positive".into());
        }
        let scale = duration / candidate.duration;
        for track in &mut candidate.tracks {
            for key in &mut track.keyframes {
                key.time *= scale;
            }
        }
        candidate.duration = duration;
        candidate.validate().map_err(|error| error.to_string())?;
        if candidate.tracks.is_empty() {
            self.compiled = candidate.compile().map_err(|error| error.to_string())?;
            self.timeline = candidate;
            self.generation += 1;
            return Ok(());
        }
        self.write_timeline(candidate, "Change animation duration")
    }

    pub fn record_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        value: AnimationValue,
    ) -> Result<(), String> {
        let mut candidate = self.timeline.clone();
        if candidate.track(target, property).is_none() {
            candidate
                .record(target, property, 0.0, value.clone())
                .map_err(|error| error.to_string())?;
            candidate
                .record(target, property, candidate.duration, value.clone())
                .map_err(|error| error.to_string())?;
        }
        candidate
            .record(
                target,
                property,
                self.player.time().min(candidate.duration),
                value,
            )
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Add animation keyframe")
    }
    pub fn move_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        from: f64,
        to: f64,
    ) -> Result<(), String> {
        let mut candidate = self.timeline.clone();
        candidate
            .track_mut(target, property)
            .ok_or("No animation track")?
            .move_keyframe(from, to)
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Move animation keyframe")
    }
    pub fn remove_keyframe(
        &mut self,
        target: NodeId,
        property: &str,
        time: f64,
    ) -> Result<(), String> {
        let mut candidate = self.timeline.clone();
        candidate
            .track_mut(target, property)
            .ok_or("No animation track")?
            .remove_keyframe(time)
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Remove animation keyframe")
    }
    pub fn set_keyframe_easing(
        &mut self,
        target: NodeId,
        property: &str,
        time: f64,
        easing: ferroui_designer_animation::Easing,
    ) -> Result<(), String> {
        let mut candidate = self.timeline.clone();
        candidate
            .track_mut(target, property)
            .ok_or("No animation track")?
            .set_easing(time, easing)
            .map_err(|error| error.to_string())?;
        self.write_timeline(candidate, "Change keyframe easing")
    }
    pub fn commit_animation(&mut self) -> Result<(), String> {
        self.write_timeline(self.timeline.clone(), "Write animation to XAML")
    }

    fn write_timeline(&mut self, candidate: Timeline, label: &str) -> Result<(), String> {
        self.write_timeline_edits(candidate, label, Vec::new())
    }
    fn write_timeline_edits(
        &mut self,
        candidate: Timeline,
        label: &str,
        ordinary: Vec<Edit>,
    ) -> Result<(), String> {
        candidate.validate().map_err(|error| error.to_string())?;
        let targets = candidate
            .tracks
            .iter()
            .map(|track| track.target)
            .collect::<BTreeSet<_>>();
        if targets.is_empty() {
            return Err("Add an animation keyframe first".into());
        }
        let mut patches = Vec::new();
        let mut inserts = Vec::new();
        let mut names = Vec::new();
        for target in targets {
            let owner = self
                .document
                .ast()
                .node(target)
                .ok_or("Animation target was removed")?;
            if let Some(animation) = self
                .animation_nodes
                .get(&target)
                .and_then(|id| self.document.ast().node(*id))
            {
                let generated = candidate
                    .to_xaml(target)
                    .map_err(|error| error.to_string())?;
                let (fragment, declaration) =
                    crate::preview_source::embed_native_fragment(&self.document, &generated)?;
                if let Some(declaration) = declaration {
                    names.push(declaration);
                }
                patches.push((animation.span, fragment));
            } else {
                if self.animation_owners.contains(&target) {
                    return Err(format!("Element {target} has an animation outside the active timeline. Edit that animation in source before recording another animation on the same control."));
                }
                let name = owner.xaml_name().map(str::to_owned).unwrap_or_else(|| {
                    let names = self
                        .document
                        .ast()
                        .elements()
                        .filter_map(|node| node.xaml_name())
                        .collect::<BTreeSet<_>>();
                    let mut name = format!("DesignerNode{target}");
                    let mut suffix = 1;
                    while names.contains(name.as_str()) {
                        name = format!("DesignerNode{target}_{suffix}");
                        suffix += 1;
                    }
                    name
                });
                if owner.xaml_name().is_none() {
                    names.push(Edit::SetAttribute {
                        node: target,
                        name: "Name".into(),
                        value: name.clone(),
                    });
                }
                let selector = format!("{}#{name}", owner.name.replace(':', "|"));
                let generated = candidate
                    .to_style_xaml(target, &selector)
                    .map_err(|error| error.to_string())?;
                let (style, declaration) =
                    crate::preview_source::embed_native_fragment(&self.document, &generated)?;
                if let Some(declaration) = declaration {
                    names.push(declaration);
                }
                let styles = owner
                    .children
                    .iter()
                    .filter_map(|id| self.document.ast().node(*id))
                    .find(|node| node.name.ends_with(".Styles"));
                if let Some(styles) = styles {
                    inserts.push(Edit::InsertChild {
                        parent: styles.id,
                        index: self.document.ast().element_children(styles.id).len(),
                        source: style,
                    });
                } else {
                    inserts.push(Edit::InsertChild {
                        parent: target,
                        index: 0,
                        source: format!(
                            "<{}.Styles>\n{}\n</{}.Styles>",
                            owner.name, style, owner.name
                        ),
                    });
                }
            }
        }
        // Text patches refer to the same source snapshot, so apply in descending order.
        patches.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
        let mut edits = patches
            .into_iter()
            .map(|(span, replacement)| Edit::Text { span, replacement })
            .collect::<Vec<_>>();
        edits.extend(names);
        edits.extend(inserts);
        edits.extend(ordinary);
        self.commit_document(label.into(), edits)
            .map_err(|error| error.to_string())?;
        self.timeline = candidate;
        self.changed(true);
        Ok(())
    }

    fn locate_animation_nodes(&mut self) {
        self.animation_nodes.clear();
        self.animation_owners.clear();
        let mut animations = self
            .document
            .ast()
            .elements()
            .filter(|node| node.local_name() == "Animation")
            .collect::<Vec<_>>();
        animations.sort_by_key(|node| node.span.start);
        for animation in animations {
            if let Some(styles) = animation
                .parent
                .and_then(|id| self.document.ast().node(id))
                .and_then(|node| node.parent)
                .and_then(|id| self.document.ast().node(id))
                .and_then(|node| node.parent)
                .and_then(|id| self.document.ast().node(id))
                .filter(|node| node.name.ends_with(".Styles"))
            {
                if let Some(owner) = styles.parent {
                    self.animation_owners.insert(owner);
                }
            }
            let target = (|| {
                let animations = animation
                    .parent
                    .and_then(|id| self.document.ast().node(id))
                    .ok_or("animation has no owning collection")?;
                if !animations.name.ends_with(".Animations") {
                    return Err("animation is outside a local Style.Animations collection");
                }
                let style = animations
                    .parent
                    .and_then(|id| self.document.ast().node(id))
                    .ok_or("animation has no style")?;
                let styles = style
                    .parent
                    .and_then(|id| self.document.ast().node(id))
                    .ok_or("style is not local to a control")?;
                if !styles.name.ends_with(".Styles") {
                    return Err("style is not local to a control");
                }
                let owner = styles
                    .parent
                    .and_then(|id| self.document.ast().node(id))
                    .ok_or("style has no owning control")?;
                let name = owner
                    .xaml_name()
                    .ok_or("the owning control needs a Name for the active timeline")?;
                let selector = format!("{}#{name}", owner.name.replace(':', "|"));
                if style.attribute("Selector") != Some(selector.as_str())
                    && style.attribute("Selector") != Some(format!("#{name}").as_str())
                {
                    return Err("the selector is not an exact match for its named owning control");
                }
                Ok(owner.id)
            })();
            match target {
                Ok(owner) if !self.animation_nodes.contains_key(&owner)=>{self.animation_nodes.insert(owner,animation.id);},
                Ok(owner)=>self.animation_warnings.push(format!("Animation {} on element {owner} is preserved in source; the active timeline already represents another animation on this control",animation.id)),
                Err(reason)=>self.animation_warnings.push(format!("Animation {} is preserved in source: {reason}",animation.id)),
            }
        }
    }

    fn import_animations(&mut self) {
        if !self.document.diagnostics().is_empty() {
            return;
        }
        self.animation_warnings.clear();
        self.locate_animation_nodes();
        let mut imported: Option<Timeline> = None;
        let mut accepted = BTreeMap::new();
        for (target, id) in &self.animation_nodes {
            let node = self.document.ast().node(*id).expect("located node");
            let source = animation_fragment(&self.document, node.id);
            let mut options = ferroui_designer_animation::ImportOptions::default();
            for setter in self.document.ast().elements().filter(|setter| {
                setter.local_name() == "Setter"
                    && setter.span.start >= node.span.start
                    && setter.span.end <= node.span.end
            }) {
                if let Some(property) = setter.attribute("Property") {
                    let base = self
                        .document
                        .ast()
                        .node(*target)
                        .and_then(|owner| owner.attribute(property))
                        .filter(|value| !value.starts_with('{'))
                        .and_then(|value| animation_value(property, value).ok())
                        .or_else(|| {
                            self.preview.visual(*target).and_then(|visual| {
                                let property_info =
                                    ferroui_inspect::Inspector::property(&visual, property).ok()?;
                                animation_value(
                                    property,
                                    &ferroui_inspect::display_value(Some(
                                        &visual.get_value_untyped(property_info),
                                    )),
                                )
                                .ok()
                            })
                        });
                    let kind = setter
                        .attribute("Value")
                        .filter(|value| !value.starts_with('{'))
                        .and_then(|value| animation_value(property, value).ok())
                        .map(|value| value.kind());
                    if let Some(kind) = kind {
                        options.property_kinds.insert(property.into(), kind);
                    }
                    if let Some(base) =
                        base.filter(|base| kind.is_none_or(|kind| kind == base.kind()))
                    {
                        options.base_values.insert(property.into(), base);
                    }
                }
            }
            match Timeline::import_xaml(&source, *target, &options) {
                Ok(import) => {
                    self.animation_warnings.extend(
                        import
                            .warnings
                            .into_iter()
                            .map(|warning| format!("Element {target}: {warning}")),
                    );
                    let mut timeline = import.timeline;
                    if let Some(existing) = imported.as_mut() {
                        let mut a = existing.clone();
                        a.tracks.clear();
                        let mut b = timeline.clone();
                        b.tracks.clear();
                        if a == b {
                            existing.tracks.append(&mut timeline.tracks);
                            accepted.insert(*target, *id);
                        } else {
                            self.animation_warnings.push(format!("Element {target} has independent timing; its source is retained and is not added to the active timeline"));
                        }
                    } else {
                        accepted.insert(*target, *id);
                        imported = Some(timeline);
                    }
                }
                Err(error) => self
                    .animation_warnings
                    .push(format!("Element {target}: {error}")),
            }
        }
        self.animation_nodes = accepted;
        self.timeline = imported.unwrap_or_else(|| {
            Timeline::new(self.timeline.duration).expect("valid retained duration")
        });
        self.compiled = self
            .timeline
            .compile()
            .expect("validated imported timeline");
    }
}

pub(crate) fn animation_value(property: &str, value: &str) -> Result<AnimationValue, String> {
    let kind = if matches!(property, "Margin" | "Padding" | "BorderThickness") {
        ValueKind::Thickness
    } else if value.starts_with('#') {
        ValueKind::Color
    } else if value.trim().parse::<f64>().is_ok() {
        ValueKind::Number
    } else if matches!(
        property,
        "RenderTransformOrigin" | "Center" | "StartPoint" | "EndPoint"
    ) {
        ValueKind::Point
    } else {
        ValueKind::Discrete
    };
    AnimationValue::parse(value, kind).map_err(|error| error.to_string())
}

// A standalone import fragment must retain prefixes declared by its ancestors.
fn animation_fragment(document: &Document, id: NodeId) -> String {
    let node = document.ast().node(id).expect("animation node");
    let mut source = document.valid_source()[node.span.start..node.span.end].to_owned();
    let mut namespaces = BTreeMap::new();
    let mut current = Some(id);
    while let Some(id) = current {
        let node = document.ast().node(id).expect("ancestor node");
        for attribute in node
            .attributes
            .iter()
            .filter(|attribute| attribute.is_namespace())
        {
            namespaces
                .entry(attribute.name.clone())
                .or_insert_with(|| attribute.value.clone());
        }
        current = node.parent;
    }
    let mut declarations = String::new();
    for (name, value) in namespaces {
        if node.attribute(&name).is_some() {
            continue;
        }
        declarations.push_str(&format!(
            " {name}=\"{}\"",
            value
                .replace('&', "&amp;")
                .replace('\"', "&quot;")
                .replace('<', "&lt;")
        ));
    }
    source.insert_str(node.name_span.end - node.span.start, &declarations);
    source
}
