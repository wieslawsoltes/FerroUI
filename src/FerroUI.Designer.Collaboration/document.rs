use crate::model::{ensure, identifier, validate_operation, PresenceEntry};
use crate::{
    CollaborationError, Conflict, MergeReport, Operation, OperationId, Presence, Result,
    SharedEdit, SharedNodeId, Snapshot, SyncMessage, VersionVector, MAX_OPERATIONS,
    PROTOCOL_VERSION,
};
use ferroui_designer_core::{Document, Edit, NodeId};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
struct WriterState {
    properties: BTreeMap<(SharedNodeId, String), OperationId>,
    source: Option<OperationId>,
}
impl WriterState {
    fn last(&self, node: &SharedNodeId, name: &str) -> Option<&OperationId> {
        self.properties
            .get(&(node.clone(), name.to_owned()))
            .max(self.source.as_ref())
    }
    fn record(&mut self, edit: &SharedEdit, id: &OperationId) {
        match edit {
            SharedEdit::SetAttribute { node, name, .. }
            | SharedEdit::RemoveAttribute { node, name }
            | SharedEdit::UndoAttribute { node, name, .. } => {
                self.properties
                    .insert((node.clone(), name.clone()), id.clone());
            }
            SharedEdit::ReplaceSource { .. } => self.source = Some(id.clone()),
            _ => {}
        }
    }
}

/// Ordered operation-set replication of a lossless XAML document. Property writes are last-writer
/// wins in Lamport order; structural conflicts are explicit and resolved in that same order.
/// A transaction arriving before its inserted-node dependency is retained and retried when the
/// dependency arrives. No server order or wall-clock synchronization is required.
///
/// This is not a character-level text CRDT: arbitrary source replacements use exact-source CAS.
/// Replay from the baseline provides deterministic structure even for concurrent cyclic moves.
#[derive(Clone)]
pub struct DocumentReplica {
    actor: String,
    document_id: String,
    baseline: String,
    baseline_hash: String,
    operations: BTreeMap<OperationId, Operation>,
    identities: BTreeMap<(String, u64), OperationId>,
    clock: u64,
    sequence: u64,
    document: Document,
    aliases: BTreeMap<SharedNodeId, NodeId>,
    writers: WriterState,
    conflicts: Vec<Conflict>,
    applied: usize,
    presence: BTreeMap<String, PresenceEntry>,
}
impl DocumentReplica {
    pub fn new(
        actor: impl Into<String>,
        document_id: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<Self> {
        let actor = actor.into();
        let document_id = document_id.into();
        let baseline = source.into();
        identifier(&actor)?;
        identifier(&document_id)?;
        ensure(
            baseline.len() <= 16 * 1024 * 1024,
            "Baseline exceeds the document size limit",
        )?;
        let mut document = Document::new(baseline.clone()).map_err(error)?;
        document.set_history_limit(0);
        let aliases = document
            .ast()
            .elements()
            .map(|n| (SharedNodeId::Baseline(n.id), n.id))
            .collect();
        let baseline_hash = fingerprint(&baseline);
        Ok(Self {
            actor,
            document_id,
            baseline,
            baseline_hash,
            operations: BTreeMap::new(),
            identities: BTreeMap::new(),
            clock: 0,
            sequence: 0,
            document,
            aliases,
            writers: WriterState::default(),
            conflicts: vec![],
            applied: 0,
            presence: BTreeMap::new(),
        })
    }
    pub fn actor(&self) -> &str {
        &self.actor
    }
    pub fn document_id(&self) -> &str {
        &self.document_id
    }
    pub fn baseline_hash(&self) -> &str {
        &self.baseline_hash
    }
    pub fn message(&self, operations: Vec<Operation>) -> SyncMessage {
        SyncMessage {
            protocol: PROTOCOL_VERSION,
            document_id: self.document_id.clone(),
            baseline_hash: self.baseline_hash.clone(),
            operations,
        }
    }
    pub fn document(&self) -> &Document {
        &self.document
    }
    pub fn source(&self) -> &str {
        self.document.source()
    }
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }
    pub fn operations(&self) -> impl Iterator<Item = &Operation> {
        self.operations.values()
    }
    pub fn resolve(&self, node: &SharedNodeId) -> Option<NodeId> {
        self.aliases
            .get(node)
            .copied()
            .filter(|id| self.document.ast().node(*id).is_some())
    }
    pub fn shared_id(&self, node: NodeId) -> Option<SharedNodeId> {
        self.aliases.iter().find_map(|(shared, id)| {
            (*id == node && self.document.ast().node(node).is_some()).then(|| shared.clone())
        })
    }
    pub fn root(&self) -> SharedNodeId {
        self.shared_id(self.document.ast().root())
            .expect("root has a shared identity")
    }
    pub fn version(&self) -> VersionVector {
        let mut sequences: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
        for id in self.operations.keys() {
            sequences.entry(&id.actor).or_default().insert(id.sequence);
        }
        sequences
            .into_iter()
            .map(|(actor, sequences)| {
                let mut contiguous = 0;
                while contiguous < u64::MAX && sequences.contains(&(contiguous + 1)) {
                    contiguous += 1;
                }
                (actor.to_owned(), contiguous)
            })
            .collect()
    }
    pub fn changes_since(&self, version: &VersionVector) -> SyncMessage {
        SyncMessage {
            protocol: PROTOCOL_VERSION,
            document_id: self.document_id.clone(),
            baseline_hash: self.baseline_hash.clone(),
            operations: self
                .operations
                .values()
                .filter(|op| op.id.sequence > *version.get(&op.id.actor).unwrap_or(&0))
                .cloned()
                .collect(),
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            protocol: PROTOCOL_VERSION,
            document_id: self.document_id.clone(),
            baseline: self.baseline.clone(),
            operations: self.operations.values().cloned().collect(),
        }
    }
    pub fn from_snapshot(actor: impl Into<String>, snapshot: Snapshot) -> Result<Self> {
        ensure(
            snapshot.protocol == PROTOCOL_VERSION,
            "Unsupported collaboration protocol",
        )?;
        let mut replica = Self::new(actor, snapshot.document_id.clone(), snapshot.baseline)?;
        replica.merge(SyncMessage {
            protocol: snapshot.protocol,
            document_id: snapshot.document_id,
            baseline_hash: replica.baseline_hash.clone(),
            operations: snapshot.operations,
        })?;
        Ok(replica)
    }
    pub fn merge_snapshot(&mut self, snapshot: Snapshot) -> Result<MergeReport> {
        ensure(snapshot.baseline==self.baseline,"Snapshot baseline differs; begin a new collaboration session or resolve the source difference explicitly")?;
        self.merge(SyncMessage {
            protocol: snapshot.protocol,
            document_id: snapshot.document_id,
            baseline_hash: self.baseline_hash.clone(),
            operations: snapshot.operations,
        })
    }
    /// Submit an atomic local transaction, rejecting edits that cannot currently be applied.
    /// Hosts broadcast the returned operation; the new source is available via source().
    pub fn submit(
        &mut self,
        label: impl Into<String>,
        edits: Vec<SharedEdit>,
    ) -> Result<Operation> {
        let operation = Operation {
            id: OperationId {
                lamport: self
                    .clock
                    .checked_add(1)
                    .ok_or_else(|| CollaborationError("Lamport counter exhausted".into()))?,
                actor: self.actor.clone(),
                sequence: self
                    .sequence
                    .checked_add(1)
                    .ok_or_else(|| CollaborationError("Actor sequence exhausted".into()))?,
            },
            document_id: self.document_id.clone(),
            label: label.into(),
            edits,
        };
        validate_operation(&operation)?;
        // Validate locally before publishing a durable operation. Remote operations may arrive
        // before dependencies, so merge retains those as visible conflicts for later replay.
        let mut document = self.document.clone();
        let mut aliases = self.aliases.clone();
        let mut writers = self.writers.clone();
        apply_operation(&mut document, &mut aliases, &mut writers, &operation)?;
        self.merge(SyncMessage {
            protocol: PROTOCOL_VERSION,
            document_id: self.document_id.clone(),
            baseline_hash: self.baseline_hash.clone(),
            operations: vec![operation.clone()],
        })?;
        Ok(operation)
    }
    pub fn set_attribute(
        &mut self,
        node: SharedNodeId,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Operation> {
        self.submit(
            "Set property",
            vec![SharedEdit::SetAttribute {
                node,
                name: name.into(),
                value: value.into(),
            }],
        )
    }
    /// Resolve a node from an independently hosted document with byte-identical source. The
    /// host's local AST IDs may differ after undo or preview history; spans provide the bridge.
    pub fn shared_id_in(&self, document: &Document, node: NodeId) -> Result<SharedNodeId> {
        ensure(
            document.is_valid() && document.source() == self.source(),
            "Host document differs from the collaboration projection; synchronize it first",
        )?;
        let node = document
            .ast()
            .node(node)
            .ok_or_else(|| CollaborationError("Unknown host document node".into()))?;
        let local = self
            .document
            .ast()
            .elements()
            .find(|candidate| candidate.span == node.span && candidate.name == node.name)
            .ok_or_else(|| {
                CollaborationError("Host node has no matching collaboration node".into())
            })?;
        self.shared_id(local.id)
            .ok_or_else(|| CollaborationError("Node lacks a shared identity".into()))
    }
    /// Convert existing designer commands to shared operations. Text-coordinate or whole-source
    /// edits intentionally become one source CAS operation; structured property/layout tools keep
    /// their fine-grained conflict semantics. Apply replica.source() to the host after success.
    pub fn submit_document_edits(
        &mut self,
        document: &Document,
        label: impl Into<String>,
        edits: Vec<Edit>,
    ) -> Result<Operation> {
        ensure(
            document.is_valid() && document.source() == self.source(),
            "Host document differs from the collaboration projection; synchronize it first",
        )?;
        let label = label.into();
        if edits
            .iter()
            .any(|edit| matches!(edit, Edit::Text { .. } | Edit::ReplaceSource { .. }))
        {
            let mut candidate = document.clone();
            candidate.apply(label.clone(), edits).map_err(error)?;
            return self.submit(
                label,
                vec![SharedEdit::ReplaceSource {
                    expected_source: self.source().into(),
                    source: candidate.source().into(),
                }],
            );
        }
        let mut shared = Vec::with_capacity(edits.len());
        for edit in edits {
            shared.push(match edit {
                Edit::SetAttribute { node, name, value } => SharedEdit::SetAttribute {
                    node: self.shared_id_in(document, node)?,
                    name,
                    value,
                },
                Edit::RemoveAttribute { node, name } => SharedEdit::RemoveAttribute {
                    node: self.shared_id_in(document, node)?,
                    name,
                },
                Edit::InsertChild {
                    parent,
                    index,
                    source,
                } => SharedEdit::InsertChild {
                    parent: self.shared_id_in(document, parent)?,
                    index,
                    source,
                },
                Edit::RemoveNode { node } => SharedEdit::RemoveNode {
                    node: self.shared_id_in(document, node)?,
                },
                Edit::MoveNode {
                    node,
                    parent,
                    index,
                } => SharedEdit::MoveNode {
                    node: self.shared_id_in(document, node)?,
                    parent: self.shared_id_in(document, parent)?,
                    index,
                },
                Edit::Text { .. } | Edit::ReplaceSource { .. } => {
                    unreachable!("source edits handled above")
                }
            });
        }
        self.submit(label, shared)
    }
    /// Undo is a new causally later write. Conditional compensation refuses to overwrite another
    /// actor's change since the original operation. Structural undo can be supplied explicitly
    /// through submit(); no hidden mutation of the replicated operation log is allowed.
    pub fn undo_property(&mut self, operation: &OperationId) -> Result<Operation> {
        let original = self
            .operations
            .get(operation)
            .ok_or_else(|| CollaborationError("Unknown operation".into()))?;
        ensure(
            original.id.actor == self.actor,
            "Only local property edits can be undone with this helper",
        )?;
        ensure(
            original.edits.len() == 1,
            "Undo helper requires one property edit",
        )?;
        let (node, name) = match &original.edits[0] {
            SharedEdit::SetAttribute { node, name, .. }
            | SharedEdit::RemoveAttribute { node, name }
            | SharedEdit::UndoAttribute { node, name, .. } => (node.clone(), name.clone()),
            _ => {
                return Err(CollaborationError(
                    "Use an explicit compensating transaction for structural edits".into(),
                ))
            }
        };
        // Replay the prefix to recover the original prior value without retaining history copies.
        let (mut before, mut aliases) = self.fresh_document()?;
        let mut prefix_writers = WriterState::default();
        for (id, operation) in &self.operations {
            if id >= &original.id {
                break;
            }
            let _ = apply_operation(&mut before, &mut aliases, &mut prefix_writers, operation);
        }
        let prior = aliases
            .get(&node)
            .and_then(|id| before.ast().node(*id))
            .and_then(|n| n.attribute(&name))
            .map(str::to_owned);
        let expected = match &original.edits[0] {
            SharedEdit::SetAttribute { value, .. } => Some(value.as_str()),
            SharedEdit::UndoAttribute { value, .. } => value.as_deref(),
            _ => None,
        };
        ensure(
            self.writers.last(&node, &name) == Some(&original.id),
            "Another applied operation owns this property; resolve the conflict before undo",
        )?;
        let current = self
            .resolve(&node)
            .and_then(|id| self.document.ast().node(id))
            .and_then(|n| n.attribute(&name));
        ensure(
            current == expected,
            "Property changed since this operation; resolve the conflict before undo",
        )?;
        let edit = SharedEdit::UndoAttribute {
            node,
            name,
            expected_writer: original.id.clone(),
            value: prior,
        };
        self.submit("Undo property edit", vec![edit])
    }
    /// Validates every envelope entry before changing the replica. Duplicate identity with
    /// different bytes is rejected as equivocation, never resolved by delivery order.
    pub fn merge(&mut self, message: SyncMessage) -> Result<MergeReport> {
        ensure(
            message.protocol == PROTOCOL_VERSION,
            "Unsupported collaboration protocol",
        )?;
        ensure(
            message.document_id == self.document_id,
            "Synchronization document does not match",
        )?;
        ensure(
            message.baseline_hash == self.baseline_hash,
            "Synchronization baseline differs; exchange a snapshot or start a new session",
        )?;
        ensure(
            message.operations.len() <= MAX_OPERATIONS,
            "Too many operations in a message",
        )?;
        message.to_json()?;
        let mut pending: BTreeMap<OperationId, Operation> = BTreeMap::new();
        let mut pending_ids: BTreeMap<(String, u64), OperationId> = BTreeMap::new();
        for operation in message.operations {
            validate_operation(&operation)?;
            ensure(
                operation.document_id == self.document_id,
                "Operation targets another document",
            )?;
            let actor_key = (operation.id.actor.clone(), operation.id.sequence);
            if let Some(existing) = self
                .identities
                .get(&actor_key)
                .or_else(|| pending_ids.get(&actor_key))
            {
                ensure(
                    existing == &operation.id,
                    "Actor reused a sequence with a different timestamp",
                )?;
            }
            if let Some(existing) = self
                .operations
                .get(&operation.id)
                .or_else(|| pending.get(&operation.id))
            {
                ensure(
                    existing == &operation,
                    "Operation identity has conflicting content",
                )?;
                continue;
            }
            pending_ids.insert(actor_key, operation.id.clone());
            pending.insert(operation.id.clone(), operation);
        }
        ensure(
            self.operations.len().saturating_add(pending.len()) <= MAX_OPERATIONS,
            "Session operation limit reached; checkpoint and start a new session",
        )?;
        let actors = pending
            .keys()
            .map(|id| id.actor.clone())
            .collect::<BTreeSet<_>>();
        for actor in actors {
            let mut actor_ids = self
                .identities
                .range((actor.clone(), 0)..=(actor.clone(), u64::MAX))
                .map(|((_, sequence), id)| (*sequence, id.lamport))
                .collect::<BTreeMap<_, _>>();
            actor_ids.extend(
                pending
                    .keys()
                    .filter(|id| id.actor == actor)
                    .map(|id| (id.sequence, id.lamport)),
            );
            let mut previous = 0;
            for lamport in actor_ids.values() {
                ensure(
                    *lamport > previous,
                    "Actor timestamps do not respect sequence order",
                )?;
                previous = *lamport;
            }
        }

        let received = pending.len();
        if received == 0 {
            return Ok(MergeReport {
                received,
                applied: self.applied,
                conflicts: self.conflicts.clone(),
            });
        }
        // Rebuild only when a newly received operation precedes an existing one. Ordered arrival
        // (the common local editing case) applies only the appended suffix.
        let append_only = self.operations.last_key_value().is_none_or(|(last, _)| {
            pending
                .first_key_value()
                .is_some_and(|(first, _)| first > last)
        });
        let (mut document, mut aliases, mut writers, mut conflicts, mut applied) = if append_only {
            (
                self.document.clone(),
                self.aliases.clone(),
                self.writers.clone(),
                self.conflicts.clone(),
                self.applied,
            )
        } else {
            let (document, aliases) = self.fresh_document()?;
            (document, aliases, WriterState::default(), vec![], 0)
        };
        let all = if append_only {
            BTreeMap::new()
        } else {
            let mut all = self.operations.clone();
            all.extend(pending.clone());
            all
        };
        let to_apply = if append_only { &pending } else { &all };
        for operation in to_apply.values() {
            match apply_operation(&mut document, &mut aliases, &mut writers, operation) {
                Ok(()) => applied += 1,
                Err(err) => conflicts.push(Conflict {
                    operation: operation.id.clone(),
                    reason: err.0,
                }),
            }
        }
        let clock = pending
            .keys()
            .map(|id| id.lamport)
            .max()
            .unwrap_or(0)
            .max(self.clock);
        let sequence = pending
            .keys()
            .filter(|id| id.actor == self.actor)
            .map(|id| id.sequence)
            .max()
            .unwrap_or(0)
            .max(self.sequence);
        self.operations.extend(pending);
        self.identities.extend(pending_ids);
        self.clock = clock;
        self.sequence = sequence;
        self.document = document;
        self.aliases = aliases;
        self.writers = writers;
        self.conflicts = conflicts;
        self.applied = applied;
        Ok(MergeReport {
            received,
            applied,
            conflicts: self.conflicts.clone(),
        })
    }
    pub fn receive_presence(&mut self, presence: Presence, now_millis: u64) -> Result<bool> {
        identifier(&presence.actor)?;
        ensure(
            presence.sequence > 0
                && presence.display_name.len() <= 256
                && presence.selection.len() <= 1024,
            "Invalid presence update",
        )?;
        if let Some(cursor) = &presence.cursor {
            ensure(
                cursor.x.is_finite() && cursor.y.is_finite(),
                "Presence coordinates must be finite",
            )?;
        }
        if self
            .presence
            .get(&presence.actor)
            .is_some_and(|prior| prior.presence.sequence >= presence.sequence)
        {
            return Ok(false);
        }
        self.presence.insert(
            presence.actor.clone(),
            PresenceEntry {
                presence,
                observed_at: now_millis,
            },
        );
        Ok(true)
    }
    /// The receiving host's monotonic clock governs expiry, avoiding remote-clock assumptions.
    pub fn presence(&self, now_millis: u64, ttl_millis: u64) -> Vec<&Presence> {
        self.presence
            .values()
            .filter(|entry| now_millis.saturating_sub(entry.observed_at) < ttl_millis)
            .map(|entry| &entry.presence)
            .collect()
    }
    pub fn prune_presence(&mut self, now_millis: u64, ttl_millis: u64) {
        self.presence
            .retain(|_, entry| now_millis.saturating_sub(entry.observed_at) < ttl_millis);
    }
    fn fresh_document(&self) -> Result<(Document, BTreeMap<SharedNodeId, NodeId>)> {
        let mut document = Document::new(self.baseline.clone()).map_err(error)?;
        document.set_history_limit(0);
        let aliases = document
            .ast()
            .elements()
            .map(|n| (SharedNodeId::Baseline(n.id), n.id))
            .collect();
        Ok((document, aliases))
    }
}
fn apply_operation(
    document: &mut Document,
    aliases: &mut BTreeMap<SharedNodeId, NodeId>,
    writers: &mut WriterState,
    operation: &Operation,
) -> Result<()> {
    let mut candidate = document.clone();
    let mut mapped = aliases.clone();
    let mut candidate_writers = writers.clone();
    // Namespace/name changes form a single syntax boundary: an intermediate prefix can be
    // unbound even when the complete attribute transaction is valid.
    if operation.edits.iter().all(|edit| {
        matches!(
            edit,
            SharedEdit::SetAttribute { .. }
                | SharedEdit::RemoveAttribute { .. }
                | SharedEdit::UndoAttribute { .. }
        )
    }) {
        let mut edits = Vec::with_capacity(operation.edits.len());
        for edit in &operation.edits {
            let resolve = |node: &SharedNodeId| {
                mapped
                    .get(node)
                    .copied()
                    .filter(|id| candidate.ast().node(*id).is_some())
                    .ok_or_else(|| CollaborationError(format!("Missing or deleted node {node:?}")))
            };
            edits.push(match edit {
                SharedEdit::SetAttribute { node, name, value } => Edit::SetAttribute {
                    node: resolve(node)?,
                    name: name.clone(),
                    value: value.clone(),
                },
                SharedEdit::RemoveAttribute { node, name } => Edit::RemoveAttribute {
                    node: resolve(node)?,
                    name: name.clone(),
                },
                SharedEdit::UndoAttribute {
                    node,
                    name,
                    expected_writer,
                    value,
                } => {
                    ensure(
                        candidate_writers.last(node, name) == Some(expected_writer),
                        "Concurrent property writer: conditional undo was not applied",
                    )?;
                    if let Some(value) = value {
                        Edit::SetAttribute {
                            node: resolve(node)?,
                            name: name.clone(),
                            value: value.clone(),
                        }
                    } else {
                        Edit::RemoveAttribute {
                            node: resolve(node)?,
                            name: name.clone(),
                        }
                    }
                }
                _ => unreachable!("attribute-only transaction"),
            });
        }
        let change = candidate
            .apply(operation.label.clone(), edits)
            .map_err(error)?;
        register_inserted(&candidate, &mut mapped, operation, 0, &change.inserted);
        for edit in &operation.edits {
            candidate_writers.record(edit, &operation.id);
        }
        *document = candidate;
        *aliases = mapped;
        *writers = candidate_writers;
        return Ok(());
    }
    for (index, edit) in operation.edits.iter().enumerate() {
        let resolve = |node: &SharedNodeId| {
            mapped.get(node).copied().filter(|id|candidate.ast().node(*id).is_some()).ok_or_else(||CollaborationError(format!("Missing or deleted node {node:?}; the operation is retained for deterministic replay")))
        };
        let local = match edit {
            SharedEdit::SetAttribute { node, name, value } => Edit::SetAttribute {
                node: resolve(node)?,
                name: name.clone(),
                value: value.clone(),
            },
            SharedEdit::RemoveAttribute { node, name } => Edit::RemoveAttribute {
                node: resolve(node)?,
                name: name.clone(),
            },
            SharedEdit::UndoAttribute {
                node,
                name,
                expected_writer,
                value,
            } => {
                ensure(
                    candidate_writers.last(node, name) == Some(expected_writer),
                    "Concurrent property writer: conditional undo was not applied",
                )?;
                if let Some(value) = value {
                    Edit::SetAttribute {
                        node: resolve(node)?,
                        name: name.clone(),
                        value: value.clone(),
                    }
                } else {
                    Edit::RemoveAttribute {
                        node: resolve(node)?,
                        name: name.clone(),
                    }
                }
            }
            SharedEdit::InsertChild {
                parent,
                index,
                source,
            } => Edit::InsertChild {
                parent: resolve(parent)?,
                index: *index,
                source: source.clone(),
            },
            SharedEdit::RemoveNode { node } => Edit::RemoveNode {
                node: resolve(node)?,
            },
            SharedEdit::MoveNode {
                node,
                parent,
                index,
            } => Edit::MoveNode {
                node: resolve(node)?,
                parent: resolve(parent)?,
                index: *index,
            },
            SharedEdit::ReplaceSource {
                expected_source,
                source,
            } => {
                ensure(
                    candidate.source() == expected_source,
                    "Concurrent source change: source replacement precondition does not match",
                )?;
                Edit::ReplaceSource {
                    source: source.clone(),
                }
            }
        };
        let change = candidate
            .apply(operation.label.clone(), vec![local])
            .map_err(error)?;
        register_inserted(&candidate, &mut mapped, operation, index, &change.inserted);
        candidate_writers.record(edit, &operation.id);
    }
    *document = candidate;
    *aliases = mapped;
    *writers = candidate_writers;
    Ok(())
}
fn register_inserted(
    document: &Document,
    aliases: &mut BTreeMap<SharedNodeId, NodeId>,
    operation: &Operation,
    edit: usize,
    ids: &[NodeId],
) {
    let mut inserted: Vec<_> = ids
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| node.is_element())
        .collect();
    inserted.sort_by_key(|node| node.span.start);
    for (ordinal, node) in inserted.into_iter().enumerate() {
        aliases.insert(
            SharedNodeId::Inserted {
                actor: operation.id.actor.clone(),
                sequence: operation.id.sequence,
                edit,
                ordinal,
            },
            node.id,
        );
    }
}
fn error(error: impl std::fmt::Display) -> CollaborationError {
    CollaborationError(error.to_string())
}

fn fingerprint(source: &str) -> String {
    Sha256::digest(source.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
