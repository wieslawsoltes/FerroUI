//! Local layer presentation and canvas intent. Locks, visibility and isolation are editor
//! state: they never write IsVisible, Opacity, markup, undo history or collaboration data.
//! Source inspectors remain able to select and intentionally edit a locked/hidden element.
use crate::DesignerSession;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElement;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Ref, Visual, WeakRef};
use ferroui_designer_core::{Ast, NodeId, NodeKind};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

/// Source-row presentation. Direct flags can be changed on this row; inherited flags
/// require changing their ancestor. Isolation preserves visible ancestor context.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LayerState {
    pub locked: bool,
    pub hidden: bool,
    pub inherited_locked: bool,
    pub inherited_hidden: bool,
    pub outside_isolation: bool,
}
impl LayerState {
    pub fn is_locked(self) -> bool {
        self.locked || self.inherited_locked
    }
    pub fn is_hidden(self) -> bool {
        self.hidden || self.inherited_hidden
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayerCommand {
    ToggleLock,
    ToggleHidden,
    Isolate,
    ExitIsolation,
    UnlockAll,
    ShowAll,
    SelectParent,
    SelectChildren,
    SelectNextSibling,
    SelectPreviousSibling,
}

#[derive(Clone, Default)]
struct LayerFlags {
    locked: BTreeSet<NodeId>,
    hidden: BTreeSet<NodeId>,
    // Each definition scope retains its own isolation while the document scope is open.
    isolation: BTreeMap<Option<NodeId>, Vec<NodeId>>,
}

/// A tab retains this alongside its Document clone. It restores only into the exact
/// same syntax allocation and URI, never a new document that happens to reuse NodeIds.
#[derive(Clone)]
pub struct LayerViewSnapshot {
    uri: String,
    syntax: Arc<Ast>,
    flags: LayerFlags,
}

/// Immutable source-row/host intent. A lock/unlock round trip invalidates a retained
/// callback just as a source, document or definition-scope transition does.
#[derive(Clone)]
pub struct LayerEditSnapshot {
    uri: String,
    revision: u64,
    syntax: Arc<Ast>,
    scope_epoch: u64,
    layer_epoch: u64,
    nodes: Vec<NodeId>,
}

/// A counted render suppression leaves layout and every authored property intact.
/// Weak references let replaced native trees die immediately. Drop balances our request
/// without releasing another presentation owner's suppression.
struct LayerVisualMask {
    visual: WeakRef<Visual>,
    hit_test: Option<Rc<dyn IDisposable>>,
}
impl LayerVisualMask {
    fn new(visual: &Ref<Visual>) -> Self {
        visual.add_render_suppression_request();
        let hit_test = visual.cast::<InputElement>().and_then(|input| {
            input.set_value_with_priority(
                InputElement::is_hit_test_visible_property(),
                false,
                BindingPriority::Animation,
            )
        });
        Self {
            visual: visual.downgrade(),
            hit_test,
        }
    }
}
impl Drop for LayerVisualMask {
    fn drop(&mut self) {
        if let Some(visual) = self.visual.upgrade() {
            visual.remove_render_suppression_request();
        }
        if let Some(binding) = self.hit_test.take() {
            binding.dispose();
        }
    }
}

#[derive(Default)]
pub(crate) struct LayerView {
    flags: LayerFlags,
    epoch: u64,
    masks: BTreeMap<NodeId, LayerVisualMask>,
}
impl LayerView {
    pub(crate) fn reset(&mut self) {
        self.masks.clear();
        self.flags = LayerFlags::default();
        self.epoch = self.epoch.wrapping_add(1);
    }
}

impl DesignerSession {
    pub fn layer_view_epoch(&self) -> u64 {
        self.layers.epoch
    }

    pub fn layer_view_snapshot(&self) -> LayerViewSnapshot {
        LayerViewSnapshot {
            uri: self.document_uri().to_owned(),
            syntax: self.document.ast_snapshot(),
            flags: self.layers.flags.clone(),
        }
    }

    pub fn restore_layer_view(&mut self, snapshot: &LayerViewSnapshot) -> bool {
        if snapshot.uri != self.document_uri()
            || !Arc::ptr_eq(&snapshot.syntax, &self.document.ast_snapshot())
        {
            return false;
        }
        self.layers.flags = snapshot.flags.clone();
        self.layer_view_changed();
        true
    }

    pub fn reset_layer_view(&mut self) {
        self.layers.reset();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Current scope's isolated roots, in source order. Empty means normal editing.
    pub fn layer_isolation(&self) -> &[NodeId] {
        self.layers
            .flags
            .isolation
            .get(&self.definition_scope().map(|scope| scope.node))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn layer_state(&self, node: NodeId) -> LayerState {
        let mut result = LayerState {
            locked: self.layers.flags.locked.contains(&node),
            hidden: self.layers.flags.hidden.contains(&node),
            ..Default::default()
        };
        let mut parent = self.document.ast().node(node).and_then(|node| node.parent);
        while let Some(id) = parent {
            result.inherited_locked |= self.layers.flags.locked.contains(&id);
            result.inherited_hidden |= self.layers.flags.hidden.contains(&id);
            parent = self.document.ast().node(id).and_then(|node| node.parent);
        }
        let isolated = self.layer_isolation();
        result.outside_isolation = !isolated.is_empty()
            && !isolated.iter().any(|root| {
                descendant_of(self.document.ast(), node, *root)
                    || descendant_of(self.document.ast(), *root, node)
            });
        result
    }

    /// Canvas selection respects ancestor locks/visibility and the active definition and
    /// isolation scopes. The source Layers inspector deliberately uses ordinary select().
    pub fn can_canvas_select(&self, node: NodeId) -> bool {
        self.ensure_layer_manipulable(node).is_ok() && self.preview.visual(node).is_some()
    }

    /// Shared guard for canvas, arrangement, hierarchy moves and layout extensions.
    /// This is presentation protection, not an authorization check: callers still use
    /// the normal writable/revision/preview checks before accepting authored changes.
    pub fn ensure_layer_manipulable(&self, node: NodeId) -> Result<(), String> {
        if !self
            .document
            .ast()
            .node(node)
            .is_some_and(|node| node.kind == NodeKind::Element)
        {
            return Err("Select an existing authored control layer".into());
        }
        let state = self.layer_state(node);
        if state.is_locked() {
            return Err("Unlock this layer or its ancestor before manipulating it".into());
        }
        if state.is_hidden() {
            return Err("Show this layer or its ancestor before manipulating it".into());
        }
        if let Some(scope) = self.definition_scope() {
            if !descendant_of(self.document.ast(), node, scope.node) {
                return Err("The layer is outside the active definition scope".into());
            }
        }
        let isolated = self.layer_isolation();
        if !isolated.is_empty()
            && !isolated
                .iter()
                .any(|root| descendant_of(self.document.ast(), node, *root))
        {
            return Err("Exit layer isolation before manipulating another layer".into());
        }
        Ok(())
    }

    /// Existing isolated children still use their original parent's coordinate space and
    /// insertion boundaries. That context does not grant permission to insert new content
    /// or reparent another branch into an owner outside the isolation/definition scope.
    pub(crate) fn ensure_layer_container_context(
        &self,
        owner: NodeId,
        moving: &[NodeId],
    ) -> Result<(), String> {
        if self.ensure_layer_manipulable(owner).is_ok() {
            return Ok(());
        }
        let state = self.layer_state(owner);
        if !state.is_locked()
            && !state.is_hidden()
            && !moving.is_empty()
            && moving.iter().all(|node| {
                self.ensure_layer_manipulable(*node).is_ok()
                    && crate::workspace_tools::parent(self, *node).ok() == Some(owner)
            })
        {
            return Ok(());
        }
        self.ensure_layer_manipulable(owner)
    }

    pub fn set_layers_locked(&mut self, nodes: &[NodeId], locked: bool) -> Result<(), String> {
        self.validate_layer_targets(nodes)?;
        let mut changed = false;
        for node in nodes {
            changed |= if locked {
                self.layers.flags.locked.insert(*node)
            } else {
                self.layers.flags.locked.remove(node)
            };
        }
        if changed {
            self.layer_view_changed();
        }
        Ok(())
    }

    pub fn set_layers_hidden(&mut self, nodes: &[NodeId], hidden: bool) -> Result<(), String> {
        self.validate_layer_targets(nodes)?;
        let mut changed = false;
        for node in nodes {
            changed |= if hidden {
                self.layers.flags.hidden.insert(*node)
            } else {
                self.layers.flags.hidden.remove(node)
            };
        }
        if changed {
            self.layer_view_changed();
        }
        Ok(())
    }

    pub fn isolate_layers(&mut self, nodes: &[NodeId]) -> Result<(), String> {
        self.validate_layer_targets(nodes)?;
        if nodes.is_empty() {
            return Err("Select one or more layers to isolate".into());
        }
        for node in nodes {
            let state = self.layer_state(*node);
            if state.is_hidden() {
                return Err("Show the selected layers before isolating them".into());
            }
            if let Some(scope) = self.definition_scope() {
                if !descendant_of(self.document.ast(), *node, scope.node) {
                    return Err("Isolate layers within the active definition scope".into());
                }
            }
        }
        let selected = nodes.iter().copied().collect::<BTreeSet<_>>();
        let roots = self
            .document
            .ast()
            .nodes_in_source_order()
            .filter(|node| {
                selected.contains(&node.id)
                    && !selected.iter().any(|other| {
                        *other != node.id && descendant_of(self.document.ast(), node.id, *other)
                    })
            })
            .map(|node| node.id)
            .collect::<Vec<_>>();
        let scope = self.definition_scope().map(|scope| scope.node);
        if self.layers.flags.isolation.get(&scope) != Some(&roots) {
            self.layers.flags.isolation.insert(scope, roots);
            self.layer_view_changed();
        }
        Ok(())
    }

    pub fn clear_layer_isolation(&mut self) {
        let scope = self.definition_scope().map(|scope| scope.node);
        if self.layers.flags.isolation.remove(&scope).is_some() {
            self.layer_view_changed();
        }
    }

    pub fn capture_layer_edit(&self, nodes: &[NodeId]) -> Result<LayerEditSnapshot, String> {
        self.validate_layer_targets(nodes)?;
        Ok(LayerEditSnapshot {
            uri: self.document_uri().to_owned(),
            revision: self.document.revision(),
            syntax: self.document.ast_snapshot(),
            scope_epoch: self.definition_scope_epoch(),
            layer_epoch: self.layer_view_epoch(),
            nodes: nodes.to_vec(),
        })
    }

    pub fn apply_layer_edit(
        &mut self,
        snapshot: &LayerEditSnapshot,
        command: LayerCommand,
    ) -> Result<(), String> {
        if snapshot.uri != self.document_uri()
            || snapshot.revision != self.document.revision()
            || !Arc::ptr_eq(&snapshot.syntax, &self.document.ast_snapshot())
            || snapshot.scope_epoch != self.definition_scope_epoch()
            || snapshot.layer_epoch != self.layer_view_epoch()
        {
            return Err("Layer view changed; use the current Layers row or selection".into());
        }
        self.apply_layer_command_to(&snapshot.nodes, command)
    }

    pub fn apply_layer_command(&mut self, command: LayerCommand) -> Result<(), String> {
        self.apply_layer_command_to(&self.selection.clone(), command)
    }

    fn apply_layer_command_to(
        &mut self,
        nodes: &[NodeId],
        command: LayerCommand,
    ) -> Result<(), String> {
        match command {
            LayerCommand::ToggleLock | LayerCommand::ToggleHidden if nodes.is_empty() => {
                Err("Select one or more authored layers".into())
            }
            LayerCommand::ToggleLock => {
                let locked = !nodes
                    .iter()
                    .all(|node| self.layers.flags.locked.contains(node));
                self.set_layers_locked(nodes, locked)
            }
            LayerCommand::ToggleHidden => {
                let hidden = !nodes
                    .iter()
                    .all(|node| self.layers.flags.hidden.contains(node));
                self.set_layers_hidden(nodes, hidden)
            }
            LayerCommand::Isolate => self.isolate_layers(nodes),
            LayerCommand::ExitIsolation => {
                self.clear_layer_isolation();
                Ok(())
            }
            LayerCommand::UnlockAll => {
                if !self.layers.flags.locked.is_empty() {
                    self.layers.flags.locked.clear();
                    self.layer_view_changed();
                }
                Ok(())
            }
            LayerCommand::ShowAll => {
                if !self.layers.flags.hidden.is_empty() {
                    self.layers.flags.hidden.clear();
                    self.layer_view_changed();
                }
                Ok(())
            }
            _ => self.navigate_layers(nodes, command),
        }
    }

    fn validate_layer_targets(&self, nodes: &[NodeId]) -> Result<(), String> {
        if !self.document.is_valid() {
            return Err("Repair the XAML source before changing its layer view".into());
        }
        if nodes.iter().any(|id| {
            !self
                .document
                .ast()
                .node(*id)
                .is_some_and(|node| node.kind == NodeKind::Element)
        }) {
            return Err("Layer actions require existing authored elements".into());
        }
        Ok(())
    }

    fn layer_view_changed(&mut self) {
        self.layers.epoch = self.layers.epoch.wrapping_add(1);
        self.refresh_layer_visuals();
        self.generation = self.generation.wrapping_add(1);
    }

    /// Reconcile after accepted preview replacement, source-map refresh or scope change.
    /// Unchanged visual masks stay alive; counted suppression is never stacked each frame.
    pub(crate) fn refresh_layer_visuals(&mut self) {
        // Source deletion of an isolated root must not leave the entire canvas blank.
        // Lock/hidden tombstones are retained for undo, but an empty isolation retires.
        let ast = self.document.ast();
        let before = self.layers.flags.isolation.clone();
        self.layers.flags.isolation.retain(|scope, roots| {
            roots.retain(|node| {
                ast.node(*node).is_some()
                    && scope.is_none_or(|scope| descendant_of(ast, *node, scope))
            });
            !roots.is_empty()
        });
        if before != self.layers.flags.isolation {
            self.layers.epoch = self.layers.epoch.wrapping_add(1);
        }
        let mut desired = BTreeMap::<NodeId, Ref<Visual>>::new();
        let mut visual_ids = BTreeSet::new();
        for node in self.document.ast().elements() {
            let state = self.layer_state(node.id);
            if !state.is_hidden() && !state.outside_isolation {
                continue;
            }
            let Some(visual) = self.preview.visual(node.id) else {
                continue;
            };
            let address = (&*visual as *const Visual) as usize;
            if visual_ids.insert(address) {
                desired.insert(node.id, visual);
            }
        }
        self.layers.masks.retain(|node, mask| {
            mask.visual.upgrade().is_some_and(|visual| {
                desired
                    .get(node)
                    .is_some_and(|current| std::ptr::eq::<Visual>(&*visual, &**current))
            })
        });
        for (node, visual) in desired {
            self.layers
                .masks
                .entry(node)
                .or_insert_with(|| LayerVisualMask::new(&visual));
        }
    }

    fn canvas_parent(&self, node: NodeId) -> Option<NodeId> {
        let mut parent = self.document.ast().node(node)?.parent;
        while let Some(id) = parent {
            if self.can_canvas_select(id) {
                return Some(id);
            }
            parent = self.document.ast().node(id)?.parent;
        }
        None
    }

    fn canvas_children(&self, parent: NodeId) -> Vec<NodeId> {
        self.document
            .ast()
            .nodes_in_source_order()
            .filter(|node| {
                node.id != parent
                    && self.can_canvas_select(node.id)
                    && self.canvas_parent(node.id) == Some(parent)
            })
            .map(|node| node.id)
            .collect()
    }

    fn navigate_layers(&mut self, nodes: &[NodeId], command: LayerCommand) -> Result<(), String> {
        let first = nodes
            .first()
            .copied()
            .ok_or("Select a layer before navigating")?;
        self.validate_layer_targets(nodes)?;
        let target = match command {
            LayerCommand::SelectParent => self.canvas_parent(first).into_iter().collect(),
            LayerCommand::SelectChildren => nodes
                .iter()
                .flat_map(|node| self.canvas_children(*node))
                .collect(),
            LayerCommand::SelectNextSibling | LayerCommand::SelectPreviousSibling => {
                let parent = self
                    .canvas_parent(first)
                    .ok_or("This layer has no selectable parent")?;
                let siblings = self.canvas_children(parent);
                let index = siblings
                    .iter()
                    .position(|node| *node == first)
                    .ok_or("The selected layer is not available in the canvas")?;
                let next = if command == LayerCommand::SelectNextSibling {
                    (index + 1) % siblings.len()
                } else {
                    (index + siblings.len() - 1) % siblings.len()
                };
                vec![siblings[next]]
            }
            _ => return Err("Unsupported layer navigation command".into()),
        };
        if target.is_empty() {
            return Err("There is no selectable layer in that direction".into());
        }
        self.select(target);
        Ok(())
    }
}

fn descendant_of(ast: &Ast, node: NodeId, ancestor: NodeId) -> bool {
    let mut current = Some(node);
    while let Some(id) = current {
        if id == ancestor {
            return true;
        }
        current = ast.node(id).and_then(|node| node.parent);
    }
    false
}
