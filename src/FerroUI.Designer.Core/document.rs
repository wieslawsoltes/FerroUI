use crate::{Ast, Node, NodeId, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
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

/// Commands are evaluated sequentially against a private candidate, then committed
/// once. Element-child indices exclude text, whitespace and comments. Move indices
/// refer to the destination list after removing the moving node.
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
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
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
            undo: Vec::new(),
            redo: Vec::new(),
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
    pub fn ast(&self) -> &Ast {
        &self.current.ast
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
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|entry| entry.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|entry| entry.label.as_str())
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
        if !self.is_valid() && !matches!(transaction.edits.as_slice(), [Edit::ReplaceSource { .. }])
        {
            return Err(DesignerError::InvalidDocument);
        }
        let mut candidate = self.current.clone();
        let mut next_id = self.next_id;
        if transaction.edits.iter().all(|edit| {
            matches!(
                edit,
                Edit::SetAttribute { .. } | Edit::RemoveAttribute { .. }
            )
        }) {
            candidate = apply_attribute_edits(candidate, transaction.edits, &mut next_id)?;
        } else {
            for edit in transaction.edits {
                candidate = apply_edit(candidate, edit, &mut next_id)?;
            }
        }
        self.next_id = next_id;
        Ok(self.commit(transaction.label, candidate))
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
        if !self.is_valid() {
            return Err(DesignerError::InvalidDocument);
        }
        let (source, pins) = patched_source(self.source(), self.ast(), patches, &[])?;
        let mut next_id = self.next_id;
        let snapshot = parse_snapshot(source, &self.current, &mut next_id, &pins)?;
        self.next_id = next_id;
        Ok(self.commit(label.into(), snapshot))
    }

    /// Source editors may temporarily contain invalid XML. The raw buffer and
    /// diagnostics enter history, while the AST/preview source remain the last
    /// valid snapshot. A subsequent valid source update reconciles node IDs.
    pub fn update_source(
        &mut self,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> SourceUpdate {
        let source = source.into();
        let mut next_id = self.next_id;
        let snapshot = match parse_snapshot(source.clone(), &self.current, &mut next_id, &[]) {
            Ok(snapshot) => {
                self.next_id = next_id;
                snapshot
            }
            Err(error) => Snapshot {
                diagnostics: vec![diagnostic(&source, &error)],
                source: source.into(),
                valid_source: self.current.valid_source.clone(),
                ast: self.current.ast.clone(),
            },
        };
        let change = self.commit(label.into(), snapshot);
        SourceUpdate {
            valid: self.is_valid(),
            diagnostics: self.diagnostics().to_vec(),
            change,
        }
    }
    pub fn update_source_at(
        &mut self,
        expected_revision: u64,
        label: impl Into<String>,
        source: impl Into<String>,
    ) -> Result<SourceUpdate, DesignerError> {
        self.check_revision(Some(expected_revision))?;
        Ok(self.update_source(label, source))
    }
    pub fn undo(&mut self) -> Result<ChangeSet, DesignerError> {
        let entry = self.undo.pop().ok_or(DesignerError::NothingToUndo)?;
        let before = std::mem::replace(&mut self.current, entry.before.clone());
        let previous_revision = self.revision;
        self.revision += 1;
        let change = diff(
            &before,
            &self.current,
            format!("Undo {}", entry.label),
            previous_revision,
            self.revision,
        );
        self.redo.push(entry);
        trim_history(&mut self.redo, self.history_limit);
        Ok(change)
    }
    pub fn redo(&mut self) -> Result<ChangeSet, DesignerError> {
        let entry = self.redo.pop().ok_or(DesignerError::NothingToRedo)?;
        let before = std::mem::replace(&mut self.current, entry.after.clone());
        let previous_revision = self.revision;
        self.revision += 1;
        let change = diff(
            &before,
            &self.current,
            format!("Redo {}", entry.label),
            previous_revision,
            self.revision,
        );
        self.undo.push(entry);
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
    fn commit(&mut self, label: String, snapshot: Snapshot) -> ChangeSet {
        if snapshot.source == self.current.source
            && snapshot.ast == self.current.ast
            && snapshot.diagnostics == self.current.diagnostics
        {
            return diff(
                &self.current,
                &self.current,
                label,
                self.revision,
                self.revision,
            );
        }
        let before = std::mem::replace(&mut self.current, snapshot);
        let previous_revision = self.revision;
        self.revision += 1;
        let change = diff(
            &before,
            &self.current,
            label.clone(),
            previous_revision,
            self.revision,
        );
        self.undo.push(HistoryEntry {
            label,
            before,
            after: self.current.clone(),
        });
        trim_history(&mut self.undo, self.history_limit);
        self.redo.clear();
        change
    }
}

fn parse_snapshot(
    source: String,
    old: &Snapshot,
    next_id: &mut NodeId,
    pins: &[(usize, NodeId)],
) -> Result<Snapshot, DesignerError> {
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

fn element(ast: &Ast, id: NodeId) -> Result<&Node, DesignerError> {
    let node = ast.node(id).ok_or(DesignerError::UnknownNode(id))?;
    if !node.is_element() {
        return Err(DesignerError::NotElement(id));
    }
    Ok(node)
}

/// Multi-selection property edits are planned against one snapshot and parsed
/// once. Last write to a property wins within the transaction. This also permits
/// an attribute and its namespace declaration to be authored atomically.
fn apply_attribute_edits(
    old: Snapshot,
    edits: Vec<Edit>,
    next_id: &mut NodeId,
) -> Result<Snapshot, DesignerError> {
    let mut values = BTreeMap::<NodeId, Vec<(String, Option<String>)>>::new();
    for edit in edits {
        let (id, name, value) = match edit {
            Edit::SetAttribute { node, name, value } => {
                validate_name(&name)?;
                (node, name, Some(value))
            }
            Edit::RemoveAttribute { node, name } => (node, name, None),
            _ => unreachable!("caller filters attribute operations"),
        };
        element(&old.ast, id)?;
        let attributes = values.entry(id).or_default();
        if let Some(attribute) = attributes
            .iter_mut()
            .find(|(existing, _)| *existing == name)
        {
            attribute.1 = value;
        } else {
            attributes.push((name, value));
        }
    }
    let mut patches = Vec::new();
    for (id, values) in values {
        let node = element(&old.ast, id)?;
        let quote = node.attributes.first().map_or('"', |a| a.quote);
        let mut added = String::new();
        for (name, value) in values {
            match (node.attribute_info(&name), value) {
                (Some(attribute), Some(value)) if attribute.value != value => {
                    patches.push(TextPatch {
                        span: attribute.value_span,
                        replacement: escape_attribute(&value, attribute.quote),
                    })
                }
                (Some(attribute), None) => patches.push(TextPatch {
                    span: attribute.span,
                    replacement: String::new(),
                }),
                (None, Some(value)) => added.push_str(&format!(
                    " {name}={quote}{}{quote}",
                    escape_attribute(&value, quote)
                )),
                _ => {}
            }
        }
        if !added.is_empty() {
            let offset = node.opening_span.end - if node.self_closing { 2 } else { 1 };
            patches.push(TextPatch {
                span: SourceSpan::new(offset, offset),
                replacement: added,
            });
        }
    }
    if patches.is_empty() {
        return Ok(old);
    }
    let (source, pins) = patched_source(&old.source, &old.ast, patches, &[])?;
    parse_snapshot(source, &old, next_id, &pins)
}

fn apply_edit(old: Snapshot, edit: Edit, next_id: &mut NodeId) -> Result<Snapshot, DesignerError> {
    let mut extra_pins = Vec::new();
    let patches = match edit {
        Edit::ReplaceSource { source } => return parse_snapshot(source, &old, next_id, &[]),
        Edit::Text { span, replacement } => vec![TextPatch { span, replacement }],
        Edit::SetAttribute { node, name, value } => {
            validate_name(&name)?;
            let node = element(&old.ast, node)?;
            if let Some(attribute) = node.attribute_info(&name) {
                if attribute.value == value {
                    return Ok(old);
                }
                vec![TextPatch {
                    span: attribute.value_span,
                    replacement: escape_attribute(&value, attribute.quote),
                }]
            } else {
                let quote = node.attributes.first().map_or('"', |a| a.quote);
                let offset = node.opening_span.end - if node.self_closing { 2 } else { 1 };
                vec![TextPatch {
                    span: SourceSpan::new(offset, offset),
                    replacement: format!(
                        " {name}={quote}{}{quote}",
                        escape_attribute(&value, quote)
                    ),
                }]
            }
        }
        Edit::RemoveAttribute { node, name } => {
            let node = element(&old.ast, node)?;
            let Some(attribute) = node.attribute_info(&name) else {
                return Ok(old);
            };
            // Preserve pre-existing whitespace, including multiline alignment.
            vec![TextPatch {
                span: attribute.span,
                replacement: String::new(),
            }]
        }
        Edit::InsertChild {
            parent,
            index,
            source,
        } => {
            let parent = element(&old.ast, parent)?;
            let children = old.ast.element_children(parent.id);
            insertion_patches(parent, &old.ast, &children, index, &source)?.0
        }
        Edit::RemoveNode { node } => {
            if node == old.ast.root() {
                return Err(DesignerError::InvalidEdit(
                    "Cannot remove the document root".into(),
                ));
            }
            let node = old.ast.node(node).ok_or(DesignerError::UnknownNode(node))?;
            vec![TextPatch {
                span: node.span,
                replacement: String::new(),
            }]
        }
        Edit::MoveNode {
            node,
            parent,
            index,
        } => {
            if node == old.ast.root() {
                return Err(DesignerError::InvalidEdit(
                    "Cannot move the document root".into(),
                ));
            }
            if node == parent || old.ast.is_descendant_of(parent, node) {
                return Err(DesignerError::InvalidEdit(
                    "Moving this node would create an ancestor cycle".into(),
                ));
            }
            let node = element(&old.ast, node)?;
            let parent = element(&old.ast, parent)?;
            let children: Vec<_> = old
                .ast
                .element_children(parent.id)
                .into_iter()
                .filter(|id| *id != node.id)
                .collect();
            if index > children.len() {
                return Err(DesignerError::InvalidEdit(format!(
                    "Child index {index} exceeds {}",
                    children.len()
                )));
            }
            let current = old
                .ast
                .element_children(parent.id)
                .iter()
                .position(|id| *id == node.id);
            if node.parent == Some(parent.id) && current == Some(index) {
                return Ok(old);
            }
            let mut fragment = old.source[node.span.range()].to_owned();
            // Preserve the namespace environment when reparenting into a template
            // or subtree with different namespace declarations.
            let old_scope = old.ast.namespaces(node.id);
            let new_scope = old.ast.namespaces(parent.id);
            let mut declarations = String::new();
            for (prefix, uri) in old_scope {
                if prefix == "xml" || new_scope.get(&prefix) == Some(&uri) {
                    continue;
                }
                let attr_name = if prefix.is_empty() {
                    "xmlns".into()
                } else {
                    format!("xmlns:{prefix}")
                };
                if node.attribute(&attr_name).is_none() {
                    declarations
                        .push_str(&format!(" {attr_name}=\"{}\"", escape_attribute(&uri, '"')));
                }
            }
            if node.namespace_uri.is_none()
                && new_scope.get("").is_some_and(|uri| !uri.is_empty())
                && node.attribute("xmlns").is_none()
            {
                declarations.push_str(" xmlns=\"\"");
            }
            let declaration_offset =
                node.opening_span.end - node.span.start - if node.self_closing { 2 } else { 1 };
            fragment.insert_str(declaration_offset, &declarations);
            let (mut patches, insert_offset, fragment_offset) =
                insertion_patches(parent, &old.ast, &children, index, &fragment)?;
            patches.push(TextPatch {
                span: node.span,
                replacement: String::new(),
            });
            // Pins are represented in old-source insertion coordinates here and
            // adjusted below after both deletion and insertion are applied.
            let deletion_delta = if node.span.end <= insert_offset {
                node.span.len()
            } else {
                0
            };
            let new_start = insert_offset - deletion_delta + fragment_offset;
            for descendant in old
                .ast
                .nodes()
                .filter(|n| n.id == node.id || old.ast.is_descendant_of(n.id, node.id))
            {
                let relative = descendant.span.start - node.span.start;
                let adjustment = if relative >= declaration_offset {
                    declarations.len()
                } else {
                    0
                };
                extra_pins.push((new_start + relative + adjustment, descendant.id));
            }
            patches
        }
    };
    let (source, mut pins) = patched_source(&old.source, &old.ast, patches, &extra_pins)?;
    pins.extend(extra_pins);
    parse_snapshot(source, &old, next_id, &pins)
}

fn insertion_patches(
    parent: &Node,
    ast: &Ast,
    children: &[NodeId],
    index: usize,
    fragment: &str,
) -> Result<(Vec<TextPatch>, usize, usize), DesignerError> {
    if index > children.len() {
        return Err(DesignerError::InvalidEdit(format!(
            "Child index {index} exceeds {}",
            children.len()
        )));
    }
    if parent.self_closing {
        let start = parent.opening_span.end - 2;
        Ok((
            vec![TextPatch {
                span: SourceSpan::new(start, parent.opening_span.end),
                replacement: format!(">{fragment}</{}>", parent.name),
            }],
            start,
            1,
        ))
    } else {
        let offset = children
            .get(index)
            .and_then(|id| ast.node(*id))
            .map_or_else(|| parent.closing_span.unwrap().start, |n| n.span.start);
        Ok((
            vec![TextPatch {
                span: SourceSpan::new(offset, offset),
                replacement: fragment.to_owned(),
            }],
            offset,
            0,
        ))
    }
}

/// Apply nonoverlapping edits and map surviving node starts through their offset
/// deltas. Replaced nodes fall back to semantic reconciliation.
fn patched_source(
    source: &str,
    ast: &Ast,
    mut patches: Vec<TextPatch>,
    extra_pins: &[(usize, NodeId)],
) -> Result<(String, Vec<(usize, NodeId)>), DesignerError> {
    patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
    for (index, patch) in patches.iter().enumerate() {
        let span = patch.span;
        if span.start > span.end
            || span.end > source.len()
            || !source.is_char_boundary(span.start)
            || !source.is_char_boundary(span.end)
        {
            return Err(DesignerError::InvalidRange(span));
        }
        if index > 0 {
            let previous = patches[index - 1].span;
            if span.start < previous.end
                || (span.start == previous.start && span.is_empty() && previous.is_empty())
            {
                return Err(DesignerError::InvalidEdit(
                    "Source patches overlap or have ambiguous insertion order".into(),
                ));
            }
        }
    }
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    let mut prefix_deltas = Vec::with_capacity(patches.len() + 1);
    prefix_deltas.push(0i128);
    for patch in &patches {
        result.push_str(&source[cursor..patch.span.start]);
        result.push_str(&patch.replacement);
        cursor = patch.span.end;
        prefix_deltas.push(
            prefix_deltas.last().unwrap() + patch.replacement.len() as i128
                - patch.span.len() as i128,
        );
    }
    result.push_str(&source[cursor..]);
    let explicitly_pinned: BTreeSet<_> = extra_pins.iter().map(|(_, id)| *id).collect();
    let mut pins = Vec::new();
    for node in ast.nodes() {
        if explicitly_pinned.contains(&node.id) {
            continue;
        }
        let old_offset = node.span.start;
        // Validated nonoverlapping patches also have nondecreasing ends. The first patch
        // ending after this node is the only one that could replace its start. Include empty
        // insertions at the node boundary in the preceding prefix so surviving nodes move
        // after their inserted siblings. One search replaces two scans per node in a batch.
        let preceding = patches.partition_point(|patch| patch.span.end <= old_offset);
        if patches
            .get(preceding)
            .is_some_and(|patch| patch.span.start <= old_offset)
        {
            continue;
        }
        let offset = old_offset as i128 + prefix_deltas[preceding];
        if offset >= 0 {
            pins.push((offset as usize, node.id));
        }
    }
    Ok((result, pins))
}

fn validate_name(name: &str) -> Result<(), DesignerError> {
    // XML itself validates Unicode NCNames and namespace declarations. Reject
    // delimiters before constructing source to prevent attribute injection.
    if name.is_empty()
        || name
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '=' | '\'' | '"' | '<' | '>' | '/' | '&'))
    {
        return Err(DesignerError::InvalidEdit(format!(
            "Invalid qualified attribute name {name:?}"
        )));
    }
    Ok(())
}

fn escape_attribute(value: &str, quote: char) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' if quote == '"' => out.push_str("&quot;"),
            '\'' if quote == '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            _ => out.push(ch),
        }
    }
    out
}

fn trim_history(history: &mut Vec<HistoryEntry>, limit: usize) {
    if history.len() > limit {
        history.drain(..history.len() - limit);
    }
}

fn diff(
    before: &Snapshot,
    after: &Snapshot,
    label: String,
    previous_revision: u64,
    revision: u64,
) -> ChangeSet {
    let old_ids: BTreeSet<_> = before.ast.nodes().map(|n| n.id).collect();
    let new_ids: BTreeSet<_> = after.ast.nodes().map(|n| n.id).collect();
    let changed = new_ids
        .intersection(&old_ids)
        .copied()
        .filter(|id| {
            let a = before.ast.node(*id).unwrap();
            let b = after.ast.node(*id).unwrap();
            a.parent != b.parent
                || a.children != b.children
                || a.kind != b.kind
                || a.name != b.name
                || a.namespace_uri != b.namespace_uri
                || a.text != b.text
                || a.attributes
                    .iter()
                    .map(|a| (&a.name, &a.value))
                    .ne(b.attributes.iter().map(|a| (&a.name, &a.value)))
        })
        .collect();
    ChangeSet {
        label,
        previous_revision,
        revision,
        inserted: new_ids.difference(&old_ids).copied().collect(),
        removed: old_ids.difference(&new_ids).copied().collect(),
        changed,
        patches: minimal_patch(&before.source, &after.source)
            .into_iter()
            .collect(),
        valid: after.diagnostics.is_empty(),
    }
}

fn minimal_patch(before: &str, after: &str) -> Option<TextPatch> {
    if before == after {
        return None;
    }
    let mut prefix = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(prefix) || !after.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let mut suffix = before[prefix..]
        .bytes()
        .rev()
        .zip(after[prefix..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(before.len() - suffix)
        || !after.is_char_boundary(after.len() - suffix)
    {
        suffix -= 1;
    }
    Some(TextPatch {
        span: SourceSpan::new(prefix, before.len() - suffix),
        replacement: after[prefix..after.len() - suffix].to_owned(),
    })
}

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
