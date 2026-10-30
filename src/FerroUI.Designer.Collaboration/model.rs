use crate::{CollaborationError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;
/// Checkpoints retain several wire batches, without allowing an unbounded operation log.
pub const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_OPERATIONS: usize = 100_000;
pub const MAX_PRESENCE_ACTORS: usize = 1024;

/// A causal Lamport timestamp with deterministic actor/sequence tie-breaking. Actor identifiers
/// must be unique per durable writer; persist a writer's sequence or use a new actor after restart.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct OperationId {
    pub lamport: u64,
    pub actor: String,
    pub sequence: u64,
}
/// Names are shared across replicas, unlike process-local AST node numbers.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum SharedNodeId {
    Baseline(u64),
    Inserted {
        actor: String,
        sequence: u64,
        edit: usize,
        ordinal: usize,
    },
}
/// Each operation is one atomic transaction. A source replacement is compare-and-set: it is
/// never allowed to silently erase unrelated concurrent edits. Structural commands use the
/// source-preserving AST engine and are re-evaluated after out-of-order delivery.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SharedEdit {
    SetAttribute {
        node: SharedNodeId,
        name: String,
        value: String,
    },
    RemoveAttribute {
        node: SharedNodeId,
        name: String,
    },
    /// Compensate only if the named operation still owns this property at replay
    /// time. Operation identity matters even when another writer chose the same value.
    UndoAttribute {
        node: SharedNodeId,
        name: String,
        expected_writer: OperationId,
        value: Option<String>,
    },
    InsertChild {
        parent: SharedNodeId,
        index: usize,
        source: String,
    },
    RemoveNode {
        node: SharedNodeId,
    },
    MoveNode {
        node: SharedNodeId,
        parent: SharedNodeId,
        index: usize,
    },
    ReplaceSource {
        expected_source: String,
        source: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub id: OperationId,
    pub document_id: String,
    pub label: String,
    pub edits: Vec<SharedEdit>,
}
/// Highest contiguous sequence received per actor. Gaps do not advance a peer's summary; replay
/// synchronization may resend later operations, which are idempotently deduplicated.
pub type VersionVector = BTreeMap<String, u64>;
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncMessage {
    pub protocol: u32,
    pub document_id: String,
    /// SHA-256 of the exact UTF-8 baseline, rejecting accidental cross-session messages.
    pub baseline_hash: String,
    pub operations: Vec<Operation>,
}
impl SyncMessage {
    pub fn to_json(&self) -> Result<String> {
        let source = serde_json::to_string(self).map_err(|e| CollaborationError(e.to_string()))?;
        ensure(
            source.len() <= MAX_MESSAGE_BYTES,
            "Synchronization message exceeds the size limit",
        )?;
        Ok(source)
    }
    pub fn from_json(source: &str) -> Result<Self> {
        ensure(
            source.len() <= MAX_MESSAGE_BYTES,
            "Synchronization message exceeds the size limit",
        )?;
        serde_json::from_str(source).map_err(|e| CollaborationError(e.to_string()))
    }

    pub(crate) fn validate_size(&self) -> Result<()> {
        serialized_size(self, MAX_MESSAGE_BYTES).map(|_| ())
    }
}
/// A complete transport-independent checkpoint. Exact baseline comparison prevents a snapshot
/// from being accidentally merged into a session based on different source text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub protocol: u32,
    pub document_id: String,
    pub baseline: String,
    pub operations: Vec<Operation>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    pub operation: OperationId,
    pub reason: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeReport {
    pub received: usize,
    pub applied: usize,
    pub conflicts: Vec<Conflict>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cursor {
    pub x: f64,
    pub y: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Presence {
    pub actor: String,
    pub sequence: u64,
    pub display_name: String,
    pub selection: Vec<SharedNodeId>,
    pub cursor: Option<Cursor>,
}
#[derive(Clone, Debug)]
pub(crate) struct PresenceEntry {
    pub presence: Presence,
    pub observed_at: u64,
}

pub(crate) fn ensure(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(CollaborationError(message.into()))
    }
}
pub(crate) fn identifier(value: &str) -> Result<()> {
    ensure(
        !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control),
        "Identifiers must contain 1..256 non-control UTF-8 bytes",
    )
}
pub(crate) fn validate_operation(operation: &Operation) -> Result<()> {
    identifier(&operation.id.actor)?;
    identifier(&operation.document_id)?;
    ensure(
        operation.id.lamport >= operation.id.sequence && operation.id.sequence > 0,
        "Operation sequence must be nonzero and no greater than its Lamport timestamp",
    )?;
    ensure(operation.label.len() <= 4096, "Operation label is too long")?;
    ensure(
        !operation.edits.is_empty() && operation.edits.len() <= 4096,
        "Transaction must have 1..4096 edits",
    )?;
    let mut bytes = 0usize;
    for edit in &operation.edits {
        match edit {
            SharedEdit::SetAttribute { node, name, value } => {
                validate_node(node)?;
                identifier(name)?;
                bytes = bytes.saturating_add(value.len());
            }
            SharedEdit::RemoveAttribute { node, name } => {
                validate_node(node)?;
                identifier(name)?;
            }
            SharedEdit::UndoAttribute {
                node,
                name,
                expected_writer,
                value,
            } => {
                validate_node(node)?;
                identifier(name)?;
                identifier(&expected_writer.actor)?;
                ensure(
                    expected_writer.sequence > 0
                        && expected_writer.lamport >= expected_writer.sequence
                        && expected_writer.lamport < operation.id.lamport,
                    "Undo precondition must identify a causally earlier property writer",
                )?;
                bytes = bytes.saturating_add(value.as_ref().map_or(0, String::len));
            }
            SharedEdit::InsertChild {
                parent,
                index,
                source,
            } => {
                validate_node(parent)?;
                ensure(*index <= 1_000_000, "Insertion index is too large")?;
                bytes = bytes.saturating_add(source.len());
            }
            SharedEdit::RemoveNode { node } => validate_node(node)?,
            SharedEdit::MoveNode {
                node,
                parent,
                index,
            } => {
                validate_node(node)?;
                validate_node(parent)?;
                ensure(*index <= 1_000_000, "Move index is too large")?;
            }
            SharedEdit::ReplaceSource {
                expected_source,
                source,
            } => {
                bytes = bytes
                    .saturating_add(expected_source.len())
                    .saturating_add(source.len());
            }
        }
    }
    ensure(
        bytes <= MAX_MESSAGE_BYTES,
        "Transaction text exceeds the size limit",
    )
}
pub(crate) fn validate_node(node: &SharedNodeId) -> Result<()> {
    match node {
        SharedNodeId::Baseline(id) => ensure(*id > 0, "Baseline node must be nonzero"),
        SharedNodeId::Inserted {
            actor,
            sequence,
            edit,
            ordinal,
        } => {
            identifier(actor)?;
            ensure(
                *sequence > 0 && *edit < 4096 && *ordinal < 1_000_000,
                "Invalid inserted node identity",
            )
        }
    }
}

/// Measure the actual wire representation, including JSON escapes, without allocating a second
/// copy of a potentially large document. Stop serialization as soon as the limit is exceeded.
pub(crate) fn serialized_size(value: &impl Serialize, limit: usize) -> Result<usize> {
    struct Counter {
        bytes: usize,
        limit: usize,
    }
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes = self.bytes.saturating_add(bytes.len());
            if self.bytes > self.limit {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Serialized collaboration data exceeds the size limit",
                ));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter { bytes: 0, limit };
    serde_json::to_writer(&mut counter, value)
        .map_err(|error| CollaborationError(error.to_string()))?;
    Ok(counter.bytes)
}
