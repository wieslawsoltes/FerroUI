use crate::model::{
    ensure, identifier, serialized_size, validate_node, validate_operation, PresenceEntry,
};
use crate::{
    CollaborationError, Conflict, MergeReport, Operation, OperationId, Presence, Result,
    SharedEdit, SharedNodeId, Snapshot, SyncMessage, VersionVector, MAX_DOCUMENT_BYTES,
    MAX_MESSAGE_BYTES, MAX_OPERATIONS, MAX_PRESENCE_ACTORS, MAX_SNAPSHOT_BYTES, PROTOCOL_VERSION,
};
use ferroui_designer_core::{Document, Edit, NodeId};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ops::Bound::{Excluded, Included, Unbounded};

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
    version: VersionVector,
    /// Sum of the contiguous actor prefixes. Equality to the log length means no sequence holes.
    contiguous_operations: usize,
    snapshot_bytes: usize,
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
            baseline.len() <= MAX_DOCUMENT_BYTES,
            "Baseline exceeds the document size limit",
        )?;
        #[derive(Serialize)]
        struct EmptySnapshot<'a> {
            protocol: u32,
            document_id: &'a str,
            baseline: &'a str,
            operations: &'a [Operation],
        }
        let snapshot_bytes = serialized_size(
            &EmptySnapshot {
                protocol: PROTOCOL_VERSION,
                document_id: &document_id,
                baseline: &baseline,
                operations: &[],
            },
            MAX_SNAPSHOT_BYTES,
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
            version: VersionVector::new(),
            contiguous_operations: 0,
            snapshot_bytes,
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
        self.version.clone()
    }
    /// A contiguous vector is a complete checkpoint precondition only when every received
    /// operation is represented by it. A later operation beyond a hole must never be discarded
    /// merely because the visible contiguous prefix has not advanced.
    pub fn has_sequence_gaps(&self) -> bool {
        self.contiguous_operations != self.operations.len()
    }
    /// Count an actor's outstanding writes without cloning source payloads into a wire message.
    pub fn pending_operation_count(&self, version: &VersionVector, actor: &str) -> usize {
        let sequence = version.get(actor).copied().unwrap_or(0);
        self.identities
            .range((
                Excluded((actor.to_owned(), sequence)),
                Included((actor.to_owned(), u64::MAX)),
            ))
            .count()
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
    /// Return one wire-sized page. Filter the writer before paging when uploading an outbox;
    /// otherwise other actors' already-acknowledged operations could consume the page. Continue
    /// with the receiving replica's updated version until no new operations remain. Only complete
    /// sender prefixes are paged: repeatedly sending a sequence beyond a hole could otherwise
    /// fill every page without advancing the receiver's vector.
    pub fn changes_since_bounded(
        &self,
        version: &VersionVector,
        actor: Option<&str>,
        max_bytes: usize,
    ) -> Result<SyncMessage> {
        let limit = max_bytes.min(MAX_MESSAGE_BYTES);
        let mut message = self.message(Vec::new());
        let mut bytes = serialized_size(&message, limit)?;
        for operation in self.operations.values().filter(|operation| {
            actor.is_none_or(|actor| operation.id.actor == actor)
                && operation.id.sequence > version.get(&operation.id.actor).copied().unwrap_or(0)
                && operation.id.sequence <= self.version[&operation.id.actor]
        }) {
            let length = serialized_size(operation, MAX_MESSAGE_BYTES)?;
            let separator = usize::from(!message.operations.is_empty());
            if length.saturating_add(separator) > limit - bytes {
                ensure(
                    !message.operations.is_empty(),
                    "One operation exceeds the synchronization page limit",
                )?;
                break;
            }
            bytes += length + separator;
            message.operations.push(operation.clone());
        }
        Ok(message)
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
        serialized_size(&snapshot, MAX_SNAPSHOT_BYTES)?;
        let mut replica = Self::new(actor, snapshot.document_id.clone(), snapshot.baseline)?;
        replica.merge_operations(snapshot.operations, false)?;
        Ok(replica)
    }
    pub fn merge_snapshot(&mut self, snapshot: Snapshot) -> Result<MergeReport> {
        ensure(
            snapshot.protocol == PROTOCOL_VERSION && snapshot.document_id == self.document_id,
            "Snapshot protocol or document does not match",
        )?;
        ensure(
            snapshot.baseline == self.baseline,
            "Snapshot baseline differs; begin a new session or resolve the source difference",
        )?;
        serialized_size(&snapshot, MAX_SNAPSHOT_BYTES)?;
        self.merge_operations(snapshot.operations, false)
    }
    /// Submit an atomic local transaction, rejecting edits that cannot currently be applied.
    /// Hosts broadcast the returned operation; the new source is available via source().
    pub fn submit(
        &mut self,
        label: impl Into<String>,
        edits: Vec<SharedEdit>,
    ) -> Result<Operation> {
        ensure(
            self.version.get(&self.actor).copied().unwrap_or(0) == self.sequence,
            "Local writer history has sequence gaps; synchronize before authoring more operations",
        )?;
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
        // The local operation is later than every received Lamport timestamp. Validate and
        // project it once, rejecting any conflict before publishing counters or log entries.
        // Remote operations instead retain conflicts for deterministic dependency replay.
        let message = self.message(vec![operation.clone()]);
        message.validate_size()?;
        self.merge_operations(message.operations, true)?;
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
        let local_ast = self.document.ast();
        let local = local_ast
            .element_at(node.span.start)
            .and_then(|id| local_ast.node(id))
            .filter(|candidate| candidate.span == node.span && candidate.name == node.name)
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
        message.validate_size()?;
        self.merge_operations(message.operations, false)
    }

    /// Checkpoint replay uses the larger retained-log budget, not the per-message wire budget.
    /// All validation happens before replacing any document, counter or operation-set state.
    fn merge_operations(
        &mut self,
        operations: Vec<Operation>,
        reject_conflicts: bool,
    ) -> Result<MergeReport> {
        ensure(
            operations.len() <= MAX_OPERATIONS,
            "Too many operations in a batch",
        )?;
        let mut pending: BTreeMap<OperationId, Operation> = BTreeMap::new();
        let mut pending_ids: BTreeMap<(String, u64), OperationId> = BTreeMap::new();
        let envelope_bytes = serialized_size(&self.message(Vec::new()), MAX_MESSAGE_BYTES)?;
        let mut snapshot_bytes = self.snapshot_bytes;
        for operation in operations {
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
            let operation_bytes = serialized_size(&operation, MAX_MESSAGE_BYTES - envelope_bytes)?;
            let separator = usize::from(!self.operations.is_empty() || !pending.is_empty());
            snapshot_bytes = snapshot_bytes.saturating_add(operation_bytes + separator);
            ensure(
                snapshot_bytes <= MAX_SNAPSHOT_BYTES,
                "Session snapshot size limit reached; checkpoint and start a new session",
            )?;
            pending_ids.insert(actor_key, operation.id.clone());
            pending.insert(operation.id.clone(), operation);
        }
        ensure(
            self.operations.len().saturating_add(pending.len()) <= MAX_OPERATIONS,
            "Session operation limit reached; checkpoint and start a new session",
        )?;
        for (key, id) in &pending_ids {
            validate_neighbors(&self.identities, key, id)?;
            validate_neighbors(&pending_ids, key, id)?;
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
        // Merge the ordered borrowed streams. Out-of-order delivery must replay old operations,
        // but does not need another allocation/copy of every retained source payload.
        let replay_count = if append_only {
            0
        } else {
            self.operations.len()
        };
        let mut previous = self.operations.values().take(replay_count).peekable();
        let mut incoming = pending.values().peekable();
        let ordered = std::iter::from_fn(|| match (previous.peek(), incoming.peek()) {
            (Some(left), Some(right)) if left.id < right.id => previous.next(),
            (_, Some(_)) => incoming.next(),
            _ => previous.next(),
        });
        for operation in ordered {
            let result = if reject_conflicts {
                // A failed local projection discards this whole candidate; no nested rollback
                // copy is needed. Remote batches must retain earlier successful operations.
                apply_to_projection(&mut document, &mut aliases, &mut writers, operation)
            } else {
                apply_operation(&mut document, &mut aliases, &mut writers, operation)
            };
            match result {
                Ok(()) => applied += 1,
                Err(err) if reject_conflicts => return Err(err),
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
        self.identities.extend(
            pending_ids
                .iter()
                .map(|(key, id)| (key.clone(), id.clone())),
        );
        // Each sequence joins its contiguous prefix at most once over the session lifetime.
        // Inspect each changed actor once, rather than sorting the entire log for every poll.
        let mut previous_actor: Option<&str> = None;
        for (actor, _) in pending_ids.keys() {
            if previous_actor == Some(actor) {
                continue;
            }
            previous_actor = Some(actor);
            let contiguous = self.version.entry(actor.clone()).or_default();
            let mut next = (actor.clone(), *contiguous);
            while let Some(sequence) = next.1.checked_add(1) {
                next.1 = sequence;
                if !self.identities.contains_key(&next) {
                    break;
                }
                *contiguous = sequence;
                self.contiguous_operations += 1;
            }
        }
        self.snapshot_bytes = snapshot_bytes;
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
        for node in &presence.selection {
            validate_node(node)?;
        }
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
        ensure(
            self.presence.contains_key(&presence.actor)
                || self.presence.len() < MAX_PRESENCE_ACTORS,
            "Presence actor limit reached; prune expired entries before admitting more peers",
        )?;
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

/// Existing and incoming identities are independently ordered by actor and sequence. Checking
/// each insertion's immediate neighbors is sufficient to validate their merged order, and costs
/// O(new operations * log(retained operations)) rather than rescanning every actor's full log.
fn validate_neighbors(
    identities: &BTreeMap<(String, u64), OperationId>,
    key: &(String, u64),
    id: &OperationId,
) -> Result<()> {
    let previous = identities
        .range(..key.clone())
        .next_back()
        .map(|(_, previous)| previous)
        .filter(|previous| previous.actor == id.actor);
    let next = identities
        .range((Excluded(key.clone()), Unbounded))
        .next()
        .map(|(_, next)| next)
        .filter(|next| next.actor == id.actor);
    for (earlier, later) in previous
        .into_iter()
        .map(|previous| (previous, id))
        .chain(next.into_iter().map(|next| (id, next)))
    {
        ensure(
            later.lamport.saturating_sub(earlier.lamport) >= later.sequence - earlier.sequence,
            "Actor timestamps do not leave causal space for every sequence",
        )?;
    }
    Ok(())
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
    apply_to_projection(
        &mut candidate,
        &mut mapped,
        &mut candidate_writers,
        operation,
    )?;
    *document = candidate;
    *aliases = mapped;
    *writers = candidate_writers;
    Ok(())
}

/// Mutate an isolated projection. The caller either commits all of it or discards it on error.
fn apply_to_projection(
    document: &mut Document,
    aliases: &mut BTreeMap<SharedNodeId, NodeId>,
    writers: &mut WriterState,
    operation: &Operation,
) -> Result<()> {
    let mut index = 0;
    while index < operation.edits.len() {
        let remaining = &operation.edits[index..];
        // Match Document's transaction boundaries, including attribute runs between structural
        // edits. A namespace declaration may follow its first use within the same atomic run.
        let count = if is_attribute_edit(&remaining[0]) {
            remaining
                .iter()
                .take_while(|edit| is_attribute_edit(edit))
                .count()
        } else {
            1
        };
        let shared = &remaining[..count];
        let edits = shared
            .iter()
            .map(|edit| local_edit(document, aliases, writers, edit))
            .collect::<Result<Vec<_>>>()?;
        let change = document
            .apply(operation.label.clone(), edits)
            .map_err(error)?;
        register_inserted(document, aliases, operation, index, &change.inserted);
        for edit in shared {
            writers.record(edit, &operation.id);
        }
        index += count;
    }
    validate_document_size(document)
}

fn is_attribute_edit(edit: &SharedEdit) -> bool {
    matches!(
        edit,
        SharedEdit::SetAttribute { .. }
            | SharedEdit::RemoveAttribute { .. }
            | SharedEdit::UndoAttribute { .. }
    )
}

/// Translate shared identities and conditional writer preconditions at one syntax boundary.
fn local_edit(
    document: &Document,
    aliases: &BTreeMap<SharedNodeId, NodeId>,
    writers: &WriterState,
    edit: &SharedEdit,
) -> Result<Edit> {
    let resolve = |node: &SharedNodeId| {
        aliases
            .get(node)
            .copied()
            .filter(|id| document.ast().node(*id).is_some())
            .ok_or_else(|| {
                CollaborationError(format!(
                    "Missing or deleted node {node:?}; retained for deterministic replay",
                ))
            })
    };
    Ok(match edit {
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
                writers.last(node, name) == Some(expected_writer),
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
                document.source() == expected_source,
                "Concurrent source change: source replacement precondition does not match",
            )?;
            Edit::ReplaceSource {
                source: source.clone(),
            }
        }
    })
}

fn validate_document_size(document: &Document) -> Result<()> {
    ensure(
        document.source().len() <= MAX_DOCUMENT_BYTES,
        "Edited document exceeds the document size limit",
    )
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
