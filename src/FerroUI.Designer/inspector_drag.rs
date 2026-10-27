//! Outline drags edit the authored document. TreeDataGrid supplies native drag gestures,
//! hit testing and adorners; its automatic row-model mutation is always intercepted.
use super::inspectors::TreeRow;
use super::{DesignerSession, DesignerWorkspace, OutlineMode};
use ferroui_base::input::{
    DataFormat, DataTransferItem, DragDrop, DragDropEffects, InputElement, Key, KeyModifiers,
    PointerType,
};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::Control;
use ferroui_controls_treedatagrid::{
    TreeDataGridRowDragEventArgs, TreeDataGridRowDropPosition as DropPosition,
};
use ferroui_designer_core::{Ast, Document, Edit, MarkupValue, NodeId, NodeKind};
use ferroui_designer_tools::LayoutCapabilities;
use ferroui_treedatagrid_core::IndexPath;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

/// Structural edits in source order. Commands operate on the outermost selected elements,
/// preserve their identities, and reject partial moves across incompatible content hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutlineCommand {
    MoveUp,
    MoveDown,
    MoveFirst,
    MoveLast,
    Indent,
    Outdent,
}

impl OutlineCommand {
    fn label(self) -> &'static str {
        match self {
            Self::MoveUp => "Move outline selection up",
            Self::MoveDown => "Move outline selection down",
            Self::MoveFirst => "Move outline selection first",
            Self::MoveLast => "Move outline selection last",
            Self::Indent => "Indent outline selection",
            Self::Outdent => "Outdent outline selection",
        }
    }
}

/// An immutable gesture snapshot. A revision alone cannot distinguish two independently
/// created documents whose local node IDs happen to match.
struct OutlineDrag {
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    nodes: Vec<NodeId>,
    hover: Cell<Option<(NodeId, DropPosition, bool)>>,
}

impl OutlineDrag {
    fn capture(
        session: &DesignerSession,
        nodes: impl IntoIterator<Item = NodeId>,
    ) -> Result<Self, String> {
        ensure_editable(session)?;
        let document = session.document();
        let selected = nodes.into_iter().collect::<BTreeSet<_>>();
        if selected.is_empty() {
            return Err("Select an authored element to move".into());
        }
        for node in &selected {
            let element = document
                .ast()
                .node(*node)
                .ok_or("The selected element no longer exists")?;
            if element.kind != NodeKind::Element {
                return Err("Move content elements, not property declarations".into());
            }
            ContentHost::of_child(session, *node)?;
        }
        // Keep source order, independently of click order, and move a selected subtree once.
        let nodes = document
            .ast()
            .nodes_in_source_order()
            .filter(|node| {
                selected.contains(&node.id)
                    && !has_selected_ancestor(document.ast(), node.id, &selected)
            })
            .map(|node| node.id)
            .collect();
        Ok(Self {
            uri: session.document_uri().to_owned(),
            revision: document.revision(),
            ast: document.ast_snapshot(),
            nodes,
            hover: Cell::new(None),
        })
    }

    fn can_drop(&self, session: &DesignerSession, target: NodeId, position: DropPosition) -> bool {
        if self.validate(session).is_err() {
            return false;
        }
        if let Some((previous, placement, allowed)) = self.hover.get() {
            if previous == target && placement == position {
                return allowed;
            }
        }
        let allowed = self.plan(session, target, position).is_ok();
        self.hover.set(Some((target, position, allowed)));
        allowed
    }

    fn validate(&self, session: &DesignerSession) -> Result<(), String> {
        ensure_editable(session)?;
        if self.uri != session.document_uri()
            || self.revision != session.document().revision()
            || !Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
        {
            return Err("The document changed during the drag; start the move again".into());
        }
        Ok(())
    }

    fn plan(
        &self,
        session: &DesignerSession,
        target: NodeId,
        position: DropPosition,
    ) -> Result<Vec<Edit>, String> {
        self.validate(session)?;
        let document = session.document();
        let selected = self.nodes.iter().copied().collect::<BTreeSet<_>>();
        if selected.contains(&target) || has_selected_ancestor(document.ast(), target, &selected) {
            return Err("An element cannot be dropped on itself or its descendants".into());
        }
        let (host, anchor) = match position {
            DropPosition::Inside => (ContentHost::of_target(session, target)?, None),
            DropPosition::Before | DropPosition::After => {
                let host = ContentHost::of_child(session, target)?;
                if !host.capabilities.reorder {
                    return Err("This content host does not support sibling reordering".into());
                }
                (host, Some(target))
            }
            DropPosition::None => {
                return Err("Choose a position inside, before or after an element".into());
            }
        };
        if selected.contains(&host.container)
            || has_selected_ancestor(document.ast(), host.container, &selected)
        {
            return Err("An element cannot become a child of its own subtree".into());
        }
        for node in &self.nodes {
            ContentHost::of_child(session, *node)?;
        }
        host.validate_capacity(document, &selected)?;

        let children = document.ast().element_children(host.container);
        let boundary = match anchor {
            Some(anchor) => {
                children
                    .iter()
                    .position(|id| *id == anchor)
                    .ok_or("The target is no longer a content child")?
                    + usize::from(position == DropPosition::After)
            }
            None => children.len(),
        };
        self.plan_boundary(
            session,
            &host,
            boundary,
            ferroui_designer_tools::Point::new(0.0, 0.0),
        )
    }

    /// Shared source-order planner for tree drops and measured canvas insertion boundaries.
    fn plan_boundary(
        &self,
        session: &DesignerSession,
        host: &ContentHost,
        boundary: usize,
        canvas_delta: ferroui_designer_tools::Point,
    ) -> Result<Vec<Edit>, String> {
        self.validate(session)?;
        let document = session.document();
        let selected = self.nodes.iter().copied().collect::<BTreeSet<_>>();
        if selected.contains(&host.container)
            || has_selected_ancestor(document.ast(), host.container, &selected)
        {
            return Err("An element cannot become a child of its own subtree".into());
        }
        host.validate_capacity(document, &selected)?;
        let children = document.ast().element_children(host.container);
        if boundary > children.len() {
            return Err("The insertion boundary is outside the content collection".into());
        }
        // MoveNode indices address the list after removing that one node. Work from the
        // original ranks and count removals before the insertion boundary; no per-node
        // document clone, reparsing, or repeated scan of a large sibling list is needed.
        let ranks = children
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect::<BTreeMap<_, _>>();
        let mut remaining = children
            .iter()
            .copied()
            .filter(|id| !selected.contains(id))
            .collect::<Vec<_>>();
        let removed_before = self
            .nodes
            .iter()
            .filter(|id| ranks.get(id).is_some_and(|rank| *rank < boundary))
            .count();
        remaining.splice(
            boundary - removed_before..boundary - removed_before,
            self.nodes.iter().copied(),
        );
        if remaining == children {
            return Ok(Vec::new());
        }
        if !host.capabilities.reorder && self.nodes.iter().any(|node| ranks.contains_key(node)) {
            return Err("This content host does not support sibling reordering".into());
        }
        let mut removed_before = 0;
        let mut edits = crate::workspace_tools::canvas_reparent_edits(
            session,
            &self.nodes,
            host.owner,
            canvas_delta,
        )?;
        edits.extend(
            self.nodes
                .iter()
                .enumerate()
                .map(|(offset, node)| {
                    removed_before +=
                        usize::from(ranks.get(node).is_some_and(|rank| *rank < boundary));
                    Edit::MoveNode {
                        node: *node,
                        parent: host.container,
                        index: boundary + offset - removed_before,
                    }
                })
                .collect::<Vec<_>>(),
        );
        Ok(edits)
    }

    fn commit(
        &self,
        session: &mut DesignerSession,
        target: NodeId,
        position: DropPosition,
    ) -> Result<(), String> {
        let edits = self.plan(session, target, position)?;
        if !edits.is_empty() {
            session
                .apply_at(self.revision, "Move outline selection", edits)
                .map_err(|error| error.to_string())?;
            session.select(self.nodes.iter().copied());
        }
        Ok(())
    }

    fn plan_command(
        &self,
        session: &DesignerSession,
        command: OutlineCommand,
    ) -> Result<Vec<Edit>, String> {
        self.validate(session)?;
        let first = *self.nodes.first().ok_or("Select an element to arrange")?;
        let host = ContentHost::of_child(session, first)?;
        for node in &self.nodes {
            if ContentHost::of_child(session, *node)?.container != host.container {
                return Err("Select sibling elements for structural keyboard commands".into());
            }
        }
        let document = session.document();
        let selected = self.nodes.iter().copied().collect::<BTreeSet<_>>();
        let mut children = document.ast().element_children(host.container);
        let is_content = |id: NodeId| {
            document
                .ast()
                .node(id)
                .is_some_and(|node| node.kind == NodeKind::Element)
        };
        if command == OutlineCommand::Outdent {
            return self.plan(session, host.owner, DropPosition::After);
        }
        if command == OutlineCommand::Indent {
            let index = children.iter().position(|id| *id == first).unwrap();
            let previous = children[..index]
                .iter()
                .rev()
                .find(|id| is_content(**id))
                .copied()
                .ok_or("No preceding sibling accepts this selection")?;
            return self.plan(session, previous, DropPosition::Inside);
        }
        if !host.capabilities.reorder {
            return Err("This content host does not support sibling reordering".into());
        }
        if matches!(
            command,
            OutlineCommand::MoveFirst | OutlineCommand::MoveLast
        ) {
            let remaining = children
                .iter()
                .copied()
                .filter(|id| !selected.contains(id) && is_content(*id))
                .collect::<Vec<_>>();
            let Some(anchor) = (if command == OutlineCommand::MoveFirst {
                remaining.first()
            } else {
                remaining.last()
            }) else {
                return Ok(Vec::new());
            };
            return self.plan(
                session,
                *anchor,
                if command == OutlineCommand::MoveFirst {
                    DropPosition::Before
                } else {
                    DropPosition::After
                },
            );
        }
        // Each contiguous selected block advances one sibling, retaining gaps between
        // disjoint blocks. The simulated ordering supplies post-removal MoveNode indices.
        let order = if command == OutlineCommand::MoveUp {
            self.nodes.clone()
        } else {
            self.nodes.iter().rev().copied().collect()
        };
        let mut edits = Vec::new();
        for node in order {
            let current = children.iter().position(|id| *id == node).unwrap();
            let neighbour = if command == OutlineCommand::MoveUp {
                (0..current)
                    .rev()
                    .find(|index| is_content(children[*index]))
            } else {
                (current + 1..children.len()).find(|index| is_content(children[*index]))
            };
            let Some(neighbour) = neighbour else { continue };
            if selected.contains(&children[neighbour]) {
                continue;
            }
            children.remove(current);
            children.insert(neighbour, node);
            edits.push(Edit::MoveNode {
                node,
                parent: host.container,
                index: neighbour,
            });
        }
        Ok(edits)
    }
}

impl DesignerSession {
    pub(crate) fn content_target_container(&self, owner: NodeId) -> Result<NodeId, String> {
        ContentHost::of_owner(self, owner).map(|host| host.container)
    }

    pub(crate) fn validate_content_insertion(&self, owner: NodeId) -> Result<(), String> {
        let host = ContentHost::of_owner(self, owner)?;
        host.validate_capacity_with_count(self.document(), &BTreeSet::new(), 1)
    }

    /// Plans identity-preserving content movement at a boundary in the original AST child
    /// list. Canvas placement and outline drags share slot, capacity, cycle and order rules.
    pub(crate) fn plan_content_move(
        &self,
        nodes: &[NodeId],
        owner: NodeId,
        boundary: usize,
        canvas_delta: ferroui_designer_tools::Point,
    ) -> Result<Vec<Edit>, String> {
        let snapshot = OutlineDrag::capture(self, nodes.iter().copied())?;
        let host = ContentHost::of_owner(self, owner)?;
        snapshot.plan_boundary(self, &host, boundary, canvas_delta)
    }

    /// Tests whether a structural command can change the current authored selection.
    pub fn can_edit_outline(&self, command: OutlineCommand) -> bool {
        OutlineDrag::capture(self, self.selection().iter().copied())
            .and_then(|snapshot| snapshot.plan_command(self, command))
            .is_ok_and(|edits| !edits.is_empty())
    }

    /// Applies a tree structural command in one source/undo transaction. A boundary no-op
    /// leaves revision/history untouched, and rejected selections are never partly moved.
    pub fn edit_outline(&mut self, command: OutlineCommand) -> Result<(), String> {
        let snapshot = OutlineDrag::capture(self, self.selection().iter().copied())?;
        let edits = snapshot.plan_command(self, command)?;
        if !edits.is_empty() {
            self.apply_at(snapshot.revision, command.label(), edits)
                .map_err(|error| error.to_string())?;
            self.select(snapshot.nodes);
        }
        Ok(())
    }
}

fn ensure_editable(session: &DesignerSession) -> Result<(), String> {
    session
        .ensure_writable()
        .map_err(|error| error.to_string())?;
    if !session.can_manipulate() {
        return Err("Outline moves require a successful preview of the current XAML".into());
    }
    Ok(())
}

fn has_selected_ancestor(ast: &Ast, node: NodeId, selected: &BTreeSet<NodeId>) -> bool {
    let mut parent = ast.node(node).and_then(|node| node.parent);
    while let Some(id) = parent {
        if selected.contains(&id) {
            return true;
        }
        parent = ast.node(id).and_then(|node| node.parent);
    }
    false
}

struct ContentHost {
    owner: NodeId,
    container: NodeId,
    capabilities: LayoutCapabilities,
    multiple: bool,
}

impl ContentHost {
    fn of_owner(session: &DesignerSession, owner: NodeId) -> Result<Self, String> {
        let document = session.document();
        let element = document
            .ast()
            .node(owner)
            .ok_or("The content host no longer exists")?;
        if element.kind != NodeKind::Element {
            return Err("A property declaration is not a content host".into());
        }
        let capabilities = session
            .runtime_layout_capabilities(owner)
            .filter(|capabilities| capabilities.drop_items && !capabilities.generated_items)
            .ok_or("This element does not accept authored layout children")?;
        // A custom panel may allow insertion while disabling reordering. Its native
        // collection remains multi-content; capability flags describe permissions.
        let multiple = capabilities.reorder
            || session
                .preview
                .visual(owner)
                .is_some_and(|visual| visual.cast::<ferroui_controls::Panel>().is_some());
        let mut slots = element
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .filter(|node| {
                node.kind == NodeKind::PropertyElement
                    && if multiple {
                        node.local_name().ends_with(".Children")
                    } else {
                        node.local_name().ends_with(".Content")
                            || node.local_name().ends_with(".Child")
                    }
            });
        let container = slots.next().map_or(owner, |node| node.id);
        if slots.next().is_some() {
            return Err("The content host has conflicting content declarations".into());
        }
        Ok(Self {
            owner,
            container,
            capabilities,
            multiple,
        })
    }

    fn of_target(session: &DesignerSession, target: NodeId) -> Result<Self, String> {
        let node = session
            .document()
            .ast()
            .node(target)
            .ok_or("The target no longer exists")?;
        if node.kind == NodeKind::PropertyElement {
            let host = Self::of_owner(session, node.parent.ok_or("A property requires an owner")?)?;
            if host.container != target {
                return Err("Drop inside the host's Children, Content or Child property".into());
            }
            Ok(host)
        } else {
            Self::of_owner(session, target)
        }
    }

    fn of_child(session: &DesignerSession, child: NodeId) -> Result<Self, String> {
        let document = session.document();
        let element = document
            .ast()
            .node(child)
            .ok_or("The element no longer exists")?;
        if element.kind != NodeKind::Element {
            return Err("Property declarations cannot be reordered as content".into());
        }
        let parent = element.parent.ok_or("The document root cannot be moved")?;
        let host = Self::of_target(session, parent)?;
        if host.container != parent {
            return Err("The element is outside its host's authored content collection".into());
        }
        Ok(host)
    }

    fn validate_capacity(
        &self,
        document: &Document,
        moving: &BTreeSet<NodeId>,
    ) -> Result<(), String> {
        self.validate_capacity_with_count(document, moving, moving.len())
    }

    fn validate_capacity_with_count(
        &self,
        document: &Document,
        moving: &BTreeSet<NodeId>,
        incoming: usize,
    ) -> Result<(), String> {
        if self.multiple {
            return Ok(());
        }
        let owner = document
            .ast()
            .node(self.owner)
            .ok_or("Unknown content host")?;
        let container = document
            .ast()
            .node(self.container)
            .ok_or("Unknown content container")?;
        let has_attribute = ["Content", "Child"]
            .iter()
            .any(|name| owner.attribute(name).is_some_and(|value| !value.is_empty()));
        let mut existing = 0;
        let mut has_text = false;
        for child in container
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
        {
            existing += usize::from(child.kind == NodeKind::Element && !moving.contains(&child.id));
            has_text |= matches!(child.kind, NodeKind::Text | NodeKind::CData)
                && child
                    .text
                    .as_ref()
                    .is_some_and(|text| !text.trim().is_empty());
        }
        if existing + incoming > 1 || has_attribute || has_text {
            return Err(
                "This host accepts one child and already has content; select an inner panel".into(),
            );
        }
        Ok(())
    }
}

fn target(event: &TreeDataGridRowDragEventArgs) -> Result<NodeId, String> {
    event
        .target_row()
        .and_then(|row| row.model())
        .as_ref()
        .and_then(|model| model.downcast_ref::<TreeRow>())
        .filter(|row| row.entry.runtime.is_none())
        .and_then(|row| row.entry.source)
        .ok_or_else(|| "Choose an authored source row".into())
}

enum OutlinePayload {
    Move(OutlineDrag),
    Reference(String, crate::property_controls::PropertyReference),
}

struct ReferencePress {
    value: String,
    reference: crate::property_controls::PropertyReference,
    started: bool,
    cancelled: bool,
}

impl ReferencePress {
    fn validate(&self, session: &DesignerSession) -> Result<(), String> {
        if self.cancelled {
            return Err("Reference drag cancelled".into());
        }
        if self.reference.document_uri != session.document_uri()
            || self.reference.revision != session.document().revision()
            || !Arc::ptr_eq(&self.reference.ast, &session.document().ast_snapshot())
        {
            return Err("The document changed during the reference drag; start again".into());
        }
        Ok(())
    }
}

/// Quote a literal argument without changing strings beginning with a markup-extension
/// brace. XML escaping is performed later by the document's attribute edit serializer.
fn markup_argument(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        "'{}{escaped}'",
        if value.starts_with('{') { "{}" } else { "" }
    )
}

fn outline_reference(
    session: &DesignerSession,
    node: NodeId,
    modifiers: KeyModifiers,
) -> Result<Option<OutlinePayload>, String> {
    use crate::property_controls::{PropertyReference, PropertyReferenceKind};
    let document = session.document();
    let element = document
        .ast()
        .node(node)
        .ok_or("The dragged source no longer exists")?;
    let key = element.attributes.iter().find(|attribute| {
        attribute.name.rsplit(':').next() == Some("Key")
            && attribute.namespace_uri.as_deref()
                == Some("http://schemas.microsoft.com/winfx/2006/xaml")
    });
    let (value, kind) = if let Some(key) = key {
        let MarkupValue::Literal(key) = key.markup()? else {
            return Err("Resource reference drags require a string key".into());
        };
        let extension = if modifiers.contains(KeyModifiers::SHIFT) {
            "DynamicResource"
        } else {
            "StaticResource"
        };
        (
            format!("{{{extension} {}}}", markup_argument(&key)),
            PropertyReferenceKind::Resource,
        )
    } else if modifiers.contains(KeyModifiers::ALT) {
        let name = element
            .xaml_name()
            .filter(|name| !name.is_empty())
            .ok_or("Alt-drag references require a named element")?;
        (
            format!("{{Binding ElementName={}}}", markup_argument(name)),
            PropertyReferenceKind::Element,
        )
    } else {
        return Ok(None);
    };
    Ok(Some(OutlinePayload::Reference(
        value,
        PropertyReference {
            document_uri: session.document_uri().to_owned(),
            revision: document.revision(),
            ast: document.ast_snapshot(),
            node,
            kind,
        },
    )))
}

#[derive(Default)]
struct OutlineHover {
    target: Cell<Option<NodeId>>,
    timer: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl OutlineHover {
    fn cancel(&self) {
        self.target.set(None);
        if let Some(timer) = self.timer.borrow_mut().take() {
            timer.dispose();
        }
    }
}

impl Drop for OutlineHover {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Resolve the path again at timer delivery. A recycled row or a newer hierarchy must
/// never expand an unrelated node simply because it occupies the old row index.
fn expand_hover_target(
    workspace: &DesignerWorkspace,
    drag: &RefCell<Option<OutlineDrag>>,
    target: NodeId,
    path: &IndexPath,
) -> bool {
    if workspace.ensure_outline_drag_view().is_err()
        || !drag.borrow().as_ref().is_some_and(|snapshot| {
            snapshot.can_drop(&workspace.session.borrow(), target, DropPosition::Inside)
        })
    {
        return false;
    }
    let source = &workspace.inspectors.outline.source;
    let current = source.try_get_model_at(path);
    if !current
        .as_ref()
        .is_some_and(|row| row.entry.source == Some(target) && row.entry.runtime.is_none())
    {
        return false;
    }
    source.expand(path);
    true
}

impl DesignerWorkspace {
    fn connect_outline_reference_press(
        self: &Rc<Self>,
        gesture: Rc<RefCell<Option<ReferencePress>>>,
    ) {
        let grid = self.outline_grid();
        let weak = Rc::downgrade(self);
        let press = gesture.clone();
        grid.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                if press.borrow_mut().take().is_some() {
                    this.rebuild_outline();
                }
                let point = event.get_current_point(None);
                let primary = match event.pointer().type_() {
                    PointerType::Mouse => point.properties.is_left_button_pressed,
                    PointerType::Pen => point.properties.is_right_button_pressed,
                    _ => false,
                };
                if !primary || this.ensure_outline_drag_view().is_err() {
                    return;
                }
                let grid = this.outline_grid();
                let Some(source) = event.source().and_then(|source| source.cast::<Control>())
                else {
                    return;
                };
                let Some(row) = grid.try_get_row_of(Some(&source)) else {
                    return;
                };
                let Some(rows) = grid.rows() else { return };
                let path = rows.row_index_to_model_index(row.row_index());
                let Some(model) = this.inspectors.outline.source.try_get_model_at(&path) else {
                    return;
                };
                let Some(node) = model.entry.source.filter(|_| model.entry.runtime.is_none())
                else {
                    return;
                };
                let Ok(Some(OutlinePayload::Reference(value, reference))) =
                    outline_reference(&this.session.borrow(), node, event.key_modifiers())
                else {
                    return;
                };

                // Keep the inspector's destination selected while the native grid tracks the
                // reference source. Preselecting also defers native click selection until
                // release and prevents Shift-resource drags from selecting a range of rows.
                let previous = this.inspectors.replace_selection_guard(true);
                let selection = this.inspectors.outline.source.row_selection().unwrap();
                let selection = selection.base();
                selection.begin_batch_update();
                selection.clear();
                selection.select(&path);
                selection.end_batch_update();
                this.inspectors.replace_selection_guard(previous);
                *press.borrow_mut() = Some(ReferencePress {
                    value,
                    reference,
                    started: false,
                    cancelled: false,
                });
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        let release = gesture.clone();
        grid.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                let Some(press) = release.borrow_mut().take() else {
                    return;
                };
                // A click still selects the clicked row normally; a completed or cancelled
                // reference drag cannot replace the destination after the native drag exits.
                if press.started || press.validate(&this.session.borrow()).is_err() {
                    event.set_handled(true);
                }
                this.rebuild_outline();
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = Rc::downgrade(self);
        grid.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key == Key::Escape && gesture.borrow().is_some() {
                    if let Some(press) = gesture.borrow_mut().as_mut() {
                        press.cancelled = true;
                    }
                    if let Some(this) = weak.upgrade() {
                        this.rebuild_outline();
                    }
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    pub(super) fn connect_inspector_drag(self: &Rc<Self>) {
        let grid = self.outline_grid();
        grid.set_auto_drag_drop_rows(true);
        let reference_press = Rc::new(RefCell::new(None::<ReferencePress>));
        self.connect_outline_reference_press(reference_press.clone());
        let weak = Rc::downgrade(self);
        grid.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key_modifiers != KeyModifiers::ALT {
                    return;
                }
                let command = match event.key {
                    Key::Up => OutlineCommand::MoveUp,
                    Key::Down => OutlineCommand::MoveDown,
                    Key::Home => OutlineCommand::MoveFirst,
                    Key::End => OutlineCommand::MoveLast,
                    Key::Right => OutlineCommand::Indent,
                    Key::Left => OutlineCommand::Outdent,
                    _ => return,
                };
                let Some(this) = weak.upgrade() else { return };
                if this.text_input_focused() {
                    return;
                }
                event.set_handled(true);
                let result = this.edit_outline(command);
                this.report(result);
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let drag = Rc::new(RefCell::new(None::<OutlineDrag>));
        let hover = Rc::new(OutlineHover::default());
        let leave = hover.clone();
        grid.add_handler(DragDrop::drag_leave_event(), move |_, _| leave.cancel());
        let weak = Rc::downgrade(self);
        let state = drag.clone();
        let start_hover = hover.clone();
        grid.row_drag_started(move |_, event| {
            start_hover.cancel();
            state.borrow_mut().take();
            event.set_allowed_effects(DragDropEffects::NONE);
            let Some(this) = weak.upgrade() else { return };
            let has_reference_press = reference_press.borrow().is_some();
            let result = (|| {
                this.ensure_outline_drag_view()?;
                let nodes = event
                    .models()
                    .iter()
                    .map(|model| {
                        model
                            .as_ref()
                            .and_then(|model| model.downcast_ref::<TreeRow>())
                            .filter(|row| row.entry.runtime.is_none())
                            .and_then(|row| row.entry.source)
                            .ok_or_else(|| "Only authored source rows can be moved".to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let session = this.session.borrow();
                if let Some(press) = reference_press.borrow_mut().as_mut() {
                    press.started = true;
                    press.validate(&session)?;
                    if nodes != [press.reference.node] {
                        return Err("The reference source selection changed during the drag".into());
                    }
                    return Ok(OutlinePayload::Reference(press.value.clone(), press.reference.clone()));
                }
                if nodes.len() == 1 {
                    if let Some(payload) = outline_reference(&session, nodes[0], event.key_modifiers())? {
                        return Ok(payload);
                    }
                } else if event.key_modifiers().contains(KeyModifiers::ALT) {
                    return Err("Drag one named element to create a property reference".into());
                }
                OutlineDrag::capture(&session, nodes).map(OutlinePayload::Move)
            })();
            if has_reference_press {
                this.rebuild_outline();
            }
            match result {
                Ok(OutlinePayload::Move(snapshot)) => {
                    *state.borrow_mut() = Some(snapshot);
                    event.set_allowed_effects(DragDropEffects::MOVE);
                }
                Ok(OutlinePayload::Reference(value, reference)) => {
                    let item = DataTransferItem::create(&crate::property_controls::property_value_data_format(), Some(value.clone()));
                    item.set(&crate::property_controls::property_reference_data_format(), Some(reference));
                    item.set(&DataFormat::text(), Some(value));
                    event.data_transfer().add(item);
                    event.set_allowed_effects(DragDropEffects::COPY);
                    this.status.set_text(Some("Drop the reference on a property value; Shift-drag resources for a dynamic reference"));
                }
                Err(error) => this.status.set_text(Some(&error)),
            }
        });
        let weak = Rc::downgrade(self);
        let state = drag.clone();
        let over_hover = hover.clone();
        grid.row_drag_over(move |_, event| {
            let this = weak.upgrade();
            let allowed = this.as_ref().is_some_and(|this| {
                this.ensure_outline_drag_view().is_ok()
                    && state.borrow().as_ref().is_some_and(|snapshot| {
                        target(event).is_ok_and(|target| {
                            snapshot.can_drop(&this.session.borrow(), target, event.position())
                        })
                    })
            });
            event.inner().set_drag_effects(if allowed {
                DragDropEffects::MOVE
            } else {
                DragDropEffects::NONE
            });
            if !allowed {
                event.set_position(DropPosition::None);
            }
            let expandable = allowed
                .then(|| target(event).ok())
                .flatten()
                .filter(|_| event.position() == DropPosition::Inside);
            if over_hover.target.get() == expandable {
                return;
            }
            over_hover.cancel();
            let Some(target) = expandable else { return };
            let Some(this) = this else { return };
            let Some(row) = event.target_row() else {
                return;
            };
            let Some(rows) = this.outline_grid().rows() else {
                return;
            };
            let path = rows.row_index_to_model_index(row.row_index());
            if path.is_empty() {
                return;
            }
            over_hover.target.set(Some(target));
            let weak = Rc::downgrade(&this);
            let state = Rc::downgrade(&state);
            let hover = Rc::downgrade(&over_hover);
            *over_hover.timer.borrow_mut() = Some(DispatcherTimer::run_once(
                move || {
                    let (Some(this), Some(state), Some(hover)) =
                        (weak.upgrade(), state.upgrade(), hover.upgrade())
                    else {
                        return;
                    };
                    if hover.target.get() == Some(target) {
                        expand_hover_target(&this, &state, target, &path);
                    }
                },
                Duration::from_millis(600),
                DispatcherPriority::BACKGROUND,
            ));
        });
        let weak = Rc::downgrade(self);
        grid.row_drop(move |_, event| {
            hover.cancel();
            // TreeDataGrid must never move its immutable view models directly, even on a
            // rejected or foreign drag. Rebuilding from the committed AST is the only update.
            event.set_handled(true);
            event.inner().set_handled(true);
            event.inner().set_drag_effects(DragDropEffects::NONE);
            let Some(snapshot) = drag.borrow_mut().take() else {
                return;
            };
            let Some(this) = weak.upgrade() else { return };
            let result = this.ensure_outline_drag_view().and_then(|_| {
                target(event).and_then(|target| {
                    snapshot.commit(&mut this.session.borrow_mut(), target, event.position())
                })
            });
            if result.is_ok() {
                event.inner().set_drag_effects(DragDropEffects::MOVE);
            }
            this.refresh(true);
            this.report(result);
        });
    }

    /// Runs a structural tree command after validating that the visible source hierarchy
    /// still describes the current document. The native grid retains/reveals selection.
    pub fn edit_outline(self: &Rc<Self>, command: OutlineCommand) -> Result<(), String> {
        self.ensure_outline_drag_view()?;
        self.session.borrow_mut().edit_outline(command)?;
        self.refresh(true);
        Ok(())
    }

    fn ensure_outline_drag_view(&self) -> Result<(), String> {
        if self.outline_mode() != OutlineMode::Source {
            return Err("Switch to the source hierarchy to move authored elements".into());
        }
        if self.pending_source.get()
            || !self
                .inspectors
                .source_outline_is_current(&self.session.borrow())
        {
            return Err("Wait for the source hierarchy to refresh before moving elements".into());
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "inspector_drag_tests.rs"]
mod tests;
