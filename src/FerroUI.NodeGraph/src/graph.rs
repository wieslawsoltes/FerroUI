//! Complete Rhino Grasshopper [`NodeGraph`] document model with multi-wire ports,
//! cycle detection, colored node groups, undo/redo history, duplicate, align, and auto-layout.

use crate::data_tree::{DataTree, Value};
use crate::node::NodeRegistry;
use crate::types::{
    AlignMode, DataMatching, GroupId, NodeId, NodeKind, NodeState, ParamModifier, ParamWidget,
    WireDisplay, WireId,
};
use indexmap::IndexMap;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

/// A single node placed on the Grasshopper canvas.
#[derive(Clone, Debug)]
pub struct NodeInstance {
    pub id: NodeId,
    pub type_id: String,
    /// Optional user-renamed display title / nickname override.
    pub custom_title: Option<String>,
    /// Canvas-space top-left position in logical pixels.
    pub position: [f64; 2],
    /// Computed or user-resized capsule dimensions `[width, height]`.
    pub size: [f64; 2],
    /// Inline widget value (slider, toggle, color, panel text, expression).
    pub widget: ParamWidget,
    /// Per-input-port Grasshopper modifiers (`Flatten`, `Graft`, `Reverse`, `Simplify`).
    pub input_modifiers: Vec<ParamModifier>,
    /// Per-output-port Grasshopper modifiers.
    pub output_modifiers: Vec<ParamModifier>,
    /// Per-input-port manual default value overrides (when unwired).
    pub input_defaults: Vec<Option<Value>>,
    /// Data-tree lacing strategy (`LongestList`, `ShortestList`, `CrossReference`).
    pub matching: DataMatching,
    /// Whether this node's output is previewed in the target viewport.
    pub preview: bool,
    /// Disabled / bypassed nodes stop downstream execution cleanly.
    pub disabled: bool,
    /// Cached output `DataTree`s produced by the most recent evaluation.
    pub cached_outputs: Vec<DataTree>,
    /// Cached last runtime state (`Normal`, `Warning`, `Error`, `Disabled`).
    pub state: NodeState,
    /// Most recent evaluation time in microseconds (for Grasshopper Profiler badge).
    pub eval_us: u64,
    /// True when inputs, widget, or upstream nodes changed since last evaluation.
    pub dirty: bool,
}

impl NodeInstance {
    pub fn output_tree(&self, port_idx: usize) -> Option<&DataTree> {
        self.cached_outputs.get(port_idx)
    }
}

/// Directed data connection between an output socket and an input socket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wire {
    pub id: WireId,
    pub from_node: NodeId,
    pub from_port: usize,
    pub to_node: NodeId,
    pub to_port: usize,
    pub display: WireDisplay,
}

/// Colored Grasshopper group blob enclosing a set of member nodes.
#[derive(Clone, Debug)]
pub struct NodeGroup {
    pub id: GroupId,
    pub title: String,
    pub color_argb: u32,
    pub members: Vec<NodeId>,
}

/// Lightweight graph snapshot stored in the Undo/Redo stack (excludes transient caches).
#[derive(Clone, Debug)]
struct GraphSnapshot {
    nodes: IndexMap<NodeId, NodeInstance>,
    wires: IndexMap<WireId, Wire>,
    groups: IndexMap<GroupId, NodeGroup>,
    next_node_id: u32,
    next_wire_id: u32,
    next_group_id: u32,
}

/// Complete interactive parametric node graph with undo/redo, groups, selection, and layout tools.
#[derive(Clone, Debug)]
pub struct NodeGraph {
    pub name: String,
    pub nodes: IndexMap<NodeId, NodeInstance>,
    pub wires: IndexMap<WireId, Wire>,
    pub groups: IndexMap<GroupId, NodeGroup>,
    pub selected_nodes: FxHashSet<NodeId>,
    pub selected_wires: FxHashSet<WireId>,
    pub camera_pan: [f64; 2],
    pub camera_zoom: f64,
    pub fancy_wires: bool,
    pub show_profiler: bool,
    pub snap_to_grid: bool,
    pub solution_locked: bool,
    pub generation: u64,
    next_node_id: u32,
    next_wire_id: u32,
    next_group_id: u32,
    undo_stack: Vec<GraphSnapshot>,
    redo_stack: Vec<GraphSnapshot>,
}

impl Default for NodeGraph {
    fn default() -> Self {
        Self::new("Untitled Grasshopper Definition")
    }
}

impl NodeGraph {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: IndexMap::new(),
            wires: IndexMap::new(),
            groups: IndexMap::new(),
            selected_nodes: FxHashSet::default(),
            selected_wires: FxHashSet::default(),
            camera_pan: [40.0, 40.0],
            camera_zoom: 1.0,
            fancy_wires: true,
            show_profiler: true,
            snap_to_grid: false,
            solution_locked: false,
            generation: 0,
            next_node_id: 1,
            next_wire_id: 1,
            next_group_id: 1,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn snapshot(&self) -> GraphSnapshot {
        let mut nodes = self.nodes.clone();
        for n in nodes.values_mut() {
            n.cached_outputs.clear();
            n.dirty = true;
        }
        GraphSnapshot {
            nodes,
            wires: self.wires.clone(),
            groups: self.groups.clone(),
            next_node_id: self.next_node_id,
            next_wire_id: self.next_wire_id,
            next_group_id: self.next_group_id,
        }
    }

    /// Saves a checkpoint on the undo stack before a mutating user gesture.
    pub fn push_undo_checkpoint(&mut self) {
        self.undo_stack.push(self.snapshot());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo_stack.pop() else { return false };
        self.redo_stack.push(self.snapshot());
        self.restore_snapshot(prev);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo_stack.pop() else { return false };
        self.undo_stack.push(self.snapshot());
        self.restore_snapshot(next);
        true
    }

    fn restore_snapshot(&mut self, snap: GraphSnapshot) {
        self.nodes = snap.nodes;
        self.wires = snap.wires;
        self.groups = snap.groups;
        self.next_node_id = snap.next_node_id;
        self.next_wire_id = snap.next_wire_id;
        self.next_group_id = snap.next_group_id;
        self.selected_nodes.retain(|id| self.nodes.contains_key(id));
        self.selected_wires.retain(|id| self.wires.contains_key(id));
        self.mark_all_dirty();
    }

    /// Computes default Grasshopper capsule size from component kind & port count.
    pub fn compute_default_size(registry: &NodeRegistry, type_id: &str) -> [f64; 2] {
        let Some(comp) = registry.get(type_id) else {
            return [150.0, 62.0];
        };
        let in_n = comp.inputs().len();
        let out_n = comp.outputs().len();
        match comp.kind() {
            NodeKind::NumberSlider => [226.0, 56.0],
            NodeKind::BoolToggle => [152.0, 44.0],
            NodeKind::MomentaryButton => [146.0, 44.0],
            NodeKind::Panel => [224.0, 128.0],
            NodeKind::ParamViewer => [216.0, 118.0],
            NodeKind::ColorSwatch => [156.0, 48.0],
            NodeKind::ExpressionBox => [236.0, 88.0],
            NodeKind::GraphMapper => [210.0, 168.0],
            NodeKind::MdSlider => [174.0, 162.0],
            NodeKind::ValueList => [196.0, 66.0],
            NodeKind::PathMapper => [220.0, 74.0],
            NodeKind::Relay => [56.0, 28.0],
            NodeKind::Scribble => [210.0, 42.0],
            NodeKind::Component => {
                let rows = in_n.max(out_n).max(1) as f64;
                let h = (rows * 24.0 + 26.0).max(52.0);
                let name_w = (comp.nickname().len() as f64 * 8.2 + 96.0).clamp(150.0, 232.0);
                [name_w, h]
            }
        }
    }

    /// Adds a node of `type_id` at canvas coordinates `(x, y)`.
    pub fn add_node(&mut self, registry: &NodeRegistry, type_id: &str, x: f64, y: f64) -> Option<NodeId> {
        let comp = registry.get(type_id)?;
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;

        let in_len = comp.inputs().len();
        let out_len = comp.outputs().len();
        let size = Self::compute_default_size(registry, type_id);
        let pos = if self.snap_to_grid {
            [(x / 20.0).round() * 20.0, (y / 20.0).round() * 20.0]
        } else {
            [x, y]
        };

        let instance = NodeInstance {
            id,
            type_id: type_id.to_string(),
            custom_title: None,
            position: pos,
            size,
            widget: comp.default_widget(),
            input_modifiers: vec![ParamModifier::NONE; in_len],
            output_modifiers: vec![ParamModifier::NONE; out_len],
            input_defaults: vec![None; in_len],
            matching: DataMatching::LongestList,
            preview: true,
            disabled: false,
            cached_outputs: vec![DataTree::new(); out_len],
            state: NodeState::Normal,
            eval_us: 0,
            dirty: true,
        };
        self.nodes.insert(id, instance);
        self.generation += 1;
        Some(id)
    }

    /// Convenience builder for programmatic/template graph construction: sets slider parameters.
    pub fn configure_slider(
        &mut self,
        id: NodeId,
        title: impl Into<String>,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        integer: bool,
    ) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.custom_title = Some(title.into());
            n.widget = ParamWidget::Slider {
                value: value.clamp(min, max),
                min,
                max,
                step,
                integer,
            };
            n.dirty = true;
            self.generation += 1;
        }
    }

    pub fn configure_toggle(&mut self, id: NodeId, title: impl Into<String>, value: bool) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.custom_title = Some(title.into());
            n.widget = ParamWidget::Toggle { value };
            n.dirty = true;
            self.generation += 1;
        }
    }

    pub fn configure_text(&mut self, id: NodeId, title: impl Into<String>, text: impl Into<String>) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.custom_title = Some(title.into());
            n.widget = ParamWidget::Text { text: text.into() };
            n.dirty = true;
            self.generation += 1;
        }
    }

    pub fn configure_color(&mut self, id: NodeId, title: impl Into<String>, argb: u32) {
        if let Some(n) = self.nodes.get_mut(&id) {
            n.custom_title = Some(title.into());
            n.widget = ParamWidget::Color { argb };
            n.dirty = true;
            self.generation += 1;
        }
    }

    pub fn set_input_default(&mut self, id: NodeId, port_idx: usize, value: Value) {
        if let Some(n) = self.nodes.get_mut(&id) {
            if port_idx < n.input_defaults.len() {
                n.input_defaults[port_idx] = Some(value);
                self.mark_dirty(id);
            }
        }
    }

    pub fn set_input_modifier(&mut self, id: NodeId, port_idx: usize, modifier: ParamModifier) {
        if let Some(n) = self.nodes.get_mut(&id) {
            if port_idx < n.input_modifiers.len() {
                n.input_modifiers[port_idx] = modifier;
                self.mark_dirty(id);
            }
        }
    }

    /// Removes a node and all wires attached to it.
    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if self.nodes.shift_remove(&id).is_none() {
            return false;
        }
        let affected_downstream: Vec<NodeId> = self
            .wires
            .values()
            .filter(|w| w.from_node == id)
            .map(|w| w.to_node)
            .collect();
        self.wires.retain(|_, w| w.from_node != id && w.to_node != id);
        for g in self.groups.values_mut() {
            g.members.retain(|&m| m != id);
        }
        self.groups.retain(|_, g| !g.members.is_empty());
        self.selected_nodes.remove(&id);
        for dn in affected_downstream {
            self.mark_dirty(dn);
        }
        self.generation += 1;
        true
    }

    /// Connects `(from_node, from_port) -> (to_node, to_port)`.
    /// If `multi_wire` is false (default Grasshopper drag), replaces existing wires on `(to_node, to_port)`.
    /// If `multi_wire` is true (Shift-drag), appends an additional wire into the input stream!
    /// Rejects self-loops and cycles automatically.
    pub fn connect(
        &mut self,
        from_node: NodeId,
        from_port: usize,
        to_node: NodeId,
        to_port: usize,
        multi_wire: bool,
    ) -> Result<WireId, &'static str> {
        if from_node == to_node {
            return Err("Cannot connect a node to itself");
        }
        if !self.nodes.contains_key(&from_node) || !self.nodes.contains_key(&to_node) {
            return Err("Unknown node id");
        }
        // Duplicate check
        if let Some(existing) = self.wires.values().find(|w| {
            w.from_node == from_node
                && w.from_port == from_port
                && w.to_node == to_node
                && w.to_port == to_port
        }) {
            return Ok(existing.id);
        }
        // Cycle check: would adding from_node -> to_node make from_node reachable from to_node?
        if self.is_reachable(to_node, from_node) {
            return Err("Connection would create a cycle in the DAG");
        }

        if !multi_wire {
            self.wires.retain(|_, w| !(w.to_node == to_node && w.to_port == to_port));
        }

        let id = WireId(self.next_wire_id);
        self.next_wire_id += 1;
        self.wires.insert(
            id,
            Wire {
                id,
                from_node,
                from_port,
                to_node,
                to_port,
                display: WireDisplay::Default,
            },
        );
        self.mark_dirty(to_node);
        self.generation += 1;
        Ok(id)
    }

    pub fn disconnect_wire(&mut self, wire_id: WireId) -> bool {
        if let Some(w) = self.wires.shift_remove(&wire_id) {
            self.selected_wires.remove(&wire_id);
            self.mark_dirty(w.to_node);
            self.generation += 1;
            true
        } else {
            false
        }
    }

    pub fn disconnect_input_port(&mut self, node: NodeId, port: usize) -> usize {
        let before = self.wires.len();
        self.wires.retain(|_, w| !(w.to_node == node && w.to_port == port));
        let removed = before - self.wires.len();
        if removed > 0 {
            self.mark_dirty(node);
            self.generation += 1;
        }
        removed
    }

    pub fn incoming_wires(&self, node: NodeId, port: usize) -> Vec<&Wire> {
        self.wires
            .values()
            .filter(|w| w.to_node == node && w.to_port == port)
            .collect()
    }

    pub fn outgoing_wires(&self, node: NodeId, port: usize) -> Vec<&Wire> {
        self.wires
            .values()
            .filter(|w| w.from_node == node && w.from_port == port)
            .collect()
    }

    /// Checks whether `target` can be reached from `start` following directed wires.
    pub fn is_reachable(&self, start: NodeId, target: NodeId) -> bool {
        let mut visited = FxHashSet::default();
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(cur) = queue.pop_front() {
            if cur == target {
                return true;
            }
            if !visited.insert(cur) {
                continue;
            }
            for w in self.wires.values() {
                if w.from_node == cur && !visited.contains(&w.to_node) {
                    queue.push_back(w.to_node);
                }
            }
        }
        false
    }

    /// Marks `root` and all downstream dependent nodes dirty for incremental re-evaluation.
    pub fn mark_dirty(&mut self, root: NodeId) {
        let mut queue = VecDeque::new();
        let mut visited = FxHashSet::default();
        queue.push_back(root);
        while let Some(cur) = queue.pop_front() {
            if !visited.insert(cur) {
                continue;
            }
            if let Some(n) = self.nodes.get_mut(&cur) {
                n.dirty = true;
            }
            for w in self.wires.values() {
                if w.from_node == cur && !visited.contains(&w.to_node) {
                    queue.push_back(w.to_node);
                }
            }
        }
        self.generation += 1;
    }

    pub fn mark_all_dirty(&mut self) {
        for n in self.nodes.values_mut() {
            n.dirty = true;
        }
        self.generation += 1;
    }

    /// Creates a colored Grasshopper Group container around `members`.
    pub fn create_group(&mut self, title: impl Into<String>, color_argb: u32, members: Vec<NodeId>) -> GroupId {
        let id = GroupId(self.next_group_id);
        self.next_group_id += 1;
        let members: Vec<NodeId> = members.into_iter().filter(|m| self.nodes.contains_key(m)).collect();
        self.groups.insert(id, NodeGroup { id, title: title.into(), color_argb, members });
        self.generation += 1;
        id
    }

    /// Groups the current selection into a colored Grasshopper blob.
    pub fn group_selection(&mut self, title: impl Into<String>, color_argb: u32) -> Option<GroupId> {
        if self.selected_nodes.is_empty() {
            return None;
        }
        self.push_undo_checkpoint();
        let members: Vec<NodeId> = self.selected_nodes.iter().copied().collect();
        Some(self.create_group(title, color_argb, members))
    }

    /// Deletes all selected nodes and selected wires with undo checkpoint.
    pub fn delete_selection(&mut self) -> bool {
        if self.selected_nodes.is_empty() && self.selected_wires.is_empty() {
            return false;
        }
        self.push_undo_checkpoint();
        let wires: Vec<WireId> = self.selected_wires.drain().collect();
        for wid in wires {
            self.disconnect_wire(wid);
        }
        let nodes: Vec<NodeId> = self.selected_nodes.drain().collect();
        for nid in nodes {
            self.remove_node(nid);
        }
        true
    }

    /// Duplicates all selected nodes (and internal wires between them) offset by `(40, 40)`.
    pub fn duplicate_selection(&mut self) -> Vec<NodeId> {
        if self.selected_nodes.is_empty() {
            return Vec::new();
        }
        self.push_undo_checkpoint();
        let mut remap: FxHashMap<NodeId, NodeId> = FxHashMap::default();
        let old_ids: Vec<NodeId> = self.selected_nodes.iter().copied().collect();
        let mut new_ids = Vec::with_capacity(old_ids.len());

        for old_id in &old_ids {
            if let Some(orig) = self.nodes.get(old_id).cloned() {
                let nid = NodeId(self.next_node_id);
                self.next_node_id += 1;
                let mut copy = orig;
                copy.id = nid;
                copy.position[0] += 44.0;
                copy.position[1] += 44.0;
                copy.dirty = true;
                self.nodes.insert(nid, copy);
                remap.insert(*old_id, nid);
                new_ids.push(nid);
            }
        }

        let internal_wires: Vec<Wire> = self
            .wires
            .values()
            .filter(|w| remap.contains_key(&w.from_node) && remap.contains_key(&w.to_node))
            .cloned()
            .collect();
        for w in internal_wires {
            let _ = self.connect(
                remap[&w.from_node],
                w.from_port,
                remap[&w.to_node],
                w.to_port,
                true,
            );
        }

        self.selected_nodes.clear();
        for &nid in &new_ids {
            self.selected_nodes.insert(nid);
        }
        self.generation += 1;
        new_ids
    }

    /// Toggles `disabled` flag on all selected nodes (`Ctrl+E` in Grasshopper).
    pub fn toggle_disable_selection(&mut self) {
        if self.selected_nodes.is_empty() {
            return;
        }
        self.push_undo_checkpoint();
        let ids: Vec<NodeId> = self.selected_nodes.iter().copied().collect();
        for id in ids {
            if let Some(n) = self.nodes.get_mut(&id) {
                n.disabled = !n.disabled;
            }
            self.mark_dirty(id);
        }
    }

    /// Inserts a Grasshopper **Relay** node inline on an existing wire (double-click wire!).
    pub fn insert_relay_on_wire(
        &mut self,
        wire_id: WireId,
        x: f64,
        y: f64,
        registry: &NodeRegistry,
    ) -> Option<NodeId> {
        let orig = self.wires.get(&wire_id).cloned()?;
        self.push_undo_checkpoint();
        let relay = self.add_node(registry, "params.relay", x - 28.0, y - 14.0)?;
        self.disconnect_wire(wire_id);
        let _ = self.connect(orig.from_node, orig.from_port, relay, 0, true);
        let _ = self.connect(relay, 0, orig.to_node, orig.to_port, true);
        self.selected_nodes.clear();
        self.selected_nodes.insert(relay);
        Some(relay)
    }

    /// Grasshopper **Rewire** (`Ctrl+Shift`-drag): moves all incoming wires from `(old_node, old_port)`
    /// onto `(new_node, new_port)` cleanly.
    pub fn rewire_input(
        &mut self,
        old_node: NodeId,
        old_port: usize,
        new_node: NodeId,
        new_port: usize,
    ) -> usize {
        let sources: Vec<(WireId, NodeId, usize)> = self
            .wires
            .values()
            .filter(|w| w.to_node == old_node && w.to_port == old_port)
            .map(|w| (w.id, w.from_node, w.from_port))
            .collect();
        if sources.is_empty() {
            return 0;
        }
        self.push_undo_checkpoint();
        let mut moved = 0;
        for (wid, fn_id, fp) in sources {
            if self.connect(fn_id, fp, new_node, new_port, true).is_ok() {
                self.disconnect_wire(wid);
                moved += 1;
            }
        }
        moved
    }

    /// Sets Grasshopper wire display style (`Default`, `Faint`, `Hidden`) on all incoming wires of `node`.
    pub fn set_node_wire_display(&mut self, node: NodeId, display: WireDisplay) {
        self.push_undo_checkpoint();
        for w in self.wires.values_mut() {
            if w.to_node == node {
                w.display = display;
            }
        }
        self.generation += 1;
    }

    /// Aligns or evenly distributes all currently selected nodes (Grasshopper multi-select align!).
    pub fn align_selection(&mut self, mode: AlignMode) -> bool {
        if self.selected_nodes.len() < 2 {
            return false;
        }
        self.push_undo_checkpoint();
        let mut ids: Vec<NodeId> = self.selected_nodes.iter().copied().collect();
        match mode {
            AlignMode::Left => {
                let target = ids.iter().filter_map(|id| self.nodes.get(id).map(|n| n.position[0])).fold(f64::INFINITY, f64::min);
                for id in ids {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[0] = target; }
                }
            }
            AlignMode::Right => {
                let target = ids.iter().filter_map(|id| self.nodes.get(id).map(|n| n.position[0] + n.size[0])).fold(f64::NEG_INFINITY, f64::max);
                for id in ids {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[0] = target - n.size[0]; }
                }
            }
            AlignMode::Top => {
                let target = ids.iter().filter_map(|id| self.nodes.get(id).map(|n| n.position[1])).fold(f64::INFINITY, f64::min);
                for id in ids {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[1] = target; }
                }
            }
            AlignMode::Bottom => {
                let target = ids.iter().filter_map(|id| self.nodes.get(id).map(|n| n.position[1] + n.size[1])).fold(f64::NEG_INFINITY, f64::max);
                for id in ids {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[1] = target - n.size[1]; }
                }
            }
            AlignMode::CenterHorizontal => {
                let sum: f64 = ids.iter().filter_map(|id| self.nodes.get(id).map(|n| n.position[0] + n.size[0] * 0.5)).sum();
                let mid = sum / ids.len() as f64;
                for id in ids {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[0] = mid - n.size[0] * 0.5; }
                }
            }
            AlignMode::CenterVertical => {
                let sum: f64 = ids.iter().filter_map(|id| self.nodes.get(id).map(|n| n.position[1] + n.size[1] * 0.5)).sum();
                let mid = sum / ids.len() as f64;
                for id in ids {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[1] = mid - n.size[1] * 0.5; }
                }
            }
            AlignMode::DistributeHorizontal => {
                if ids.len() < 3 { return false; }
                ids.sort_by(|a, b| self.nodes[a].position[0].total_cmp(&self.nodes[b].position[0]));
                let x0 = self.nodes[&ids[0]].position[0];
                let x1 = self.nodes[&ids[ids.len() - 1]].position[0];
                let step = (x1 - x0) / (ids.len() - 1) as f64;
                for (i, id) in ids.into_iter().enumerate() {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[0] = x0 + step * i as f64; }
                }
            }
            AlignMode::DistributeVertical => {
                if ids.len() < 3 { return false; }
                ids.sort_by(|a, b| self.nodes[a].position[1].total_cmp(&self.nodes[b].position[1]));
                let y0 = self.nodes[&ids[0]].position[1];
                let y1 = self.nodes[&ids[ids.len() - 1]].position[1];
                let step = (y1 - y0) / (ids.len() - 1) as f64;
                for (i, id) in ids.into_iter().enumerate() {
                    if let Some(n) = self.nodes.get_mut(&id) { n.position[1] = y0 + step * i as f64; }
                }
            }
        }
        self.generation += 1;
        true
    }

    /// Sugiyama-style layered topological auto-layout arranging nodes cleanly left-to-right.
    pub fn auto_layout(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        self.push_undo_checkpoint();

        let mut in_deg: FxHashMap<NodeId, usize> = self.nodes.keys().map(|&id| (id, 0)).collect();
        let mut adj: FxHashMap<NodeId, Vec<NodeId>> = FxHashMap::default();
        for w in self.wires.values() {
            adj.entry(w.from_node).or_default().push(w.to_node);
            *in_deg.entry(w.to_node).or_default() += 1;
        }

        let mut rank: FxHashMap<NodeId, usize> = FxHashMap::default();
        let mut q: VecDeque<NodeId> = in_deg
            .iter()
            .filter_map(|(&id, &d)| if d == 0 { Some(id) } else { None })
            .collect();

        while let Some(u) = q.pop_front() {
            let r_u = *rank.get(&u).unwrap_or(&0);
            if let Some(nbrs) = adj.get(&u) {
                for &v in nbrs {
                    let e = rank.entry(v).or_insert(0);
                    *e = (*e).max(r_u + 1);
                    let d = in_deg.get_mut(&v).unwrap();
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        q.push_back(v);
                    }
                }
            }
        }

        let mut columns: Vec<Vec<NodeId>> = Vec::new();
        for &id in self.nodes.keys() {
            let c = *rank.get(&id).unwrap_or(&0);
            if columns.len() <= c {
                columns.resize_with(c + 1, Vec::new);
            }
            columns[c].push(id);
        }

        let mut x = 50.0;
        for col in columns {
            let mut y = 50.0;
            let mut max_w = 160.0_f64;
            for id in col {
                if let Some(n) = self.nodes.get_mut(&id) {
                    n.position = [x, y];
                    y += n.size[1] + 34.0;
                    max_w = max_w.max(n.size[0]);
                }
            }
            x += max_w + 86.0;
        }
        self.generation += 1;
    }

    /// Computes axis-aligned bounding box `[min_x, min_y, max_x, max_y]` of all nodes in canvas space.
    pub fn content_bounds(&self) -> Option<[f64; 4]> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        for n in self.nodes.values() {
            min_x = min_x.min(n.position[0]);
            min_y = min_y.min(n.position[1]);
            max_x = max_x.max(n.position[0] + n.size[0]);
            max_y = max_y.max(n.position[1] + n.size[1]);
        }
        Some([min_x, min_y, max_x, max_y])
    }

    /// Adjusts `camera_pan` and `camera_zoom` to frame all nodes inside a viewport of `(vw, vh)`.
    pub fn fit_to_view(&mut self, vw: f64, vh: f64) {
        let Some([min_x, min_y, max_x, max_y]) = self.content_bounds() else {
            self.camera_pan = [40.0, 40.0];
            self.camera_zoom = 1.0;
            return;
        };
        let pad = 64.0;
        let cw = (max_x - min_x).max(120.0);
        let ch = (max_y - min_y).max(100.0);
        let zoom = ((vw - pad * 2.0) / cw).min((vh - pad * 2.0) / ch).clamp(0.35, 1.35);
        self.camera_zoom = zoom;
        self.camera_pan = [
            (vw - cw * zoom) * 0.5 - min_x * zoom,
            (vh - ch * zoom) * 0.5 - min_y * zoom,
        ];
    }

    /// Computes bounding box `[min_x, min_y, max_x, max_y]` for a colored Grasshopper group.
    pub fn group_bounds(&self, group: &NodeGroup) -> Option<[f64; 4]> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut any = false;
        for id in &group.members {
            if let Some(n) = self.nodes.get(id) {
                any = true;
                min_x = min_x.min(n.position[0]);
                min_y = min_y.min(n.position[1]);
                max_x = max_x.max(n.position[0] + n.size[0]);
                max_y = max_y.max(n.position[1] + n.size[1]);
            }
        }
        if any {
            Some([min_x - 18.0, min_y - 30.0, max_x + 18.0, max_y + 18.0])
        } else {
            None
        }
    }
}
