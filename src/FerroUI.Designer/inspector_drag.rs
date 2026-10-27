//! Outline drags edit the authored document. TreeDataGrid supplies native drag gestures,
//! hit testing and adorners; its automatic row-model mutation is always intercepted.
use super::inspectors::TreeRow;
use super::{DesignerSession, DesignerWorkspace, OutlineMode};
use ferroui_base::input::DragDropEffects;
use ferroui_controls_treedatagrid::{
    TreeDataGridRowDragEventArgs, TreeDataGridRowDropPosition as DropPosition,
};
use ferroui_designer_core::{Ast, Document, Edit, NodeId, NodeKind};
use ferroui_designer_tools::LayoutCapabilities;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

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
                return Err("Choose a position inside, before or after an element".into())
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

        // MoveNode indices address the list after removing that one node. Work from the
        // original ranks and count removals before the insertion boundary; no per-node
        // document clone, reparsing, or repeated scan of a large sibling list is needed.
        let children = document.ast().element_children(host.container);
        let ranks = children
            .iter()
            .enumerate()
            .map(|(index, id)| (*id, index))
            .collect::<BTreeMap<_, _>>();
        let boundary = match anchor {
            Some(anchor) => {
                ranks
                    .get(&anchor)
                    .copied()
                    .ok_or("The target is no longer a content child")?
                    + usize::from(position == DropPosition::After)
            }
            None => children.len(),
        };
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
        Ok(self
            .nodes
            .iter()
            .enumerate()
            .map(|(offset, node)| {
                removed_before += usize::from(ranks.get(node).is_some_and(|rank| *rank < boundary));
                Edit::MoveNode {
                    node: *node,
                    parent: host.container,
                    index: boundary + offset - removed_before,
                }
            })
            .collect())
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
        if existing + moving.len() > 1 || has_attribute || has_text {
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

impl DesignerWorkspace {
    pub(super) fn connect_inspector_drag(self: &Rc<Self>) {
        let grid = self.outline_grid();
        grid.set_auto_drag_drop_rows(true);
        let drag = Rc::new(RefCell::new(None::<OutlineDrag>));
        let weak = Rc::downgrade(self);
        let state = drag.clone();
        grid.row_drag_started(move |_, event| {
            state.borrow_mut().take();
            event.set_allowed_effects(DragDropEffects::NONE);
            let Some(this) = weak.upgrade() else { return };
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
                OutlineDrag::capture(&this.session.borrow(), nodes)
            })();
            match result {
                Ok(snapshot) => {
                    *state.borrow_mut() = Some(snapshot);
                    event.set_allowed_effects(DragDropEffects::MOVE);
                }
                Err(error) => this.status.set_text(Some(&error)),
            }
        });
        let weak = Rc::downgrade(self);
        let state = drag.clone();
        grid.row_drag_over(move |_, event| {
            let allowed = weak.upgrade().is_some_and(|this| {
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
        });
        let weak = Rc::downgrade(self);
        grid.row_drop(move |_, event| {
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
