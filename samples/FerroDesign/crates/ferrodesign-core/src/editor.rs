use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

const RENDER_CHANGE_HISTORY: usize = 128;
const RENDER_CHANGE_NODE_LIMIT: usize = 4096;
const RENDER_CHANGE_RETAINED_IDS: usize = 16384;

/// An opaque position in one editor's complete render-change history. Holding
/// the identity allocation prevents a replaced editor from reusing its token.
#[derive(Clone, Debug)]
pub struct EditorRenderToken {
    identity: Arc<()>,
    sequence: u64,
}

impl PartialEq for EditorRenderToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.identity, &other.identity) && self.sequence == other.sequence
    }
}
impl Eq for EditorRenderToken {}

/// Opaque ownership of one pending transaction. Retaining its allocation keeps
/// cancellation safe even if a whole Editor is replaced at the same revision.
/// This identity is transient and does not enter documents or history entries.
#[derive(Clone, Debug)]
pub struct EditorTransactionToken {
    identity: Arc<()>,
}
impl PartialEq for EditorTransactionToken {
    fn eq(&self, other: &Self) -> bool { Arc::ptr_eq(&self.identity, &other.identity) }
}
impl Eq for EditorTransactionToken {}

/// Complete render invalidation between two editor revisions. `nodes` includes
/// edited nodes and their descendants; `ancestors` carries derived bounds and
/// group/effect dependencies. IDs can occur on pages outside the active page.
/// Full refreshes and incomplete/oversized history always return `Full`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditorRenderChanges {
    Full,
    Nodes { nodes: Vec<NodeId>, ancestors: Vec<NodeId> },
}

impl EditorRenderChanges {
    fn len(&self) -> usize {
        match self {
            Self::Full => 0,
            Self::Nodes { nodes, ancestors } => nodes.len() + ancestors.len(),
        }
    }
}

struct RenderChangeLog {
    identity: Arc<()>,
    sequence: u64,
    published_revision: u64,
    entries: VecDeque<(u64, EditorRenderChanges)>,
    retained_ids: usize,
}

impl Default for RenderChangeLog {
    fn default() -> Self {
        Self {
            identity: Arc::new(()), sequence: 0, published_revision: 0,
            entries: VecDeque::new(), retained_ids: 0,
        }
    }
}

impl RenderChangeLog {
    fn append(&mut self, revision: u64, changes: EditorRenderChanges) {
        if self.sequence == u64::MAX {
            *self = Self::default();
        }
        self.sequence += 1;
        self.published_revision = revision;
        self.retained_ids += changes.len();
        self.entries.push_back((self.sequence, changes));
        while self.entries.len() > RENDER_CHANGE_HISTORY
            || self.retained_ids > RENDER_CHANGE_RETAINED_IDS
        {
            if let Some((_, removed)) = self.entries.pop_front() {
                self.retained_ids -= removed.len();
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    HorizontalCenter,
    Right,
    Top,
    VerticalCenter,
    Bottom,
}
pub type Alignment = Align;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distribution {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tool {
    #[default]
    Select,
    Frame,
    Rectangle,
    Ellipse,
    Line,
    Arrow,
    Polygon,
    Star,
    Pen,
    Pencil,
    Text,
    Hand,
    Comment,
    Slice,
    Section,
}
#[derive(Clone, Debug, PartialEq)]
struct Metadata {
    name: String,
    pages: Vec<Page>,
    styles: BTreeMap<StyleId, Style>,
    variables: BTreeMap<VariableId, Variable>,
    variable_mode: String,
    comments: Vec<CommentThread>,
    guides: BTreeMap<NodeId, Vec<Guide>>,
    next_id: u64,
}
impl Metadata {
    fn capture(d: &Document) -> Self {
        Self {
            name: d.name.clone(),
            pages: d.pages.clone(),
            styles: d.styles.clone(),
            variables: d.variables.clone(),
            variable_mode: d.variable_mode.clone(),
            comments: d.comments.clone(),
            guides: d.guides.clone(),
            next_id: d.next_id,
        }
    }
    fn restore(&self, d: &mut Document) {
        d.name = self.name.clone();
        d.pages = self.pages.clone();
        d.styles = self.styles.clone();
        d.variables = self.variables.clone();
        d.variable_mode = self.variable_mode.clone();
        d.comments = self.comments.clone();
        d.guides = self.guides.clone();
        d.next_id = self.next_id
    }
}
#[derive(Clone, Debug)]
struct HistoryEntry {
    label: String,
    nodes: BTreeMap<NodeId, (Option<Node>, Option<Node>)>,
    before: Metadata,
    after: Metadata,
    selection_before: Vec<NodeId>,
    selection_after: Vec<NodeId>,
    page_before: NodeId,
    page_after: NodeId,
    node_only: bool,
}
#[derive(Clone, Debug)]
enum PendingSnapshot {
    Document(Document),
    Nodes {
        nodes: BTreeMap<NodeId, Node>,
        metadata: Metadata,
    },
}
#[derive(Clone, Debug)]
struct PendingTransaction {
    token: EditorTransactionToken,
    label: String,
    before: PendingSnapshot,
    selection: Vec<NodeId>,
    page: NodeId,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clipboard {
    pub roots: Vec<NodeId>,
    pub nodes: BTreeMap<NodeId, Node>,
    pub bounds: Option<Rect>,
}
/// Committed history is a node delta. A gesture captures once, updates at pointer
/// frequency, and commits a single undo step with stable selection identities.
pub struct Editor {
    pub document: Document,
    pub selection: Vec<NodeId>,
    pub active_page: NodeId,
    pub scene: SceneIndex,
    pub revision: u64,
    pub viewport: Viewport,
    pub tool: Tool,
    pub clipboard: Option<Clipboard>,
    pub grid_size: f64,
    pub snap_to_grid: bool,
    pub snap_to_objects: bool,
    undo: Vec<HistoryEntry>,
    redo: Vec<HistoryEntry>,
    pending: Option<PendingTransaction>,
    previous_nodes: BTreeMap<NodeId, Node>,
    previous_page: NodeId,
    previous_roots: Vec<NodeId>,
    layout_node_count: usize,
    history_limit: usize,
    paste_count: u32,
    render_changes: RenderChangeLog,
}
impl Default for Editor {
    fn default() -> Self {
        Self::new(Document::default())
    }
}
impl Editor {
    pub fn new(document: Document) -> Self {
        let active_page = document.pages.first().map(|p| p.id).unwrap_or(1);
        let mut e = Self {
            document,
            selection: Vec::new(),
            active_page,
            scene: SceneIndex::default(),
            revision: 0,
            viewport: Viewport::default(),
            tool: Tool::Select,
            clipboard: None,
            grid_size: 8.0,
            snap_to_grid: false,
            snap_to_objects: true,
            undo: Vec::new(),
            redo: Vec::new(),
            pending: None,
            previous_nodes: BTreeMap::new(),
            previous_page: 0,
            previous_roots: Vec::new(),
            layout_node_count: 0,
            history_limit: 100,
            paste_count: 0,
            render_changes: RenderChangeLog::default(),
        };
        e.refresh();
        e
    }
    pub fn demo() -> Self {
        Self::new(seeded_document())
    }
    pub fn render_change_token(&self) -> EditorRenderToken {
        EditorRenderToken {
            identity: self.render_changes.identity.clone(),
            sequence: self.render_changes.sequence,
        }
    }
    /// Return every dependency changed since `token`, or request a full scene
    /// preparation. Consumers never need to assert that their own dirty list is
    /// complete. As with the spatial index, arbitrary public document writes
    /// must be followed by `refresh`; scoped previews report their existing IDs
    /// through `refresh_preview_nodes`.
    pub fn render_changes_since(&self, token: &EditorRenderToken) -> EditorRenderChanges {
        let log = &self.render_changes;
        if !Arc::ptr_eq(&token.identity, &log.identity)
            || token.sequence > log.sequence
            || self.document.revision != log.published_revision
            || self.revision != log.published_revision
        {
            return EditorRenderChanges::Full;
        }
        let mut sequence = token.sequence;
        let mut nodes = BTreeSet::new();
        let mut ancestors = BTreeSet::new();
        for (next, changes) in &log.entries {
            if *next <= token.sequence { continue; }
            if sequence.checked_add(1) != Some(*next) {
                return EditorRenderChanges::Full;
            }
            let EditorRenderChanges::Nodes { nodes: changed, ancestors: parents } = changes else {
                return EditorRenderChanges::Full;
            };
            nodes.extend(changed.iter().copied());
            ancestors.extend(parents.iter().copied());
            if nodes.len() + ancestors.len() > RENDER_CHANGE_NODE_LIMIT {
                return EditorRenderChanges::Full;
            }
            sequence = *next;
        }
        if sequence != log.sequence { return EditorRenderChanges::Full; }
        ancestors.retain(|id| !nodes.contains(id));
        EditorRenderChanges::Nodes {
            nodes: nodes.into_iter().collect(), ancestors: ancestors.into_iter().collect(),
        }
    }

    fn record_scoped_render_changes(&mut self, changed: &[NodeId]) {
        if changed.len() > RENDER_CHANGE_NODE_LIMIT {
            self.render_changes.append(self.revision, EditorRenderChanges::Full);
            return;
        }
        let mut nodes = BTreeSet::new();
        let mut ancestors = BTreeSet::new();
        let mut stack = changed.to_vec();
        while let Some(id) = stack.pop() {
            if !nodes.insert(id) { continue; }
            let Some(node) = self.document.nodes.get(&id) else {
                self.render_changes.append(self.revision, EditorRenderChanges::Full);
                return;
            };
            if nodes.len() + ancestors.len() > RENDER_CHANGE_NODE_LIMIT
                || stack.len().saturating_add(node.children.len()) > RENDER_CHANGE_NODE_LIMIT
            {
                self.render_changes.append(self.revision, EditorRenderChanges::Full);
                return;
            }
            stack.extend(node.children.iter().copied());
        }
        for &id in &nodes {
            let mut parent = self.document.nodes.get(&id).and_then(|node| node.parent);
            let mut seen = BTreeSet::new();
            while let Some(id) = parent {
                if self.document.page(id).is_some() { break; }
                if !seen.insert(id) || seen.len() > MAX_DOCUMENT_DEPTH {
                    self.render_changes.append(self.revision, EditorRenderChanges::Full);
                    return;
                }
                if !nodes.contains(&id) { ancestors.insert(id); }
                if nodes.len() + ancestors.len() > RENDER_CHANGE_NODE_LIMIT {
                    self.render_changes.append(self.revision, EditorRenderChanges::Full);
                    return;
                }
                let Some(node) = self.document.nodes.get(&id) else {
                    self.render_changes.append(self.revision, EditorRenderChanges::Full);
                    return;
                };
                parent = node.parent;
            }
        }
        self.render_changes.append(self.revision, EditorRenderChanges::Nodes {
            nodes: nodes.into_iter().collect(), ancestors: ancestors.into_iter().collect(),
        });
    }
    pub fn refresh(&mut self) {
        // Full refresh permits arbitrary direct document writes. If a scoped
        // gesture was open, the last refreshed nodes plus its original captures
        // reconstruct the complete state before those writes.
        self.promote_node_transaction(true);
        self.selection
            .retain(|id| self.document.nodes.contains_key(id));
        if self.document.page(self.active_page).is_none() {
            self.active_page = self.document.pages.first().map(|p| p.id).unwrap_or(1)
        }
        let roots = self
            .document
            .page(self.active_page)
            .map(|p| p.children.as_slice())
            .unwrap_or_default();
        let mut structural_change = self.previous_page != self.active_page
            || self.previous_nodes.len() != self.document.nodes.len()
            || self.previous_roots != roots;
        let mut geometry_changed = Vec::new();
        self.layout_node_count = 0;
        self.revision = self.revision.wrapping_add(1);
        self.document.revision = self.revision;
        for (id, n) in &mut self.document.nodes {
            self.layout_node_count += usize::from(n.layout.is_some());
            let content_changed = self
                .previous_nodes
                .get(id)
                .map(|old| {
                    structural_change |= n.parent != old.parent
                        || n.children != old.children
                        || n.visible != old.visible
                        || n.kind != old.kind
                        || n.clip_content != old.clip_content;
                    if n.x != old.x
                        || n.y != old.y
                        || n.width != old.width
                        || n.height != old.height
                        || n.rotation != old.rotation
                        || n.flip_x != old.flip_x
                        || n.flip_y != old.flip_y
                        || n.affine_transform != old.affine_transform
                        || n.effects != old.effects
                        || n.strokes != old.strokes
                    {
                        geometry_changed.push(*id);
                    }
                    let revision = n.revision;
                    n.revision = old.revision;
                    let changed = *n != *old;
                    n.revision = revision;
                    changed
                })
                .unwrap_or(true);
            if content_changed {
                n.revision = self.revision;
                self.previous_nodes.insert(*id, n.clone());
            }
        }
        if self.previous_nodes.len() != self.document.nodes.len() {
            self.previous_nodes
                .retain(|id, _| self.document.nodes.contains_key(id));
        }
        if structural_change {
            self.scene.rebuild(&self.document, self.active_page);
            self.previous_page = self.active_page;
            self.previous_roots = self
                .document
                .page(self.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default();
        } else if !geometry_changed.is_empty() {
            self.scene
                .update_nodes(&self.document, self.active_page, &geometry_changed);
        }
        self.render_changes.append(self.revision, EditorRenderChanges::Full);
    }
    /// Re-index transient edits to a known set of existing nodes without adding
    /// history. Call `refresh` instead when changing pages, document metadata,
    /// node membership, or nodes outside `changed`.
    pub fn refresh_preview_nodes(&mut self, changed: &[NodeId]) {
        self.capture_pending_nodes(changed, true);
        self.refresh_nodes(changed);
    }
    /// Whether any document node uses auto layout. The count follows refreshed
    /// node edits, imports and history, so querying it does not scan the scene.
    /// Callers with arbitrary direct document mutations must first `refresh`.
    pub fn has_auto_layout(&self) -> bool {
        self.layout_node_count != 0
    }
    /// Refresh a known set of node edits without comparing unrelated text,
    /// paths, images, or paint stacks. Arbitrary mutations through `document`
    /// still use `refresh`, and generic transactions retain that full scan.
    fn refresh_nodes(&mut self, changed: &[NodeId]) {
        if self.previous_page != self.active_page
            || self.previous_nodes.len() != self.document.nodes.len()
        {
            self.refresh();
            return;
        }
        self.selection
            .retain(|id| self.document.nodes.contains_key(id));
        self.revision = self.revision.wrapping_add(1);
        self.document.revision = self.revision;
        let mut structural_change = false;
        let mut geometry_changed = Vec::new();
        let mut content_changed_ids = Vec::new();
        for id in changed {
            let Some(node) = self.document.nodes.get_mut(id) else {
                continue;
            };
            let had_layout = self.previous_nodes.get(id).is_some_and(|old| old.layout.is_some());
            if node.layout.is_some() != had_layout {
                if had_layout {
                    self.layout_node_count = self.layout_node_count.saturating_sub(1);
                } else {
                    self.layout_node_count += 1;
                }
            }
            let content_changed = if let Some(old) = self.previous_nodes.get(id) {
                // Ordinary leaf conversions retain traversal membership and
                // sibling order. Their new geometry and ancestors can refresh
                // in place; container, layout, component and mask roles keep
                // the conservative structural path even without children.
                let stable_leaf_role = !node.kind.is_container() && !old.kind.is_container()
                    && node.children.is_empty() && old.children.is_empty()
                    && node.layout.is_none() && old.layout.is_none()
                    && node.component_id.is_none() && old.component_id.is_none()
                    && node.component_source.is_none() && old.component_source.is_none()
                    && !node.is_mask && !old.is_mask;
                structural_change |= node.parent != old.parent
                    || node.children != old.children
                    || node.visible != old.visible
                    || (node.opacity > 0.0) != (old.opacity > 0.0)
                    || (node.kind != old.kind && !stable_leaf_role)
                    || node.clip_content != old.clip_content;
                if node.kind != old.kind
                    || node.x != old.x
                    || node.y != old.y
                    || node.width != old.width
                    || node.height != old.height
                    || node.rotation != old.rotation
                    || node.flip_x != old.flip_x
                    || node.flip_y != old.flip_y
                    || node.affine_transform != old.affine_transform
                    || node.effects != old.effects
                    || node.strokes != old.strokes
                {
                    geometry_changed.push(*id);
                }
                let revision = node.revision;
                node.revision = old.revision;
                let differs = *node != *old;
                node.revision = revision;
                differs
            } else {
                structural_change = true;
                true
            };
            if content_changed {
                content_changed_ids.push(*id);
                node.revision = self.revision;
                self.previous_nodes.insert(*id, node.clone());
            }
        }
        if structural_change {
            self.scene.rebuild(&self.document, self.active_page);
        } else if !geometry_changed.is_empty() {
            self.scene
                .update_nodes(&self.document, self.active_page, &geometry_changed);
        }
        if structural_change {
            self.render_changes.append(self.revision, EditorRenderChanges::Full);
        } else {
            self.record_scoped_render_changes(&content_changed_ids);
        }
    }
    pub fn begin_transaction(&mut self, label: impl Into<String>) {
        if self.pending.is_none() {
            self.pending = Some(PendingTransaction {
                token: EditorTransactionToken { identity: Arc::new(()) },
                label: label.into(),
                before: PendingSnapshot::Document(self.document.clone()),
                selection: self.selection.clone(),
                page: self.active_page,
            })
        } else {
            self.promote_node_transaction(false);
        }
    }
    /// Capture a gesture that directly edits only known existing nodes. Preview
    /// mutations must report their IDs through `refresh_preview_nodes`; generic
    /// edits and full refreshes promote the snapshot before proceeding. Layout
    /// can change ancestors and siblings, so any auto-layout document keeps a
    /// complete snapshot. Creation, hierarchy changes, and resize constraints
    /// should use `begin_transaction`.
    pub fn begin_node_transaction(&mut self, ids: &[NodeId], label: impl Into<String>) {
        if self.layout_node_count != 0
            || ids.iter().any(|id| !self.document.nodes.contains_key(id))
        {
            self.begin_transaction(label);
            return;
        }
        if self.pending.is_some() {
            self.capture_pending_nodes(ids, false);
            return;
        }
        self.pending = Some(PendingTransaction {
            token: EditorTransactionToken { identity: Arc::new(()) },
            label: label.into(),
            before: PendingSnapshot::Nodes {
                nodes: ids.iter().filter_map(|id| {
                    self.document.nodes.get(id).map(|node| (*id, node.clone()))
                }).collect(),
                metadata: Metadata::capture(&self.document),
            },
            selection: self.selection.clone(),
            page: self.active_page,
        });
    }
    /// Before a scoped API mutates a new node, capture its current value. A
    /// direct preview reports after mutation, so use the last refreshed value.
    fn capture_pending_nodes(&mut self, ids: &[NodeId], preview: bool) {
        let Some(PendingTransaction {
            before: PendingSnapshot::Nodes { nodes, .. }, ..
        }) = &mut self.pending else {
            return;
        };
        let source = if preview { &self.previous_nodes } else { &self.document.nodes };
        for id in ids {
            if !nodes.contains_key(id) {
                if let Some(node) = source.get(id) {
                    nodes.insert(*id, node.clone());
                }
            }
        }
    }
    fn promote_node_transaction(&mut self, unreported_changes: bool) {
        let Some(pending) = &mut self.pending else { return };
        let PendingSnapshot::Nodes { nodes, metadata } = &pending.before else { return };
        let mut before = self.document.clone();
        if unreported_changes {
            before.nodes = self.previous_nodes.clone();
        }
        for (id, node) in nodes {
            before.nodes.insert(*id, node.clone());
        }
        metadata.restore(&mut before);
        pending.before = PendingSnapshot::Document(before);
    }
    pub fn commit_transaction(&mut self) -> bool {
        if self.previous_nodes.len() != self.document.nodes.len() {
            self.promote_node_transaction(true);
        }
        let Some(p) = self.pending.take() else {
            return false;
        };
        let (mut before_nodes, before, scoped) = match p.before {
            PendingSnapshot::Document(document) => {
                let metadata = Metadata::capture(&document);
                (document.nodes, metadata, false)
            }
            PendingSnapshot::Nodes { nodes, metadata } => (nodes, metadata, true),
        };
        let keys: Vec<_> = if scoped {
            before_nodes.keys().copied().collect()
        } else {
            before_nodes.keys().chain(self.document.nodes.keys())
                .copied().collect::<BTreeSet<_>>().into_iter().collect()
        };
        let mut nodes = BTreeMap::new();
        for id in keys.iter().copied() {
            let mut old = before_nodes.remove(&id);
            let after = self.document.nodes.get(&id);
            if let (Some(a), Some(b)) = (old.as_mut(), after) {
                a.revision = b.revision;
            }
            if old.as_ref() != after {
                nodes.insert(id, (old, after.cloned()));
            }
        }
        let after = Metadata::capture(&self.document);
        let node_only = scoped && before == after;
        if nodes.is_empty() && before == after {
            if node_only {
                self.refresh_nodes(&keys);
            } else {
                self.refresh();
            }
            return false;
        }
        self.undo.push(HistoryEntry {
            label: p.label,
            nodes,
            before,
            after,
            selection_before: p.selection,
            selection_after: self.selection.clone(),
            page_before: p.page,
            page_after: self.active_page,
            node_only,
        });
        if self.undo.len() > self.history_limit {
            self.undo.remove(0);
        }
        self.redo.clear();
        if node_only {
            self.refresh_nodes(&keys);
        } else {
            self.refresh();
        }
        true
    }
    pub fn cancel_transaction(&mut self) {
        if self.previous_nodes.len() != self.document.nodes.len() {
            self.promote_node_transaction(true);
        }
        if let Some(p) = self.pending.take() {
            self.selection = p.selection;
            self.active_page = p.page;
            match p.before {
                PendingSnapshot::Document(before) => {
                    self.document = before;
                    self.refresh();
                }
                PendingSnapshot::Nodes { nodes, metadata } => {
                    let changed: Vec<_> = nodes.keys().copied().collect();
                    self.document.nodes.extend(nodes);
                    metadata.restore(&mut self.document);
                    self.refresh_nodes(&changed);
                }
            }
        }
    }
    pub fn transact<R>(
        &mut self,
        label: impl Into<String>,
        operation: impl FnOnce(&mut Document) -> R,
    ) -> R {
        let own = self.pending.is_none();
        if own {
            self.begin_transaction(label)
        } else {
            self.promote_node_transaction(false);
        }
        let r = operation(&mut self.document);
        if own {
            self.commit_transaction();
        } else {
            self.refresh()
        }
        r
    }
    pub(crate) fn edit<R>(&mut self, label: &str, operation: impl FnOnce(&mut Self) -> R) -> R {
        let own = self.pending.is_none();
        if own {
            self.begin_transaction(label)
        } else {
            self.promote_node_transaction(false);
        }
        let r = operation(self);
        if own {
            self.commit_transaction();
        } else {
            self.refresh()
        }
        r
    }
    /// Internal operations using this path may change only the supplied nodes.
    /// Open scoped gestures capture additional nodes before mutation; open
    /// general transactions retain their original complete snapshot.
    pub(crate) fn edit_nodes<R>(
        &mut self,
        label: &str,
        ids: &[NodeId],
        operation: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if self.pending.is_some() {
            self.capture_pending_nodes(ids, false);
            let result = operation(self);
            if matches!(self.pending.as_ref().map(|pending| &pending.before),
                Some(PendingSnapshot::Nodes { .. })) {
                self.refresh_nodes(ids);
            } else {
                self.refresh();
            }
            return result;
        }
        let before: BTreeMap<_, _> = ids
            .iter()
            .map(|id| (*id, self.document.nodes.get(id).cloned()))
            .collect();
        let metadata = Metadata::capture(&self.document);
        let selection_before = self.selection.clone();
        let page_before = self.active_page;
        let result = operation(self);
        let mut nodes = BTreeMap::new();
        for (id, mut old) in before {
            let new = self.document.nodes.get(&id);
            if let (Some(old), Some(new)) = (old.as_mut(), new) {
                old.revision = new.revision;
            }
            if old.as_ref() != new {
                nodes.insert(id, (old, new.cloned()));
            }
        }
        if !nodes.is_empty() {
            self.undo.push(HistoryEntry {
                label: label.into(),
                nodes,
                before: metadata.clone(),
                after: metadata,
                selection_before,
                selection_after: self.selection.clone(),
                page_before,
                page_after: self.active_page,
                node_only: true,
            });
            if self.undo.len() > self.history_limit {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.refresh_nodes(ids);
        result
    }
    /// Whether a live edit currently owns the next history transaction.
    pub fn has_active_transaction(&self) -> bool {
        self.pending.is_some()
    }
    pub fn active_transaction_token(&self) -> Option<EditorTransactionToken> {
        self.pending.as_ref().map(|pending| pending.token.clone())
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label.as_str())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label.as_str())
    }
    pub fn history_len(&self) -> usize {
        self.undo.len()
    }
    pub fn undo(&mut self) -> bool {
        if self.pending.is_some() {
            self.cancel_transaction();
            return true;
        }
        let Some(e) = self.undo.pop() else {
            return false;
        };
        for (id, (before, _)) in &e.nodes {
            if let Some(n) = before {
                self.document.nodes.insert(*id, n.clone());
            } else {
                self.document.nodes.remove(id);
            }
        }
        e.before.restore(&mut self.document);
        self.selection = e.selection_before.clone();
        self.active_page = e.page_before;
        let changed: Vec<_> = e.nodes.keys().copied().collect();
        let node_only = e.node_only;
        self.redo.push(e);
        if node_only {
            self.refresh_nodes(&changed);
        } else {
            self.refresh();
        }
        true
    }
    pub fn redo(&mut self) -> bool {
        if self.pending.is_some() {
            self.commit_transaction();
        }
        let Some(e) = self.redo.pop() else {
            return false;
        };
        for (id, (_, after)) in &e.nodes {
            if let Some(n) = after {
                self.document.nodes.insert(*id, n.clone());
            } else {
                self.document.nodes.remove(id);
            }
        }
        e.after.restore(&mut self.document);
        self.selection = e.selection_after.clone();
        self.active_page = e.page_after;
        let changed: Vec<_> = e.nodes.keys().copied().collect();
        let node_only = e.node_only;
        self.undo.push(e);
        if node_only {
            self.refresh_nodes(&changed);
        } else {
            self.refresh();
        }
        true
    }
    pub fn clear_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.pending = None
    }
    pub fn set_history_limit(&mut self, n: usize) {
        self.history_limit = n.max(1);
        let excess = self.undo.len().saturating_sub(self.history_limit);
        self.undo.drain(..excess);
    }
    pub fn select(&mut self, id: NodeId, additive: bool) {
        if !self.document.nodes.contains_key(&id) {
            return;
        }
        if !additive {
            self.selection.clear()
        }
        if additive && self.selection.contains(&id) {
            self.selection.retain(|n| *n != id)
        } else if !self.selection.contains(&id) {
            self.selection.push(id)
        }
    }
    pub fn clear_selection(&mut self) {
        self.selection.clear()
    }
    pub fn select_all(&mut self) {
        self.selection = self
            .document
            .page(self.active_page)
            .map(|p| {
                p.children
                    .iter()
                    .filter(|id| {
                        !self.document.effective_locked(**id)
                            && self.document.effective_visible(**id)
                    })
                    .copied()
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn select_rect(&mut self, r: Rect, additive: bool) {
        if !additive {
            self.selection.clear()
        }
        let mut ids: Vec<_> = self
            .scene
            .spatial
            .query(r)
            .into_iter()
            .filter(|id| {
                !self.document.effective_locked(*id)
                    && self
                        .document
                        .world_bounds(*id)
                        .is_some_and(|b| r.contains_rect(b))
            })
            .collect();
        let set: BTreeSet<_> = ids.iter().copied().collect();
        ids.retain(|id| !has_selected_ancestor(&self.document, *id, &set));
        for id in ids {
            if !self.selection.contains(&id) {
                self.selection.push(id)
            }
        }
    }
    pub fn hit_test(&self, p: Point, tolerance: f64) -> Option<NodeId> {
        self.scene.hit_test(&self.document, p, tolerance, false)
    }
    /// Presentation hit testing filters the active frame/overlay before
    /// choosing the topmost shape. Locked authored layers remain interactive.
    pub fn prototype_hit_test(&self, root: NodeId, p: Point, tolerance: f64) -> Option<NodeId> {
        self.scene.hit_test_in_subtree(&self.document, root, p, tolerance, true)
    }
    pub fn visible_nodes(&self, r: Rect) -> Vec<NodeId> {
        self.scene.visible_nodes(r)
    }
    pub fn selection_bounds(&self) -> Option<Rect> {
        self.selection
            .iter()
            .filter_map(|id| self.document.world_bounds(*id))
            .reduce(|a, b| a.union(b))
    }
    pub fn root_selection(&self) -> Vec<NodeId> {
        let selected: BTreeSet<_> = self.selection.iter().copied().collect();
        self.selection
            .iter()
            .filter(|id| {
                !self.document.effective_locked(**id)
                    && !has_selected_ancestor(&self.document, **id, &selected)
            })
            .copied()
            .collect()
    }
    pub fn set_active_page(&mut self, page: NodeId) -> bool {
        if self.document.page(page).is_none() {
            return false;
        }
        self.active_page = page;
        self.selection.clear();
        self.refresh();
        true
    }
    pub fn add_page(&mut self, name: impl Into<String>) -> NodeId {
        let name = name.into();
        self.edit("Add page", |e| {
            let id = e.document.add_page(name);
            e.active_page = id;
            e.selection.clear();
            id
        })
    }
    pub fn add_node(&mut self, kind: NodeKind, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.edit("Create layer", |e| {
            let id = e
                .document
                .insert_node(e.active_page, Node::new(kind, kind.label(), x, y, w, h))
                .expect("active page exists");
            e.selection = vec![id];
            id
        })
    }
    pub fn insert_node(&mut self, parent: NodeId, node: Node) -> Result<NodeId, DocumentError> {
        self.edit("Create layer", |e| {
            let id = e.document.insert_node(parent, node)?;
            e.selection = vec![id];
            Ok(id)
        })
    }
    pub fn update_node<R>(
        &mut self,
        id: NodeId,
        label: &str,
        update: impl FnOnce(&mut Node) -> R,
    ) -> Option<R> {
        self.edit_nodes(label, &[id], |e| {
            let node = e.document.nodes.get_mut(&id)?;
            let bindings = crate::style_edit::StyleBindingSnapshot::capture(node, e.previous_nodes.get(&id));
            let result = update(node);
            bindings.detach_changed(node);
            Some(result)
        })
    }
    /// Update each existing node once and retain one undo record. This scoped
    /// callback cannot mutate document metadata or unrelated nodes, so small
    /// inspector edits do not copy and compare the whole document.
    pub fn update_nodes(
        &mut self,
        ids: &[NodeId],
        label: &str,
        mut update: impl FnMut(&mut Node),
    ) -> usize {
        let ids: Vec<_> = ids.iter().copied().collect::<BTreeSet<_>>().into_iter().collect();
        self.edit_nodes(label, &ids, |editor| {
            let mut count = 0;
            for id in &ids {
                if let Some(node) = editor.document.nodes.get_mut(id) {
                    let bindings = crate::style_edit::StyleBindingSnapshot::capture(node, editor.previous_nodes.get(id));
                    update(node);
                    bindings.detach_changed(node);
                    count += 1;
                }
            }
            count
        })
    }
    /// Apply inspector changes once per existing node, then preserve normal
    /// document-wide auto layout when any layout exists or the edit introduces
    /// one. The cached layout count avoids scanning the document just to choose
    /// the scoped path; edited values are prepared before capturing history.
    pub fn update_nodes_with_layout(
        &mut self,
        ids: &[NodeId],
        label: &str,
        mut update: impl FnMut(&mut Node),
    ) -> usize {
        let edits: Vec<_> = ids.iter().copied().collect::<BTreeSet<_>>().into_iter()
            .filter_map(|id| self.document.nodes.get(&id).map(|before| {
                let mut node = before.clone();
                update(&mut node);
                crate::style_edit::detach_changed_style_bindings(before, &mut node);
                (id, node)
            })).collect();
        let count = edits.len();
        if self.layout_node_count != 0 || edits.iter().any(|(_, node)| node.layout.is_some()) {
            self.transact(label, |document| {
                document.nodes.extend(edits);
                document.layout_all();
            });
        } else {
            let ids: Vec<_> = edits.iter().map(|(id, _)| *id).collect();
            self.edit_nodes(label, &ids, |editor| editor.document.nodes.extend(edits));
        }
        count
    }
    pub fn move_selection(&mut self, dx: f64, dy: f64) {
        if !dx.is_finite() || !dy.is_finite() {
            return;
        }
        let ids = self.root_selection();
        self.edit_nodes("Move layers", &ids, |e| {
            for id in &ids {
                e.move_world(*id, Point::new(dx, dy))
            }
        })
    }
    fn move_world(&mut self, id: NodeId, delta: Point) {
        let p = self.document.nodes.get(&id).and_then(|n| n.parent);
        let delta = p
            .and_then(|id| self.document.world_transform(id))
            .and_then(|t| t.inverse())
            .map(|t| t.apply_vector(delta))
            .unwrap_or(delta);
        if let Some(n) = self.document.nodes.get_mut(&id) {
            n.x += delta.x;
            n.y += delta.y
        }
    }
    pub fn resize_node(&mut self, id: NodeId, w: f64, h: f64) {
        self.resize_node_impl(id, w, h, None)
    }
    /// Resize and assign the final position in the parent's coordinate space.
    /// Both changes share one edit, refresh, and undo transaction.
    pub fn resize_node_and_position(
        &mut self,
        id: NodeId,
        w: f64,
        h: f64,
        position: Point,
    ) {
        self.resize_node_impl(id, w, h, Some(position))
    }
    fn resize_node_impl(
        &mut self,
        id: NodeId,
        w: f64,
        h: f64,
        position: Option<Point>,
    ) {
        if !w.is_finite()
            || !h.is_finite()
            || position.is_some_and(|p| !p.x.is_finite() || !p.y.is_finite())
        {
            return;
        }
        self.edit("Resize layer", |e| {
            if let Some(n) = e.document.nodes.get(&id) {
                let (ow, oh) = (n.width, n.height);
                let original_transform = n.local_transform();
                if !e.document.nodes.get_mut(&id).unwrap().resize_vector_geometry(w.max(0.0), h.max(0.0)) {
                    return;
                }
                e.document
                    .apply_constraints(id, ow, oh, w.max(0.0), h.max(0.0));
                if let Some(n) = e.document.nodes.get_mut(&id) {
                    n.width = w.max(0.0);
                    n.height = h.max(0.0);
                    // Resizing preserves the transformed local origin, including
                    // imported affine layers; dimensions change in local space.
                    let resized_transform = n.local_transform();
                    n.x += original_transform.e - resized_transform.e;
                    n.y += original_transform.f - resized_transform.f;
                }
                e.document.layout_node(id);
                if let Some(position) = position {
                    if let Some(n) = e.document.nodes.get_mut(&id) {
                        n.x = position.x;
                        n.y = position.y;
                    }
                }
            }
        })
    }
    pub fn rotate_selection(&mut self, degrees: f64) {
        if !degrees.is_finite() {
            return;
        }
        let ids = self.root_selection();
        self.edit_nodes("Rotate layers", &ids, |e| {
            for id in &ids {
                if let Some(n) = e.document.nodes.get_mut(id) {
                    n.rotation = (n.rotation + degrees).rem_euclid(360.0)
                }
            }
        })
    }
    pub fn set_fill(&mut self, id: NodeId, c: Color) {
        self.update_node(id, "Change fill", |n| {
            n.fills = vec![Fill::solid(c)];
            n.overridden_properties.insert("fills".into());
        });
    }
    pub fn set_selection_fill(&mut self, c: Color) {
        self.update_nodes(&self.selection.clone(), "Change fill", |node| {
            node.fills = vec![Fill::solid(c)];
            node.overridden_properties.insert("fills".into());
        });
    }
    pub fn set_text(&mut self, id: NodeId, text: impl Into<String>) {
        let text = text.into();
        self.update_node(id, "Edit text", |n| {
            n.text = text;
            n.text_runs.clear();
            n.overridden_properties.insert("text".into());
        });
    }
    pub fn toggle_visibility(&mut self, id: NodeId) {
        self.update_node(id, "Toggle visibility", |n| n.visible = !n.visible);
    }
    pub fn toggle_locked(&mut self, id: NodeId) {
        self.update_node(id, "Toggle lock", |n| n.locked = !n.locked);
    }
    pub fn delete_selection(&mut self) {
        self.edit("Delete layers", |e| {
            for id in e.root_selection() {
                e.document.remove_subtree(id);
            }
            e.selection.clear()
        })
    }
    pub fn copy_selection(&mut self) -> bool {
        let roots = self.root_selection();
        if roots.is_empty() {
            return false;
        }
        let mut nodes = BTreeMap::new();
        for id in &roots {
            for child in self.document.descendants(*id) {
                if let Some(n) = self.document.nodes.get(&child) {
                    nodes.insert(child, n.clone());
                }
            }
        }
        for id in &roots {
            if let Some(t) = self.document.world_transform(*id) {
                if let Some(n) = nodes.get_mut(id) {
                    assign_transform(n, t);
                    n.parent = None
                }
            }
        }
        self.clipboard = Some(Clipboard {
            roots,
            nodes,
            bounds: self.selection_bounds(),
        });
        self.paste_count = 0;
        true
    }
    pub fn cut_selection(&mut self) -> bool {
        if !self.copy_selection() {
            return false;
        }
        self.delete_selection();
        true
    }
    pub fn clipboard_json(&self) -> Option<String> {
        serde_json::to_string(self.clipboard.as_ref()?).ok()
    }
    pub fn paste(&mut self) -> Vec<NodeId> {
        let Some(c) = self.clipboard.clone() else {
            return Vec::new();
        };
        self.paste_count += 1;
        let d = 24.0 * self.paste_count as f64;
        self.paste_clipboard(&c, self.active_page, Point::new(d, d))
    }
    pub fn paste_at(&mut self, p: Point) -> Vec<NodeId> {
        let Some(c) = self.clipboard.clone() else {
            return Vec::new();
        };
        let origin = c.bounds.map(|b| Point::new(b.x, b.y)).unwrap_or_default();
        self.paste_clipboard(&c, self.active_page, p - origin)
    }
    pub fn paste_clipboard(&mut self, c: &Clipboard, parent: NodeId, offset: Point) -> Vec<NodeId> {
        self.edit("Paste layers", |e| {
            let roots = clone_tree_into(&mut e.document, &c.nodes, &c.roots, parent, offset, false);
            e.selection = roots.clone();
            roots
        })
    }
    /// Import editable SVG layers beside the existing page contents. Parsing and
    /// validation happen before the transaction; errors leave history, selection,
    /// and the document unchanged. The returned imported frame is selected.
    pub fn import_svg(&mut self, source: &str, name: &str) -> Result<Vec<NodeId>, DocumentError> {
        let imported = parse_svg(source, name)?;
        if self.document.nodes.len() + imported.document.nodes.len() > MAX_DOCUMENT_NODES {
            return Err(DocumentError::Invalid(
                "SVG import exceeds the document node limit".into(),
            ));
        }
        if self.document.page(self.active_page).is_none() {
            return Err(DocumentError::InvalidParent(self.active_page));
        }
        let right = self
            .document
            .page(self.active_page)
            .unwrap()
            .children
            .iter()
            .filter_map(|id| self.document.world_bounds(*id))
            .map(|b| b.right())
            .reduce(f64::max);
        let original = imported
            .document
            .world_bounds(imported.root)
            .unwrap_or_default();
        let offset = Point::new(
            right.map(|r| r + 64.0).unwrap_or(0.0) - original.x,
            -original.y,
        );
        Ok(self.edit("Import SVG", |editor| {
            let roots = clone_tree_into(
                &mut editor.document,
                &imported.document.nodes,
                &[imported.root],
                editor.active_page,
                offset,
                false,
            );
            editor.selection = roots.clone();
            roots
        }))
    }
    pub fn duplicate_selection(&mut self) -> Vec<NodeId> {
        self.edit("Duplicate layers", |e| {
            let roots = e.root_selection();
            let source = e.document.nodes.clone();
            let mut out = Vec::new();
            for id in roots {
                let Some(n) = source.get(&id) else { continue };
                let parent = n.parent.unwrap_or(e.active_page);
                let ids = clone_tree_into(
                    &mut e.document,
                    &source,
                    &[id],
                    parent,
                    Point::new(24.0, 24.0),
                    false,
                );
                if let Some(new) = ids.first() {
                    let siblings = e.document.children_mut(parent).unwrap();
                    siblings.retain(|i| i != new);
                    let index = siblings
                        .iter()
                        .position(|i| *i == id)
                        .map(|i| i + 1)
                        .unwrap_or(siblings.len());
                    siblings.insert(index, *new);
                }
                out.extend(ids)
            }
            e.selection = out.clone();
            out
        })
    }
    pub fn group_selection(&mut self) -> Option<NodeId> {
        self.edit("Group selection", |e| e.group_internal(NodeKind::Group))
    }
    pub fn frame_selection(&mut self) -> Option<NodeId> {
        self.edit("Frame selection", |e| e.group_internal(NodeKind::Frame))
    }
    fn group_internal(&mut self, kind: NodeKind) -> Option<NodeId> {
        let roots = self.root_selection();
        if roots.is_empty() {
            return None;
        }
        let first = self
            .document
            .nodes
            .get(&roots[0])?
            .parent
            .unwrap_or(self.active_page);
        let parent = if roots
            .iter()
            .all(|id| self.document.nodes.get(id).and_then(|n| n.parent) == Some(first))
        {
            first
        } else {
            self.active_page
        };
        let inv = self
            .document
            .world_transform(parent)
            .unwrap_or_default()
            .inverse()?;
        let bounds = roots
            .iter()
            .filter_map(|id| {
                let n = self.document.nodes.get(id)?;
                Some((inv * self.document.world_transform(*id)?).transform_rect(n.local_bounds()))
            })
            .reduce(|a, b| a.union(b))?;
        let transforms: Vec<_> = roots
            .iter()
            .filter_map(|id| Some((*id, self.document.world_transform(*id)?)))
            .collect();
        let index = self
            .document
            .children(parent)?
            .iter()
            .position(|id| roots.contains(id))
            .unwrap_or(self.document.children(parent)?.len());
        let mut n = Node::new(
            kind,
            kind.label(),
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
        );
        n.fills.clear();
        n.clip_content = false;
        let group = self.document.insert_node(parent, n).ok()?;
        {
            let siblings = self.document.children_mut(parent)?;
            siblings.retain(|id| *id != group);
            siblings.insert(index.min(siblings.len()), group)
        }
        let inverse = self.document.world_transform(group)?.inverse()?;
        for (id, world) in transforms {
            let _ = self.document.reparent(id, group, usize::MAX);
            if let Some(n) = self.document.nodes.get_mut(&id) {
                assign_transform(n, inverse * world)
            }
        }
        self.selection = vec![group];
        Some(group)
    }
    pub fn ungroup_selection(&mut self) -> Vec<NodeId> {
        self.edit("Ungroup selection", |e| {
            let mut out = Vec::new();
            for id in e.root_selection() {
                let Some(group) = e.document.nodes.get(&id).cloned() else {
                    continue;
                };
                if !matches!(
                    group.kind,
                    NodeKind::Group | NodeKind::Frame | NodeKind::Section
                ) {
                    out.push(id);
                    continue;
                }
                let parent = group.parent.unwrap_or(e.active_page);
                let Some(inv) = e
                    .document
                    .world_transform(parent)
                    .unwrap_or_default()
                    .inverse()
                else {
                    continue;
                };
                let index = e
                    .document
                    .children(parent)
                    .and_then(|c| c.iter().position(|n| *n == id))
                    .unwrap_or(0);
                for (i, child) in group.children.iter().enumerate() {
                    let Some(world) = e.document.world_transform(*child) else {
                        continue;
                    };
                    let _ = e.document.reparent(*child, parent, index + i);
                    if let Some(n) = e.document.nodes.get_mut(child) {
                        assign_transform(n, inv * world)
                    }
                    out.push(*child)
                }
                e.document.remove_subtree(id);
            }
            e.selection = out.clone();
            out
        })
    }
    pub fn align_selection(&mut self, a: Align) {
        self.edit("Align layers", |e| {
            let ids = e.root_selection();
            let Some(bounds) = ids
                .iter()
                .filter_map(|id| e.document.world_bounds(*id))
                .reduce(|a, b| a.union(b))
            else {
                return;
            };
            for id in ids {
                let Some(b) = e.document.world_bounds(id) else {
                    continue;
                };
                let delta = match a {
                    Align::Left => Point::new(bounds.x - b.x, 0.0),
                    Align::HorizontalCenter => Point::new(bounds.center().x - b.center().x, 0.0),
                    Align::Right => Point::new(bounds.right() - b.right(), 0.0),
                    Align::Top => Point::new(0.0, bounds.y - b.y),
                    Align::VerticalCenter => Point::new(0.0, bounds.center().y - b.center().y),
                    Align::Bottom => Point::new(0.0, bounds.bottom() - b.bottom()),
                };
                e.move_world(id, delta)
            }
        })
    }
    pub fn distribute_selection(&mut self, d: Distribution) {
        self.edit("Distribute layers", |e| {
            let mut items: Vec<_> = e
                .root_selection()
                .into_iter()
                .filter_map(|id| Some((id, e.document.world_bounds(id)?)))
                .collect();
            if items.len() < 3 {
                return;
            }
            let horizontal = d == Distribution::Horizontal;
            items.sort_by(|a, b| {
                (if horizontal { a.1.x } else { a.1.y })
                    .total_cmp(&(if horizontal { b.1.x } else { b.1.y }))
            });
            let first = items.first().unwrap().1;
            let last = items.last().unwrap().1;
            let start = if horizontal { first.x } else { first.y };
            let end = if horizontal {
                last.right()
            } else {
                last.bottom()
            };
            let sum: f64 = items
                .iter()
                .map(|(_, b)| if horizontal { b.width } else { b.height })
                .sum();
            let gap = (end - start - sum) / (items.len() - 1) as f64;
            let mut cursor = start;
            for (id, b) in items {
                let delta = cursor - if horizontal { b.x } else { b.y };
                e.move_world(
                    id,
                    if horizontal {
                        Point::new(delta, 0.0)
                    } else {
                        Point::new(0.0, delta)
                    },
                );
                cursor += if horizontal {
                    b.width + gap
                } else {
                    b.height + gap
                }
            }
        })
    }
    pub fn arrange_selection(&mut self, a: Arrange) {
        self.edit("Arrange layers", |e| {
            let selection: BTreeSet<_> = e.root_selection().into_iter().collect();
            let parents: BTreeSet<_> = selection
                .iter()
                .filter_map(|id| e.document.nodes.get(id).and_then(|n| n.parent))
                .collect();
            for parent in parents {
                let Some(c) = e.document.children_mut(parent) else {
                    continue;
                };
                match a {
                    Arrange::BringToFront => {
                        let moved: Vec<_> = c
                            .iter()
                            .filter(|id| selection.contains(id))
                            .copied()
                            .collect();
                        c.retain(|id| !selection.contains(id));
                        c.extend(moved)
                    }
                    Arrange::SendToBack => {
                        let mut moved: Vec<_> = c
                            .iter()
                            .filter(|id| selection.contains(id))
                            .copied()
                            .collect();
                        c.retain(|id| !selection.contains(id));
                        moved.append(c);
                        *c = moved
                    }
                    Arrange::BringForward => {
                        for i in (0..c.len().saturating_sub(1)).rev() {
                            if selection.contains(&c[i]) && !selection.contains(&c[i + 1]) {
                                c.swap(i, i + 1)
                            }
                        }
                    }
                    Arrange::SendBackward => {
                        for i in 1..c.len() {
                            if selection.contains(&c[i]) && !selection.contains(&c[i - 1]) {
                                c.swap(i, i - 1)
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn set_auto_layout(&mut self, layout: AutoLayout) -> Option<NodeId> {
        self.edit("Add auto layout", |e| {
            let id = if e.selection.len() == 1
                && e.document
                    .nodes
                    .get(&e.selection[0])
                    .is_some_and(|n| n.kind.is_container())
            {
                e.selection[0]
            } else {
                e.group_internal(NodeKind::Frame)?
            };
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.layout = Some(layout);
                if n.kind == NodeKind::Group {
                    n.kind = NodeKind::Frame
                }
            }
            e.document.layout_all();
            Some(id)
        })
    }
    pub fn remove_auto_layout(&mut self, id: NodeId) {
        self.update_node(id, "Remove auto layout", |n| n.layout = None);
    }
    pub fn create_component(&mut self) -> Option<NodeId> {
        self.edit("Create component", |e| {
            let id = if e.selection.len() == 1
                && e.document
                    .nodes
                    .get(&e.selection[0])
                    .is_some_and(|n| n.kind.is_container())
            {
                e.selection[0]
            } else {
                e.group_internal(NodeKind::Component)?
            };
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.kind = NodeKind::Component;
                n.component_id = None;
                n.component_source = None;
                n.component_property_values.clear();
                n.metadata.remove(crate::COMPONENT_SET_METADATA_KEY);
            }
            Some(id)
        })
    }
    pub fn create_instance(&mut self, component: NodeId, x: f64, y: f64) -> Option<NodeId> {
        self.edit("Create instance", |e| {
            let id = e.document.create_component_instance(component, e.active_page, x, y).ok()?;
            e.selection = vec![id];
            Some(id)
        })
    }
    pub fn detach_instance(&mut self, id: NodeId) -> bool {
        self.edit("Detach instance", |e| {
            if !e
                .document
                .nodes
                .get(&id)
                .is_some_and(|n| n.kind == NodeKind::Instance)
            {
                return false;
            }
            for child in e.document.descendants(id) {
                if let Some(n) = e.document.nodes.get_mut(&child) {
                    n.component_id = None;
                    n.component_source = None;
                    n.overridden_properties.clear();
                    n.component_property_definitions.clear();
                    n.component_property_values.clear();
                    if n.kind == NodeKind::Instance {
                        n.kind = NodeKind::Frame
                    }
                }
            }
            e.document.repair_component_references();
            true
        })
    }
    pub fn sync_instances(&mut self, component: NodeId) -> usize {
        self.edit("Update component instances", |e| {
            sync_component_instances(&mut e.document, component)
        })
    }
    pub fn add_interaction(
        &mut self,
        source: NodeId,
        target: NodeId,
        transition: Transition,
    ) -> bool {
        self.edit("Add prototype interaction", |e| {
            if !e.document.nodes.contains_key(&target) {
                return false;
            }
            let Some(n) = e.document.nodes.get_mut(&source) else {
                return false;
            };
            n.interactions.push(Interaction {
                trigger: Trigger::Click,
                action: PrototypeAction::NavigateTo { target },
                transition,
                delay_ms: 0,
                key: None,
            });
            true
        })
    }
    pub fn add_comment(
        &mut self,
        node: Option<NodeId>,
        position: Point,
        author: impl Into<String>,
        body: impl Into<String>,
        created_at: impl Into<String>,
    ) -> u64 {
        let author = author.into();
        let body = body.into();
        let created_at = created_at.into();
        self.edit("Add comment", |e| {
            let id = e.document.allocate_id();
            e.document.comments.push(CommentThread {
                id,
                node: Some(node.unwrap_or(e.active_page)),
                position,
                resolved: false,
                messages: vec![CommentMessage {
                    author,
                    body,
                    created_at,
                }],
            });
            id
        })
    }
    pub fn resolve_comment(&mut self, id: u64, resolved: bool) {
        self.transact("Resolve comment", |d| {
            if let Some(c) = d.comments.iter_mut().find(|c| c.id == id) {
                c.resolved = resolved
            }
        })
    }
    pub fn add_fill_style(&mut self, name: impl Into<String>, color: Color) -> StyleId {
        let name = name.into();
        self.transact("Create color style", |d| {
            let id = d.allocate_id();
            d.styles.insert(
                id,
                Style {
                    id,
                    name,
                    description: String::new(),
                    value: StyleValue::Fill {
                        fills: vec![Fill::solid(color)],
                    },
                },
            );
            id
        })
    }
    pub fn apply_style(&mut self, id: NodeId, style: StyleId) -> bool {
        self.apply_local_style(&[id], style).is_ok_and(|changed| changed != 0)
    }
    pub fn set_variable_mode(&mut self, mode: impl Into<String>) {
        let mode = mode.into();
        self.transact("Change variable mode", |d| {
            d.variable_mode = mode;
            d.apply_variables();
            d.layout_all();
        });
    }
    pub fn to_json(&self) -> Result<String, DocumentError> {
        self.document.to_json()
    }
    pub fn load_json(&mut self, json: &str) -> Result<(), DocumentError> {
        let d = Document::from_json(json)?;
        self.edit("Open document", |e| {
            e.active_page = d.pages[0].id;
            e.document = d;
            e.selection.clear()
        });
        Ok(())
    }
    pub fn export_svg(&self) -> String {
        self.document.export_svg(self.active_page, &[])
    }
    pub fn export_selection_svg(&self) -> String {
        self.document.export_svg(self.active_page, &self.selection)
    }
    pub fn snap_point(&self, p: Point) -> Point {
        if self.snap_to_grid {
            let s = self.grid_size.max(1.0);
            Point::new((p.x / s).round() * s, (p.y / s).round() * s)
        } else {
            p
        }
    }
    pub fn smart_snap(&self, bounds: Rect, threshold: f64, exclude: &[NodeId]) -> SnapResult {
        let mut result = SnapResult::default();
        let mut bx = threshold;
        let mut by = threshold;
        let sx = [bounds.x, bounds.center().x, bounds.right()];
        let sy = [bounds.y, bounds.center().y, bounds.bottom()];
        for id in self.scene.spatial.query(bounds.inflate(threshold * 8.0)) {
            if exclude.contains(&id) || exclude.iter().any(|p| self.document.is_ancestor(*p, id)) {
                continue;
            }
            let Some(t) = self.document.world_bounds(id) else {
                continue;
            };
            for x in [t.x, t.center().x, t.right()] {
                for source in sx {
                    let d = x - source;
                    if d.abs() < bx {
                        bx = d.abs();
                        result.delta.x = d;
                        result.vertical_guide = Some(x)
                    }
                }
            }
            for y in [t.y, t.center().y, t.bottom()] {
                for source in sy {
                    let d = y - source;
                    if d.abs() < by {
                        by = d.abs();
                        result.delta.y = d;
                        result.horizontal_guide = Some(y)
                    }
                }
            }
        }
        result
    }
    pub fn boolean_selection(&mut self, op: BooleanOperation) -> Option<NodeId> {
        self.edit("Boolean selection", |e| {
            let roots = e.root_selection();
            if roots.len() < 2 {
                return None;
            }
            let fills = e
                .document
                .nodes
                .get(&roots[0])
                .map(|n| n.fills.clone())
                .unwrap_or_else(|| vec![Fill::solid(Color::rgb(168, 132, 255))]);
            let id = e.group_internal(NodeKind::Boolean)?;
            if let Some(n) = e.document.nodes.get_mut(&id) {
                n.boolean_operation = op;
                n.fills = fills;
                n.name = match op {
                    BooleanOperation::Union => "Union",
                    BooleanOperation::Subtract => "Subtract",
                    BooleanOperation::Intersect => "Intersect",
                    BooleanOperation::Exclude => "Exclude",
                }
                .into()
            }
            Some(id)
        })
    }
    /// Updates a vector anchor or handle from an absolute document-space
    /// position. Wrap repeated calls in begin_transaction/commit_transaction
    /// for one undo record per pointer gesture. Bounds are rebased after each
    /// update while preserving every unaffected point's world position.
    pub fn move_vector_point(
        &mut self,
        id: NodeId,
        point: PathPointId,
        world_position: Point,
    ) -> bool {
        let Some(inverse) = self
            .document
            .world_transform(id)
            .and_then(Transform::inverse)
        else {
            return false;
        };
        let local = inverse.apply(world_position);
        self.edit_vector_path(id, "Move vector point", |path| {
            path.move_edit_point(point, local).then_some(())
        })
        .is_some()
    }

    /// Subdivides a segment without changing the curve's geometry or fill rule.
    /// The new anchor ID is valid in the resulting command stream.
    pub fn split_vector_segment(
        &mut self,
        id: NodeId,
        segment: usize,
        t: f64,
    ) -> Option<PathPointId> {
        self.edit_vector_path(id, "Insert vector point", |path| {
            path.split_segment(segment, t)
        })
    }

    pub fn delete_vector_anchor(&mut self, id: NodeId, segment: usize) -> bool {
        self.edit_vector_path(id, "Delete vector point", |path| {
            path.delete_anchor(segment).then_some(())
        })
        .is_some()
    }

    pub fn convert_vector_anchor(&mut self, id: NodeId, segment: usize, mode: AnchorMode) -> bool {
        self.edit_vector_path(id, "Convert vector point", |path| {
            path.convert_anchor(segment, mode).then_some(())
        })
        .is_some()
    }

    pub(crate) fn edit_vector_path<R>(
        &mut self,
        id: NodeId,
        label: &str,
        operation: impl FnOnce(&mut VectorPath) -> Option<R>,
    ) -> Option<R> {
        if self.document.effective_locked(id) {
            return None;
        }
        let mut node = self.document.nodes.get(&id)?.clone();
        if node.kind != NodeKind::Vector || node.vector_network.is_some() {
            return None;
        }
        let result = operation(&mut node.path)?;
        if !normalize_vector_bounds(&mut node) {
            return None;
        }
        self.edit_nodes(label, &[id], |editor| {
            editor.document.nodes.insert(id, node);
        });
        Some(result)
    }


}
#[derive(Clone, Copy, Debug, Default)]
pub struct SnapResult {
    pub delta: Point,
    pub vertical_guide: Option<f64>,
    pub horizontal_guide: Option<f64>,
}
fn has_selected_ancestor(document: &Document, id: NodeId, selected: &BTreeSet<NodeId>) -> bool {
    let mut current = id;
    for _ in 0..MAX_DOCUMENT_DEPTH {
        let Some(parent) = document.nodes.get(&current).and_then(|node| node.parent) else {
            return false;
        };
        if parent != id && selected.contains(&parent) {
            return true;
        }
        current = parent;
    }
    false
}

#[cfg(test)]
mod performance_tests {
    use super::*;

    #[test]
    fn transaction_identity_survives_promotion_and_never_reuses_a_replaced_editor() {
        let mut editor = Editor::default();
        let page = editor.active_page;
        let child = editor.document.insert_node(page,
            Node::new(NodeKind::Rectangle, "Owned", 0.0, 0.0, 40.0, 30.0)).unwrap();
        editor.refresh();
        let original = editor.document.to_json().unwrap();
        editor.begin_node_transaction(&[child], "Move layers");
        let token = editor.active_transaction_token().unwrap();
        editor.begin_transaction("Promote snapshot");
        assert_eq!(editor.active_transaction_token(), Some(token.clone()));
        editor.cancel_transaction();
        assert!(editor.active_transaction_token().is_none());
        assert_eq!(editor.document.to_json().unwrap(), original);
        editor.begin_transaction("Move layers");
        assert_ne!(editor.active_transaction_token(), Some(token.clone()));
        let mut replacement = Editor::new(editor.document.clone());
        replacement.revision = editor.revision;
        replacement.document.revision = editor.document.revision;
        replacement.begin_transaction("Move layers");
        assert_ne!(replacement.active_transaction_token(), editor.active_transaction_token());
        assert_ne!(replacement.active_transaction_token(), Some(token));
        assert_eq!(replacement.document.to_json().unwrap(), original);
        assert_eq!(replacement.history_len(), 0);
    }

    fn fixture() -> (Document, NodeId, NodeId, NodeId) {
        let mut document = Document::new("Scoped edit fixture");
        let page = document.pages[0].id;
        let mut frame = Node::new(NodeKind::Frame, "Clipped frame", 80.0, 30.0, 200.0, 180.0);
        frame.clip_content = true;
        frame.rotation = 17.0;
        let frame = document.insert_node(page, frame).unwrap();
        let child = document
            .insert_node(frame, Node::new(NodeKind::Rectangle, "Child", 20.0, 20.0, 80.0, 60.0))
            .unwrap();
        let sibling = document
            .insert_node(page, Node::new(NodeKind::Text, "Text", 330.0, 20.0, 200.0, 80.0)
                .with_text("Unrelated café Ω 🚀", 18.0, Color::BLACK))
            .unwrap();
        for index in 0..64 {
            document.insert_node(page, Node::new(NodeKind::Rectangle, format!("Other {index}"),
                index as f64 * 12.0, 400.0, 8.0, 8.0)).unwrap();
        }
        (document, frame, child, sibling)
    }

    fn equivalent(actual: &Editor, reference: &Editor) {
        assert_eq!(actual.to_json().unwrap(), reference.to_json().unwrap());
        assert_eq!(actual.selection, reference.selection);
        assert_eq!(actual.active_page, reference.active_page);
        assert_eq!(actual.history_len(), reference.history_len());
        assert_eq!(actual.can_undo(), reference.can_undo());
        assert_eq!(actual.can_redo(), reference.can_redo());
        let mut rebuilt = SceneIndex::default();
        rebuilt.rebuild(&actual.document, actual.active_page);
        assert_eq!(actual.scene.transforms, rebuilt.transforms);
        assert_eq!(actual.scene.draw_order, rebuilt.draw_order);
        for rect in [Rect::new(-100.0, -100.0, 1500.0, 1000.0),
            Rect::new(50.0, 20.0, 200.0, 200.0), Rect::new(400.0, 380.0, 100.0, 60.0)] {
            assert_eq!(actual.visible_nodes(rect), rebuilt.visible_nodes(rect));
        }
        for id in actual.document.nodes.keys() {
            assert_eq!(actual.scene.spatial.bounds(*id), rebuilt.spatial.bounds(*id));
        }
    }

    #[test]
    fn scoped_edits_match_full_transactions_and_rebuilt_scene_through_history() {
        let (document, frame, child, sibling) = fixture();
        let mut actual = Editor::new(document.clone());
        let mut reference = Editor::new(document);
        actual.selection = vec![frame, child];
        reference.selection = actual.selection.clone();
        actual.move_selection(18.0, -4.0);
        reference.transact("Move layers", |document| {
            let frame = document.nodes.get_mut(&frame).unwrap();
            frame.x += 18.0;
            frame.y -= 4.0;
        });
        equivalent(&actual, &reference);
        actual.update_nodes(&[child, child, sibling], "Paint", |node| {
            node.fills = vec![Fill::solid(Color::rgb(13, 42, 197))];
        });
        reference.transact("Paint", |document| {
            for id in [child, sibling] {
                document.nodes.get_mut(&id).unwrap().fills = vec![Fill::solid(Color::rgb(13, 42, 197))];
            }
        });
        equivalent(&actual, &reference);
        actual.update_node(frame, "Hide frame", |node| node.visible = false);
        reference.transact("Hide frame", |document| document.nodes.get_mut(&frame).unwrap().visible = false);
        equivalent(&actual, &reference);
        for _ in 0..3 {
            assert!(actual.undo());
            assert!(reference.undo());
            equivalent(&actual, &reference);
        }
        for _ in 0..3 {
            assert!(actual.redo());
            assert!(reference.redo());
            equivalent(&actual, &reference);
        }
        actual.document.validate().unwrap();
    }

    #[test]
    fn scoped_noop_keeps_redo_and_open_generic_transaction_preserves_all_changes() {
        let (document, frame, child, _) = fixture();
        let mut editor = Editor::new(document);
        let original = editor.to_json().unwrap();
        editor.update_node(child, "Move child", |node| node.x += 5.0);
        assert!(editor.undo());
        editor.update_node(child, "No change", |_| {});
        assert!(editor.can_redo());
        assert_eq!(editor.to_json().unwrap(), original);
        editor.begin_transaction("Mixed document edit");
        editor.document.name = "Renamed document".into();
        editor.document.nodes.get_mut(&frame).unwrap().x += 20.0;
        editor.update_nodes(&[child], "Nested fill", |node| node.opacity = 0.5);
        assert!(editor.commit_transaction());
        let edited = editor.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.to_json().unwrap(), original);
        assert!(editor.redo());
        assert_eq!(editor.to_json().unwrap(), edited);
        editor.begin_transaction("Canceled mixed edit");
        editor.document.name = "Canceled".into();
        editor.update_node(child, "Nested text", |node| node.text = "Discard".into());
        editor.cancel_transaction();
        assert_eq!(editor.to_json().unwrap(), edited);
    }

    #[test]
    fn scoped_gesture_previews_commit_once_and_cancel_without_full_snapshot() {
        let (document, frame, child, sibling) = fixture();
        let mut actual = Editor::new(document.clone());
        let mut reference = Editor::new(document);
        actual.selection = vec![frame];
        reference.selection = actual.selection.clone();
        let original = actual.to_json().unwrap();
        actual.begin_node_transaction(&[frame], "Drag frame");
        reference.begin_transaction("Drag frame");
        for _ in 0..24 {
            actual.document.nodes.get_mut(&frame).unwrap().x += 1.0;
            actual.refresh_preview_nodes(&[frame]);
            reference.document.nodes.get_mut(&frame).unwrap().x += 1.0;
            reference.refresh();
            equivalent(&actual, &reference);
            assert!(matches!(actual.pending.as_ref().map(|p| &p.before),
                Some(PendingSnapshot::Nodes { .. })));
        }
        // Scoped APIs can add another node to the gesture before changing it.
        actual.update_node(child, "Nested fill", |node| node.opacity = 0.5);
        reference.update_node(child, "Nested fill", |node| node.opacity = 0.5);
        assert!(actual.commit_transaction());
        assert!(reference.commit_transaction());
        equivalent(&actual, &reference);
        assert_eq!(actual.history_len(), 1);
        assert!(actual.undo());
        assert!(reference.undo());
        equivalent(&actual, &reference);
        assert_eq!(actual.to_json().unwrap(), original);
        assert!(actual.redo());
        assert!(reference.redo());
        equivalent(&actual, &reference);
        let committed = actual.to_json().unwrap();
        actual.begin_node_transaction(&[frame], "Canceled drag");
        actual.document.nodes.get_mut(&frame).unwrap().rotation += 30.0;
        actual.refresh_preview_nodes(&[frame]);
        // A late preview report captures the previous value of a newly affected
        // node rather than capturing its already-mutated value.
        actual.document.nodes.get_mut(&sibling).unwrap().opacity = 0.25;
        actual.refresh_preview_nodes(&[sibling]);
        actual.cancel_transaction();
        assert_eq!(actual.to_json().unwrap(), committed);
        equivalent(&actual, &reference);
        actual.begin_node_transaction(&[frame], "No movement");
        assert!(!actual.commit_transaction());
        assert_eq!(actual.history_len(), 1);
    }

    #[test]
    fn scoped_gesture_promotes_before_general_edits_and_unreported_full_refresh() {
        let (document, frame, _, sibling) = fixture();
        let mut editor = Editor::new(document);
        let original = editor.to_json().unwrap();
        editor.begin_node_transaction(&[frame], "Drag then create");
        editor.document.nodes.get_mut(&frame).unwrap().x += 10.0;
        editor.refresh_preview_nodes(&[frame]);
        editor.transact("Create another layer", |document| {
            document.name = "Combined edit".into();
            document.insert_node(document.pages[0].id,
                Node::new(NodeKind::Rectangle, "Added", 10.0, 20.0, 30.0, 40.0)).unwrap();
        });
        assert!(matches!(editor.pending.as_ref().map(|p| &p.before),
            Some(PendingSnapshot::Document(_))));
        assert!(editor.commit_transaction());
        let edited = editor.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.to_json().unwrap(), original);
        assert!(editor.redo());
        assert_eq!(editor.to_json().unwrap(), edited);
        assert!(editor.undo());

        editor.begin_node_transaction(&[frame], "Direct document edit");
        editor.document.nodes.get_mut(&frame).unwrap().x += 12.0;
        editor.refresh_preview_nodes(&[frame]);
        editor.document.nodes.get_mut(&sibling).unwrap().text = "Unreported Ω".into();
        editor.document.name = "Direct metadata".into();
        editor.refresh();
        assert!(matches!(editor.pending.as_ref().map(|p| &p.before),
            Some(PendingSnapshot::Document(_))));
        assert!(editor.commit_transaction());
        assert!(editor.undo());
        assert_eq!(editor.to_json().unwrap(), original);
    }

    #[test]
    fn scoped_gesture_uses_full_snapshot_for_nested_auto_layout() {
        let (mut document, frame, child, _) = fixture();
        document.nodes.get_mut(&frame).unwrap().layout = Some(AutoLayout {
            width: SizingMode::Hug,
            ..AutoLayout::default()
        });
        document.nodes.get_mut(&child).unwrap().kind = NodeKind::Frame;
        document.nodes.get_mut(&child).unwrap().layout = Some(AutoLayout {
            width: SizingMode::Hug,
            ..AutoLayout::default()
        });
        let leaf = document.insert_node(child,
            Node::new(NodeKind::Rectangle, "Nested", 0.0, 0.0, 40.0, 20.0)).unwrap();
        document.layout_all();
        let mut editor = Editor::new(document);
        let original = editor.to_json().unwrap();
        editor.begin_node_transaction(&[leaf], "Layout resize");
        assert!(matches!(editor.pending.as_ref().map(|p| &p.before),
            Some(PendingSnapshot::Document(_))));
        editor.document.nodes.get_mut(&leaf).unwrap().width += 100.0;
        editor.document.layout_all();
        editor.refresh();
        assert!(editor.commit_transaction());
        assert!(editor.undo());
        assert_eq!(editor.to_json().unwrap(), original);
        editor.begin_node_transaction(&[leaf], "Cancel layout resize");
        editor.document.nodes.get_mut(&leaf).unwrap().height += 100.0;
        editor.document.layout_all();
        editor.refresh();
        editor.cancel_transaction();
        assert_eq!(editor.to_json().unwrap(), original);
    }

    #[test]
    fn inspector_scope_preserves_introduced_layout_and_cached_layout_history() {
        let (document, frame, child, sibling) = fixture();
        let mut editor = Editor::new(document);
        let mut calls = 0;
        assert_eq!(editor.update_nodes_with_layout(&[sibling, sibling], "Opacity", |node| {
            calls += 1;
            node.opacity = 0.5;
        }), 1);
        assert_eq!(calls, 1);
        assert!(editor.undo.last().unwrap().node_only);
        assert_eq!(editor.layout_node_count, 0);
        let before_layout = editor.to_json().unwrap();
        editor.update_nodes_with_layout(&[frame], "Add layout", |node| {
            node.layout = Some(AutoLayout {
                width: SizingMode::Hug,
                ..AutoLayout::default()
            });
        });
        assert!(!editor.undo.last().unwrap().node_only);
        assert_eq!(editor.layout_node_count, 1);
        assert_eq!(editor.document.nodes[&child].x, 16.0);
        let with_layout = editor.to_json().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.layout_node_count, 0);
        assert_eq!(editor.to_json().unwrap(), before_layout);
        assert!(editor.redo());
        assert_eq!(editor.layout_node_count, 1);
        assert_eq!(editor.to_json().unwrap(), with_layout);
        editor.update_nodes_with_layout(&[child], "Resize layout child", |node| node.width += 40.0);
        assert!(!editor.undo.last().unwrap().node_only);
        assert_eq!(editor.document.nodes[&frame].width, editor.document.nodes[&child].width + 32.0);
        assert!(editor.undo());
        assert_eq!(editor.to_json().unwrap(), with_layout);
        editor.update_node(frame, "Remove layout", |node| node.layout = None);
        assert_eq!(editor.layout_node_count, 0);
        assert!(editor.undo());
        assert_eq!(editor.layout_node_count, 1);
    }

    #[test]
    fn ancestor_selection_filter_preserves_order_duplicates_and_locked_semantics() {
        let (mut document, frame, child, sibling) = fixture();
        document.nodes.get_mut(&sibling).unwrap().locked = true;
        let mut editor = Editor::new(document);
        editor.selection = vec![child, sibling, frame, frame];
        let reference: Vec<_> = editor.selection.iter().filter(|id| {
            !editor.document.effective_locked(**id)
                && !editor.selection.iter().any(|ancestor| ancestor != *id
                    && editor.document.is_ancestor(*ancestor, **id))
        }).copied().collect();
        assert_eq!(editor.root_selection(), reference);
        assert_eq!(reference, vec![frame, frame]);
        editor.select_rect(Rect::new(-1000.0, -1000.0, 3000.0, 3000.0), false);
        assert!(editor.selection.contains(&frame));
        assert!(!editor.selection.contains(&child));
        assert!(!editor.selection.contains(&sibling));
    }

    #[test]
    fn spatial_updates_match_exact_bounds_across_bucket_and_oversized_transitions() {
        let mut spatial = SpatialIndex::new(64.0);
        let mut bounds = BTreeMap::new();
        for (index, rect) in [Rect::new(2.0, 3.0, 10.0, 11.0),
            Rect::new(8.0, 9.0, 10.0, 11.0), Rect::new(61.0, 61.0, 12.0, 12.0),
            Rect::new(-128.0, -128.0, 4000.0, 4000.0),
            Rect::new(-256.0, -256.0, 4100.0, 4100.0),
            Rect::new(-31.0, -31.0, 9.0, 9.0)].into_iter().enumerate() {
            spatial.insert(10, rect);
            bounds.insert(10, rect);
            let other = Rect::new(index as f64 * 7.0, 0.0, 4.0, 4.0);
            spatial.insert(index as u64 + 20, other);
            bounds.insert(index as u64 + 20, other);
            for query in [Rect::new(-64.0, -64.0, 128.0, 128.0),
                Rect::new(60.0, 60.0, 16.0, 16.0), Rect::new(-10000.0, -10000.0, 20000.0, 20000.0)] {
                let expected: Vec<_> = bounds.iter().filter_map(|(id, rect)| rect.intersects(query).then_some(*id)).collect();
                assert_eq!(spatial.query(query), expected);
            }
        }
        spatial.insert(10, Rect::new(f64::NAN, 0.0, 1.0, 1.0));
        assert_eq!(spatial.bounds(10), None);
        assert!(!spatial.query(Rect::new(-5000.0, -5000.0, 10000.0, 10000.0)).contains(&10));
    }
}
fn assign_transform(n: &mut Node, t: Transform) {
    n.set_local_transform(t)
}
fn clone_tree_into(
    d: &mut Document,
    source: &BTreeMap<NodeId, Node>,
    roots: &[NodeId],
    parent: NodeId,
    offset: Point,
    instance: bool,
) -> Vec<NodeId> {
    if d.children(parent).is_none() {
        return Vec::new();
    }
    let mut ordered = Vec::new();
    let mut stack: Vec<_> = roots.iter().rev().copied().collect();
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(n) = source.get(&id) {
            ordered.push(id);
            stack.extend(n.children.iter().rev().copied())
        }
    }
    if d.nodes.len() + ordered.len() > MAX_DOCUMENT_NODES {
        return Vec::new();
    }
    let remap: BTreeMap<_, _> = ordered.iter().map(|id| (*id, d.allocate_id())).collect();
    for old in ordered {
        let Some(original) = source.get(&old) else {
            continue;
        };
        let mut n = original.clone();
        n.id = remap[&old];
        n.revision = 0;
        n.children = n
            .children
            .iter()
            .filter_map(|id| remap.get(id).copied())
            .collect();
        n.parent = if roots.contains(&old) {
            Some(parent)
        } else {
            original.parent.and_then(|id| remap.get(&id).copied())
        };
        if roots.contains(&old) {
            n.x += offset.x;
            n.y += offset.y
        }
        if instance {
            n.component_source = Some(old);
            n.component_property_definitions.clear();
            n.overridden_properties.clear();
            if n.kind == NodeKind::Component {
                n.kind = NodeKind::Instance;
                n.component_id = Some(old);
            }
        } else {
            crate::components::remap_component_references(&mut n, &remap);
            if roots.contains(&old) && n.kind == NodeKind::Component && d.is_component_set(parent) {
                n.variant_properties = crate::components::unique_variant_properties(d, parent, &n.variant_properties);
            }
        }
        for i in &mut n.interactions {
            match &mut i.action {
                PrototypeAction::NavigateTo { target }
                | PrototypeAction::OpenOverlay { target, .. }
                | PrototypeAction::ScrollTo { target } => {
                    if let Some(id) = remap.get(target) {
                        *target = *id
                    }
                }
                _ => {}
            }
        }
        d.nodes.insert(n.id, n);
    }
    let ids: Vec<_> = roots
        .iter()
        .filter_map(|id| remap.get(id).copied())
        .collect();
    if let Some(c) = d.children_mut(parent) {
        c.extend(ids.iter().copied())
    }
    ids
}
/// Update materialized instances and dependent main components while retaining
/// stable matching layer IDs and independent authored overrides.
pub fn sync_component_instances(document: &mut Document, component: NodeId) -> usize {
    document.refresh_component_instances(component).unwrap_or(0)
}

#[cfg(test)]
mod render_change_tests {
    use super::*;

    fn nested_editor() -> (Editor, NodeId, NodeId, NodeId) {
        let mut document = Document::new("Render changes");
        let frame = document.insert_node(1, Node::new(NodeKind::Frame, "Frame", 10.0, 20.0, 120.0, 100.0)).unwrap();
        let child = document.insert_node(frame, Node::new(NodeKind::Rectangle, "Child", 5.0, 6.0, 20.0, 20.0)).unwrap();
        let sibling = document.insert_node(frame, Node::new(NodeKind::Rectangle, "Sibling", 40.0, 6.0, 20.0, 20.0)).unwrap();
        (Editor::new(document), frame, child, sibling)
    }

    fn scoped(editor: &Editor, token: &EditorRenderToken, nodes: &[NodeId], ancestors: &[NodeId]) {
        let mut nodes = nodes.to_vec();
        let mut ancestors = ancestors.to_vec();
        nodes.sort_unstable(); nodes.dedup();
        ancestors.sort_unstable(); ancestors.dedup();
        assert_eq!(editor.render_changes_since(token), EditorRenderChanges::Nodes { nodes, ancestors });
    }

    #[test]
    fn render_change_feed_tracks_leaf_dependencies_and_ignores_noop_edits() {
        let (mut editor, frame, child, _) = nested_editor();
        let before = editor.render_change_token();
        editor.update_node(child, "Move", |node| node.x += 3.0);
        scoped(&editor, &before, &[child], &[frame]);
        let after = editor.render_change_token();
        editor.update_node(child, "No change", |_| {});
        assert_ne!(after, editor.render_change_token());
        scoped(&editor, &after, &[], &[]);
    }

    #[test]
    fn render_change_feed_includes_transformed_descendants_and_coalesces_missed_frames() {
        let (mut editor, frame, child, sibling) = nested_editor();
        let before = editor.render_change_token();
        editor.update_node(child, "Paint", |node| node.fills = vec![Fill::solid(Color::BLACK)]);
        editor.update_node(frame, "Transform parent", |node| node.rotation = 25.0);
        editor.update_node(sibling, "Opacity", |node| node.opacity = 0.6);
        scoped(&editor, &before, &[frame, child, sibling], &[]);
    }

    #[test]
    fn render_change_feed_preserves_preview_cancel_commit_and_undo_redo_continuity() {
        let (mut editor, frame, child, _) = nested_editor();
        let original = editor.document.nodes[&child].x;
        editor.begin_node_transaction(&[child], "Drag");
        editor.document.nodes.get_mut(&child).unwrap().x += 9.0;
        editor.refresh_preview_nodes(&[child]);
        let preview = editor.render_change_token();
        editor.cancel_transaction();
        assert_eq!(editor.document.nodes[&child].x, original);
        scoped(&editor, &preview, &[child], &[frame]);

        editor.begin_node_transaction(&[child], "Drag");
        editor.document.nodes.get_mut(&child).unwrap().x += 4.0;
        editor.refresh_preview_nodes(&[child]);
        assert!(editor.commit_transaction());
        assert_eq!(editor.history_len(), 1);
        let committed = editor.render_change_token();
        assert!(editor.undo());
        scoped(&editor, &committed, &[child], &[frame]);
        let undone = editor.render_change_token();
        assert!(editor.redo());
        scoped(&editor, &undone, &[child], &[frame]);
        assert_eq!(editor.document.nodes[&child].x, original + 4.0);
    }

    #[test]
    fn render_change_feed_requires_full_for_direct_structural_and_visibility_changes() {
        let (mut editor, frame, child, _) = nested_editor();
        let before = editor.render_change_token();
        editor.document.nodes.get_mut(&child).unwrap().x += 7.0;
        editor.refresh();
        assert_eq!(editor.render_changes_since(&before), EditorRenderChanges::Full);
        let before = editor.render_change_token();
        editor.update_node(frame, "Reorder children", |node| node.children.reverse());
        assert_eq!(editor.render_changes_since(&before), EditorRenderChanges::Full);
        let before = editor.render_change_token();
        editor.update_node(child, "Hide through opacity", |node| node.opacity = 0.0);
        assert_eq!(editor.render_changes_since(&before), EditorRenderChanges::Full);
        let before = editor.render_change_token();
        editor.transact("Global metadata", |document| document.name = "Renamed".into());
        assert_eq!(editor.render_changes_since(&before), EditorRenderChanges::Full);
    }

    #[test]
    fn render_change_feed_does_not_reuse_replaced_editor_or_tampered_revision() {
        let (mut editor, _, child, _) = nested_editor();
        let token = editor.render_change_token();
        let replacement = Editor::new(editor.document.clone());
        assert_eq!(replacement.render_changes_since(&token), EditorRenderChanges::Full);
        editor.document.revision += 1;
        assert_eq!(editor.render_changes_since(&token), EditorRenderChanges::Full);
        editor.refresh();
        let token = editor.render_change_token();
        editor.update_node(child, "Move", |node| node.x += 1.0);
        let future = editor.render_change_token();
        assert_eq!(replacement.render_changes_since(&future), EditorRenderChanges::Full);
        assert!(matches!(editor.render_changes_since(&token), EditorRenderChanges::Nodes { .. }));
    }

    #[test]
    fn render_change_feed_bounds_retained_history_and_declines_skipped_revisions() {
        let (mut editor, _, child, _) = nested_editor();
        let old = editor.render_change_token();
        for _ in 0..=RENDER_CHANGE_HISTORY {
            editor.update_node(child, "Move", |node| node.x += 1.0);
        }
        assert_eq!(editor.render_changes.entries.len(), RENDER_CHANGE_HISTORY);
        assert!(editor.render_changes.retained_ids <= RENDER_CHANGE_RETAINED_IDS);
        assert_eq!(editor.render_changes_since(&old), EditorRenderChanges::Full);
        let current = editor.render_change_token();
        scoped(&editor, &current, &[], &[]);
    }

    #[test]
    fn render_change_feed_declines_oversized_subtree_and_resets_identity_at_sequence_wrap() {
        let mut document = Document::new("Large dependency set");
        let frame = document.insert_node(1, Node::new(NodeKind::Frame, "Frame", 0.0, 0.0, 100.0, 100.0)).unwrap();
        for _ in 0..RENDER_CHANGE_NODE_LIMIT {
            document.insert_node(frame, Node::new(NodeKind::Rectangle, "Child", 0.0, 0.0, 1.0, 1.0)).unwrap();
        }
        let mut editor = Editor::new(document);
        let before = editor.render_change_token();
        editor.update_node(frame, "Move large group", |node| node.x += 1.0);
        assert_eq!(editor.render_changes_since(&before), EditorRenderChanges::Full);
        editor.render_changes.sequence = u64::MAX;
        let before_wrap = editor.render_change_token();
        editor.update_node(frame, "No change", |_| {});
        assert_eq!(editor.render_changes_since(&before_wrap), EditorRenderChanges::Full);
        assert_eq!(editor.render_changes.sequence, 1);
    }
}
