mod changes;
mod editing;

use crate::{Ast, NodeId, SourceSpan};
use changes::{diff, unchanged};
use editing::{apply_edits, patched_source};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub line: u32,
    pub column: u32,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesignerError {
    Parse {
        message: String,
        line: u32,
        column: u32,
    },
    UnknownNode(NodeId),
    NotElement(NodeId),
    InvalidRange(SourceSpan),
    InvalidEdit(String),
    InvalidDocument,
    RevisionConflict {
        expected: u64,
        actual: u64,
    },
    NothingToUndo,
    NothingToRedo,
}
impl fmt::Display for DesignerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                message,
                line,
                column,
            } => write!(f, "{message} ({line}:{column})"),
            Self::UnknownNode(id) => write!(f, "Unknown document node {id}"),
            Self::NotElement(id) => write!(f, "Node {id} is not an element"),
            Self::InvalidRange(span) => {
                write!(f, "Invalid UTF-8 source range {}..{}", span.start, span.end)
            }
            Self::InvalidEdit(message) => f.write_str(message),
            Self::InvalidDocument => f.write_str(
                "Visual edits require valid authored XAML; repair the source buffer first",
            ),
            Self::RevisionConflict { expected, actual } => write!(
                f,
                "Document revision changed: expected {expected}, actual {actual}"
            ),
            Self::NothingToUndo => f.write_str("No edit to undo"),
            Self::NothingToRedo => f.write_str("No edit to redo"),
        }
    }
}
impl std::error::Error for DesignerError {}

/// Commands are evaluated against a private candidate, then committed once.
/// Consecutive attribute edits are validated together, allowing a property and
/// its namespace declaration in either order. Other commands observe preceding
/// commands. Element-child indices exclude text, whitespace and comments. Move
/// indices refer to the destination list after removing the moving node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edit {
    SetAttribute {
        node: NodeId,
        name: String,
        value: String,
    },
    RemoveAttribute {
        node: NodeId,
        name: String,
    },
    InsertChild {
        parent: NodeId,
        index: usize,
        source: String,
    },
    RemoveNode {
        node: NodeId,
    },
    MoveNode {
        node: NodeId,
        parent: NodeId,
        index: usize,
    },
    ReplaceSource {
        source: String,
    },
    /// A UTF-8 byte-range edit in the current candidate source. For simultaneous
    /// source-coordinate patches use [`Document::apply_text_patches`].
    Text {
        span: SourceSpan,
        replacement: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub label: String,
    pub expected_revision: Option<u64>,
    pub edits: Vec<Edit>,
}
impl Transaction {
    pub fn new(label: impl Into<String>, edits: Vec<Edit>) -> Self {
        Self {
            label: label.into(),
            expected_revision: None,
            edits,
        }
    }
    pub fn at_revision(mut self, revision: u64) -> Self {
        self.expected_revision = Some(revision);
        self
    }
}

/// Patch positions are UTF-8 bytes in the *previous* source snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPatch {
    pub span: SourceSpan,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeSet {
    pub label: String,
    pub previous_revision: u64,
    pub revision: u64,
    pub inserted: Vec<NodeId>,
    pub removed: Vec<NodeId>,
    pub changed: Vec<NodeId>,
    /// A minimal contiguous replacement of the source, suitable for editor sync.
    pub patches: Vec<TextPatch>,
    pub valid: bool,
}
impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.previous_revision == self.revision
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceUpdate {
    pub change: ChangeSet,
    pub valid: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone)]
struct Snapshot {
    source: Arc<str>,
    valid_source: Arc<str>,
    ast: Arc<Ast>,
    diagnostics: Vec<Diagnostic>,
}
#[derive(Clone)]
struct HistoryEntry {
    label: String,
    before: Snapshot,
    after: Snapshot,
}

/// An owned source buffer with a last-valid syntax snapshot and bounded history.
/// Runtime acceptance is deliberately separate: the host may keep an older live
/// tree when compilation fails, while authored and syntactically valid XAML stays
/// available for correction.
#[derive(Clone)]
pub struct Document {
    current: Snapshot,
    revision: u64,
    next_id: NodeId,
    undo: VecDeque<HistoryEntry>,
    redo: VecDeque<HistoryEntry>,
    history_limit: usize,
    saved_source: Arc<str>,
}
impl Document {
    pub fn new(source: impl Into<String>) -> Result<Self, DesignerError> {
        let source = source.into();
        let mut ast = Ast::parse(&source)?;
        let mut next_id = 1;
        ast.allocate(&mut next_id);
        let source: Arc<str> = source.into();
        Ok(Self {
            saved_source: source.clone(),
            current: Snapshot {
                source: source.clone(),
                valid_source: source,
                ast: Arc::new(ast),
                diagnostics: Vec::new(),
            },
            revision: 0,
            next_id,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            history_limit: 256,
        })
    }
    pub fn source(&self) -> &str {
        &self.current.source
    }
    pub fn valid_source(&self) -> &str {
        &self.current.valid_source
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    /// Rebase a restored tab onto its host's monotonic revision sequence without altering
    /// source, stable node IDs, dirty state or history. Reserve the next revision for editing.
    pub fn advance_revision_to(&mut self, minimum: u64) -> Result<(), DesignerError> {
        if minimum == u64::MAX || self.revision == u64::MAX {
            return Err(DesignerError::InvalidEdit(
                "Document revision exhausted".into(),
            ));
        }
        self.revision = self.revision.max(minimum);
        Ok(())
    }
    pub fn ast(&self) -> &Ast {
        &self.current.ast
    }
    /// Share the exact immutable syntax snapshot without copying its nodes.
    /// A gesture can retain this handle together with a revision to reject a
    /// command prepared for a different document, even if its local IDs match.
    /// Invalid source returns the last-valid syntax, like `ast()`.
    pub fn ast_snapshot(&self) -> Arc<Ast> {
        self.current.ast.clone()
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.current.diagnostics
    }
    pub fn is_valid(&self) -> bool {
        self.current.diagnostics.is_empty()
    }
    pub fn is_dirty(&self) -> bool {
        self.saved_source != self.current.source
    }
    pub fn mark_saved(&mut self) {
        self.saved_source.clone_from(&self.current.source);
    }
    /// Acknowledge the exact snapshot written by an asynchronous storage provider. The
    /// current buffer may already contain newer edits; revision and history are unchanged.
    pub fn mark_saved_snapshot(&mut self, source: impl Into<String>) {
        self.saved_source = source.into().into();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.back().map(|entry| entry.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.back().map(|entry| entry.label.as_str())
    }
    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = limit;
        trim_history(&mut self.undo, limit);
        trim_history(&mut self.redo, limit);
    }
    pub fn apply(
        &mut self,
        label: impl Into<String>,
        edits: Vec<Edit>,
    ) -> Result<ChangeSet, DesignerError> {
        self.transact(Transaction::new(label, edits))
    }
    pub fn apply_at(
        &mut self,
        expected_revision: u64,
        label: impl Into<String>,
        edits: Vec<Edit>,
    ) -> Result<ChangeSet, DesignerError> {
        self.transact(Transaction::new(label, edits).at_revision(expected_revision))
    }
    pub fn transact(&mut self, transaction: Transaction) -> Result<ChangeSet, DesignerError> {
        self.check_revision(transaction.expected_revision)?;
        if transaction.edits.is_empty() {
            return Ok(unchanged(&self.current, transaction.label, self.revision));
        }
        if !self.is_valid() && !matches!(transaction.edits.as_slice(), [Edit::ReplaceSource { .. }])
        {
            return Err(DesignerError::InvalidDocument);
        }
        let mut next_id = self.next_id;
        let candidate = apply_edits(self.current.clone(), transaction.edits, &mut next_id)?;
        let change = self.commit(transaction.label, candidate)?;
        self.next_id = next_id;
        Ok(change)
    }

    /// Apply disjoint editor patches atomically. All spans refer to the current
    /// authored source; UTF-8 boundaries and overlap are checked before mutation.
    pub fn apply_text_patches(
        &mut self,
        expected_revision: u64,
        label: impl Into<String>,
        patches: Vec<TextPatch>,
    ) -> Result<ChangeSet, DesignerError> {
        self.check_revision(Some(expected_revision))?;
        if patches.is_empty() {
            return Ok(unchanged(&self.current, label.into(), self.revision));
        }
        if !self.is_valid() {
            return Err(DesignerError::InvalidDocument);
        }
        let (source, pins) = patched_source(self.source(), self.ast(), patches, &[])?;
        let mut next_id = self.next_id;
        let snapshot = parse_snapshot(source, &self.current, &mut next_id, &pins)?;
        let change = self.commit(label.into(), snapshot)?;
        self.next_id = next_id;
        Ok(change)
    }

    /// Source editors may temporarily contain invalid XML. The raw buffer and
    /// diagnostics enter history, while the AST/preview source remain the last
    /// valid snapshot. A subsequent valid source update reconciles node IDs.
    pub fn update_source(
        &mut self,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> SourceUpdate {
        match self.try_update_source(label, source) {
            Ok(update) => update,
            Err(error) => SourceUpdate {
                change: unchanged(&self.current, "Rejected source edit".into(), self.revision),
                valid: false,
                diagnostics: vec![diagnostic(self.source(), &error)],
            },
        }
    }

    /// Fallible source editing for hosts that must distinguish a rejected write
    /// from an accepted but temporarily invalid XML buffer. Revision exhaustion
    /// cannot alter source, node identity, history or saved-state tracking.
    pub fn try_update_source(
        &mut self,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<SourceUpdate, DesignerError> {
        let source = source.into();
        let label = label.into();
        if source == self.source() {
            return Ok(SourceUpdate {
                change: unchanged(&self.current, label, self.revision),
                valid: self.is_valid(),
                diagnostics: self.diagnostics().to_vec(),
            });
        }
        self.next_revision()?;
        let mut next_id = self.next_id;
        let snapshot = match parse_snapshot(source.clone(), &self.current, &mut next_id, &[]) {
            Ok(snapshot) => snapshot,
            Err(error) => Snapshot {
                diagnostics: vec![diagnostic(&source, &error)],
                source: source.into(),
                valid_source: self.current.valid_source.clone(),
                ast: self.current.ast.clone(),
            },
        };
        let change = self.commit(label, snapshot)?;
        self.next_id = next_id;
        Ok(SourceUpdate {
            valid: self.is_valid(),
            diagnostics: self.diagnostics().to_vec(),
            change,
        })
    }
    pub fn update_source_at(
        &mut self,
        expected_revision: u64,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<SourceUpdate, DesignerError> {
        self.check_revision(Some(expected_revision))?;
        self.try_update_source(label, source)
    }
    pub fn undo(&mut self) -> Result<ChangeSet, DesignerError> {
        if self.undo.is_empty() {
            return Err(DesignerError::NothingToUndo);
        }
        let revision = self.next_revision()?;
        let entry = self.undo.pop_back().expect("nonempty undo history");
        let before = std::mem::replace(&mut self.current, entry.before.clone());
        let previous_revision = self.revision;
        self.revision = revision;
        let change = diff(
            &before,
            &self.current,
            format!("Undo {}", entry.label),
            previous_revision,
            self.revision,
        );
        self.redo.push_back(entry);
        trim_history(&mut self.redo, self.history_limit);
        Ok(change)
    }
    pub fn redo(&mut self) -> Result<ChangeSet, DesignerError> {
        if self.redo.is_empty() {
            return Err(DesignerError::NothingToRedo);
        }
        let revision = self.next_revision()?;
        let entry = self.redo.pop_back().expect("nonempty redo history");
        let before = std::mem::replace(&mut self.current, entry.after.clone());
        let previous_revision = self.revision;
        self.revision = revision;
        let change = diff(
            &before,
            &self.current,
            format!("Redo {}", entry.label),
            previous_revision,
            self.revision,
        );
        self.undo.push_back(entry);
        trim_history(&mut self.undo, self.history_limit);
        Ok(change)
    }
    fn check_revision(&self, expected: Option<u64>) -> Result<(), DesignerError> {
        if let Some(expected) = expected {
            if expected != self.revision {
                return Err(DesignerError::RevisionConflict {
                    expected,
                    actual: self.revision,
                });
            }
        }
        Ok(())
    }
    fn next_revision(&self) -> Result<u64, DesignerError> {
        self.revision
            .checked_add(1)
            .ok_or_else(|| DesignerError::InvalidEdit("Document revision exhausted".into()))
    }

    fn commit(&mut self, label: String, snapshot: Snapshot) -> Result<ChangeSet, DesignerError> {
        if snapshot.source == self.current.source
            && (Arc::ptr_eq(&snapshot.ast, &self.current.ast) || snapshot.ast == self.current.ast)
            && snapshot.diagnostics == self.current.diagnostics
        {
            return Ok(unchanged(&self.current, label, self.revision));
        }
        let revision = self.next_revision()?;
        let before = std::mem::replace(&mut self.current, snapshot);
        let previous_revision = self.revision;
        self.revision = revision;
        let change = diff(
            &before,
            &self.current,
            label.clone(),
            previous_revision,
            self.revision,
        );
        if self.history_limit > 0 {
            self.undo.push_back(HistoryEntry {
                label,
                before,
                after: self.current.clone(),
            });
            trim_history(&mut self.undo, self.history_limit);
        }
        self.redo.clear();
        Ok(change)
    }
}

fn parse_snapshot(
    source: String,
    old: &Snapshot,
    next_id: &mut NodeId,
    pins: &[(usize, NodeId)],
) -> Result<Snapshot, DesignerError> {
    // Source-only restoration can reuse the exact last-valid syntax. Explicit
    // pins must still reconcile: moving identical siblings may change identity
    // without changing a single source byte.
    if pins.is_empty() && source == old.valid_source.as_ref() {
        return Ok(Snapshot {
            source: old.valid_source.clone(),
            valid_source: old.valid_source.clone(),
            ast: old.ast.clone(),
            diagnostics: Vec::new(),
        });
    }
    let mut ast = Ast::parse(&source)?;
    ast.reconcile(&old.ast, &old.valid_source, &source, next_id, pins);
    let source: Arc<str> = source.into();
    Ok(Snapshot {
        source: source.clone(),
        valid_source: source,
        ast: Arc::new(ast),
        diagnostics: Vec::new(),
    })
}

fn trim_history(history: &mut VecDeque<HistoryEntry>, limit: usize) {
    if history.len() > limit {
        history.drain(..history.len() - limit);
    }
}

#[cfg(test)]
mod tests;

fn diagnostic(source: &str, error: &DesignerError) -> Diagnostic {
    let (line, column) = match error {
        DesignerError::Parse { line, column, .. } => (*line, *column),
        _ => (1, 1),
    };
    let mut offset = 0;
    for (index, text) in source.split_inclusive('\n').enumerate() {
        if index + 1 == line as usize {
            offset += text
                .chars()
                .take(column.saturating_sub(1) as usize)
                .map(char::len_utf8)
                .sum::<usize>();
            break;
        }
        offset += text.len();
    }
    offset = offset.min(source.len());
    Diagnostic {
        message: error.to_string(),
        line,
        column,
        span: SourceSpan::new(offset, offset),
    }
}
