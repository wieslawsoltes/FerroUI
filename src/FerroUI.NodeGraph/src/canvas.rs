//! **Visual-Tree Grasshopper Canvas Coordination & Hardware Vector Wire/Grid Layer**.
//!
//! Every node, port socket, slider, toggle, expression box, panel, GraphMapper, and MD Slider
//! is built from **real FerroUI UI Visuals** hosted inside a GPU-transformed `Canvas` (`MatrixTransform`).
//! [`WireBackdropLayer`] sits directly behind that visual tree and renders:
//! - Infinite adaptive Rhino Grasshopper minor/major grid
//! - Colored Grasshopper Node Groups measured directly from mounted real node visuals
//! - **Fancy Bezier Wires** whose endpoints are queried pixel-perfectly from the **real FerroUI port
//!   socket `Visual`s** via `Visual::translate_point`!
//! - Live drag-to-connect Bezier preview wire with magnetic socket snapping & compatibility glow
//! - Marquee box selection, interactive bottom-right Minimap, and double-click wire Relay insertion!

use crate::eval::evaluate_graph;
use crate::graph::{NodeGraph, NodeInstance};
use crate::node::NodeRegistry;
use crate::types::{
    NodeId, PortDirection, WireDataKind, WireDisplay, WireId,
};
use ferroui_base::input::{
    InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutableDashStyle, ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, IImmutableBrush, IPen,
    MatrixTransform, PenLineCap, StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point,
    Rect, Ref, StyledElementImpl, Visual, VisualImpl, WeakRef,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_view::prelude::*;
use ferroui_view::ViewClass;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// Live drag-to-wire gesture initiated from any real FerroUI port socket visual.
#[derive(Clone, Debug)]
pub struct ActiveWireDrag {
    pub from_node: NodeId,
    pub from_port: usize,
    pub from_dir: PortDirection,
    pub cursor_screen: Point,
    pub multi_wire: bool,
    pub snap_target: Option<(NodeId, usize, PortDirection, Point, bool)>,
}

/// Shared coordinator connecting the real FerroUI node visual tree, port sockets,
/// persistent `MatrixTransform` camera, and [`WireBackdropLayer`].
#[derive(Clone)]
pub struct VisualGraphState {
    pub graph: Rc<RefCell<NodeGraph>>,
    pub registry: Rc<NodeRegistry>,
    pub socket_visuals: Rc<RefCell<FxHashMap<(NodeId, PortDirection, usize), WeakRef<Visual>>>>,
    pub node_visuals: Rc<RefCell<FxHashMap<NodeId, WeakRef<Visual>>>>,
    pub world_transform: Rc<RefCell<Option<Ref<MatrixTransform>>>>,
    pub wire_layer: Rc<RefCell<Option<WeakRef<WireBackdropLayer>>>>,
    pub active_wire: Rc<RefCell<Option<ActiveWireDrag>>>,
    pub marquee: Rc<RefCell<Option<(Point, Point)>>>,
    pub hover_port: Rc<RefCell<Option<(NodeId, PortDirection, usize, Point)>>>,
    pub on_evaluated: Rc<dyn Fn(&NodeGraph)>,
    /// Incremented ONLY when nodes are added/removed/loaded to reconcile `for_each` cleanly.
    pub topology_rev: Signal<u64>,
    /// Incremented when node outputs/values/errors change so live node UI badges refresh.
    pub eval_rev: Signal<u64>,
    /// Incremented when selection changes so node borders & Align bar refresh.
    pub selection_rev: Signal<u64>,
    /// Floating Grasshopper Quick-Search popup position `(screen_pt, canvas_pt)`.
    pub quick_search: Signal<Option<(Point, [f64; 2])>>,
    /// Floating Grasshopper Node Context Menu `(node_id, screen_pt)`.
    pub context_menu: Signal<Option<(NodeId, Point)>>,
    pub status_msg: Signal<String>,
}

impl VisualGraphState {
    pub fn new(
        graph: Rc<RefCell<NodeGraph>>,
        registry: Rc<NodeRegistry>,
        on_evaluated: Rc<dyn Fn(&NodeGraph)>,
    ) -> Self {
        Self {
            graph,
            registry,
            socket_visuals: Rc::new(RefCell::new(FxHashMap::default())),
            node_visuals: Rc::new(RefCell::new(FxHashMap::default())),
            world_transform: Rc::new(RefCell::new(None)),
            wire_layer: Rc::new(RefCell::new(None)),
            active_wire: Rc::new(RefCell::new(None)),
            marquee: Rc::new(RefCell::new(None)),
            hover_port: Rc::new(RefCell::new(None)),
            on_evaluated,
            topology_rev: signal(0),
            eval_rev: signal(0),
            selection_rev: signal(0),
            quick_search: signal(None),
            context_menu: signal(None),
            status_msg: signal(String::from(
                "Real FerroUI Visual Nodes · Drag sliders/curves/expressions live · Double-click or Space for Search · Right-drag Pan · Wheel Zoom",
            )),
        }
    }

    /// Updates the persistent GPU `MatrixTransform` on the FerroUI node visual container
    /// (zero layout passes!) and repaints the vector wire/grid layer in lockstep.
    pub fn sync_camera_transform(&self) {
        let (pan, z) = {
            let g = self.graph.borrow();
            (g.camera_pan, g.camera_zoom)
        };
        if let Some(tf) = self.world_transform.borrow().as_ref() {
            tf.set_matrix(Matrix::create_scale(z, z) * Matrix::create_translation(pan[0], pan[1]));
        }
        self.invalidate_wires();
    }

    pub fn invalidate_wires(&self) {
        if let Some(weak) = self.wire_layer.borrow().as_ref() {
            if let Some(layer) = weak.upgrade() {
                layer.invalidate_visual();
            }
        }
    }

    pub fn notify_topology_changed(&self) {
        if self.topology_rev.is_alive() {
            self.topology_rev.update(|v| *v = v.wrapping_add(1));
        }
        self.reevaluate_and_notify();
    }

    pub fn notify_selection_changed(&self) {
        if self.selection_rev.is_alive() {
            self.selection_rev.update(|v| *v = v.wrapping_add(1));
        }
        self.invalidate_wires();
    }

    pub fn reevaluate_and_notify(&self) {
        {
            let mut g = self.graph.borrow_mut();
            let _ = evaluate_graph(&mut g, &self.registry, 0.0);
        }
        if self.eval_rev.is_alive() {
            self.eval_rev.update(|v| *v = v.wrapping_add(1));
        }
        (self.on_evaluated)(&self.graph.borrow());
        self.invalidate_wires();
    }

    /// Updates `Canvas.Left` / `Canvas.Top` on every mounted node visual to match `NodeGraph::nodes`
    /// (used after Auto-Layout, Align, Undo/Redo, or dragging!).
    pub fn sync_all_node_positions(&self) {
        let g = self.graph.borrow();
        let visuals = self.node_visuals.borrow();
        for (nid, node) in &g.nodes {
            if let Some(weak) = visuals.get(nid) {
                if let Some(vis) = weak.upgrade() {
                    ferroui_controls::Canvas::set_left(&vis, node.position[0]);
                    ferroui_controls::Canvas::set_top(&vis, node.position[1]);
                }
            }
        }
        self.invalidate_wires();
    }

    pub fn screen_to_canvas(&self, pt: Point) -> [f64; 2] {
        let g = self.graph.borrow();
        let z = g.camera_zoom.max(0.1);
        [(pt.x - g.camera_pan[0]) / z, (pt.y - g.camera_pan[1]) / z]
    }

    pub fn fit_view(&self) {
        let (w, h) = self
            .wire_layer
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|l| (l.bounds().width.max(400.0), l.bounds().height.max(300.0)))
            .unwrap_or((900.0, 600.0));
        self.graph.borrow_mut().fit_to_view(w, h);
        self.sync_camera_transform();
    }

    pub fn zoom_at(&self, screen_pt: Point, wheel_dy: f64) {
        let factor = if wheel_dy > 0.0 {
            1.12
        } else if wheel_dy < 0.0 {
            1.0 / 1.12
        } else {
            1.0
        };
        {
            let mut g = self.graph.borrow_mut();
            let old_z = g.camera_zoom;
            let new_z = (old_z * factor).clamp(0.25, 2.5);
            let wx = (screen_pt.x - g.camera_pan[0]) / old_z;
            let wy = (screen_pt.y - g.camera_pan[1]) / old_z;
            g.camera_zoom = new_z;
            g.camera_pan = [screen_pt.x - wx * new_z, screen_pt.y - wy * new_z];
        }
        self.sync_camera_transform();
    }

    /// Queries the exact screen-space center of a real FerroUI port socket `Visual` relative to
    /// `WireBackdropLayer`, falling back cleanly to analytical canvas math on frame 0 before layout.
    pub fn port_screen_pos(
        &self,
        layer: &WireBackdropLayer,
        node_id: NodeId,
        dir: PortDirection,
        port_idx: usize,
    ) -> Option<Point> {
        if let Some(weak) = self.socket_visuals.borrow().get(&(node_id, dir, port_idx)) {
            if let Some(vis) = weak.upgrade() {
                let b = vis.bounds();
                if b.width > 0.5 && b.height > 0.5 {
                    let center = Point::new(b.width * 0.5, b.height * 0.5);
                    if let Some(pt) = vis.translate_point(center, layer) {
                        return Some(pt);
                    }
                }
            }
        }
        // Fallback before initial layout pass finishes
        let g = self.graph.borrow();
        let node = g.nodes.get(&node_id)?;
        let c = fallback_port_canvas(node, &self.registry, dir, port_idx);
        Some(Point::new(
            c[0] * g.camera_zoom + g.camera_pan[0],
            c[1] * g.camera_zoom + g.camera_pan[1],
        ))
    }

    /// Finds the nearest compatible real FerroUI port socket within magnetic snap radius (`22px`).
    pub fn find_snap_socket(
        &self,
        screen_pt: Point,
        from_node: NodeId,
        from_port: usize,
        from_dir: PortDirection,
    ) -> Option<(NodeId, usize, PortDirection, Point, bool)> {
        let layer = self.wire_layer.borrow().as_ref()?.upgrade()?;
        let g = self.graph.borrow();
        let want_dir = match from_dir {
            PortDirection::Output => PortDirection::Input,
            PortDirection::Input => PortDirection::Output,
        };

        let mut best: Option<(f64, NodeId, usize, Point, bool)> = None;
        let max_d2 = 24.0 * 24.0;

        for (&nid, node) in &g.nodes {
            if nid == from_node {
                continue;
            }
            let Some(comp) = self.registry.get(&node.type_id) else { continue };
            let count = match want_dir {
                PortDirection::Input => comp.inputs().len(),
                PortDirection::Output => comp.outputs().len(),
            };
            for pidx in 0..count {
                let Some(sp) = self.port_screen_pos(&layer, nid, want_dir, pidx) else { continue };
                let d2 = (sp.x - screen_pt.x).powi(2) + (sp.y - screen_pt.y).powi(2);
                if d2 <= max_d2 && best.as_ref().map_or(true, |b| d2 < b.0) {
                    let (src_n, dst_n) = match from_dir {
                        PortDirection::Output => (from_node, nid),
                        PortDirection::Input => (nid, from_node),
                    };
                    let compatible = !g.is_reachable(dst_n, src_n);
                    let _ = from_port;
                    best = Some((d2, nid, pidx, sp, compatible));
                }
            }
        }

        best.map(|(_, nid, pidx, sp, ok)| (nid, pidx, want_dir, sp, ok))
    }

    /// Completes an active wire drag initiated from a real port socket visual.
    pub fn finish_wire_drag(&self, screen_pt: Point, shift: bool) {
        let Some(drag) = self.active_wire.borrow_mut().take() else {
            return;
        };
        let multi = drag.multi_wire || shift;
        let snap = self.find_snap_socket(screen_pt, drag.from_node, drag.from_port, drag.from_dir);
        if let Some((hit_node, hit_port, _dir, _pt, compatible)) = snap {
            if compatible {
                let (src_n, src_p, dst_n, dst_p) = match drag.from_dir {
                    PortDirection::Output => (drag.from_node, drag.from_port, hit_node, hit_port),
                    PortDirection::Input => (hit_node, hit_port, drag.from_node, drag.from_port),
                };
                let ok = {
                    let mut g = self.graph.borrow_mut();
                    g.push_undo_checkpoint();
                    g.connect(src_n, src_p, dst_n, dst_p, multi).is_ok()
                };
                if ok {
                    self.reevaluate_and_notify();
                    return;
                }
            }
        }
        self.invalidate_wires();
    }
}

fn fallback_port_canvas(
    node: &NodeInstance,
    reg: &NodeRegistry,
    dir: PortDirection,
    port_idx: usize,
) -> [f64; 2] {
    let count = reg
        .get(&node.type_id)
        .map(|c| match dir {
            PortDirection::Input => c.inputs().len(),
            PortDirection::Output => c.outputs().len(),
        })
        .unwrap_or(1);
    let x = match dir {
        PortDirection::Input => node.position[0],
        PortDirection::Output => node.position[0] + node.size[0],
    };
    if count <= 1 {
        [x, node.position[1] + node.size[1] * 0.5]
    } else {
        let usable = (node.size[1] - 32.0).max(1.0);
        [x, node.position[1] + 22.0 + usable * (port_idx as f64 / (count - 1) as f64)]
    }
}

// =============================================================================
// Vector Wire, Grid, Group, Marquee & Minimap Backdrop Control
// =============================================================================

#[derive(Clone, Debug)]
enum BackdropDrag {
    Idle,
    Panning { start_screen: Point, start_pan: [f64; 2] },
    Marquee { start_screen: Point },
    Minimap,
}

struct WirePalette {
    bg: Rc<dyn IBrush>,
    minor_grid: Rc<dyn IPen>,
    major_grid: Rc<dyn IPen>,
    wire_empty: Rc<dyn IPen>,
    wire_item: Rc<dyn IPen>,
    wire_list_outer: Rc<dyn IPen>,
    wire_list_inner: Rc<dyn IPen>,
    wire_tree_outer: Rc<dyn IPen>,
    wire_tree_inner: Rc<dyn IPen>,
    wire_selected: Rc<dyn IPen>,
    wire_faint: Rc<dyn IPen>,
    wire_preview_ok: Rc<dyn IPen>,
    wire_preview_bad: Rc<dyn IPen>,
    marquee_fill: Rc<dyn IBrush>,
    marquee_border: Rc<dyn IPen>,
    minimap_bg: Rc<dyn IBrush>,
    minimap_border: Rc<dyn IPen>,
    minimap_view: Rc<dyn IPen>,
    text_muted: Rc<dyn IBrush>,
    text_accent: Rc<dyn IBrush>,
    text_primary: Rc<dyn IBrush>,
    tooltip_bg: Rc<dyn IBrush>,
    tooltip_border: Rc<dyn IPen>,
    snap_ok_pen: Rc<dyn IPen>,
    snap_bad_pen: Rc<dyn IPen>,
    node_dot: Rc<dyn IBrush>,
    node_dot_sel: Rc<dyn IBrush>,
}

impl WirePalette {
    fn new() -> Self {
        let dashed_tree = Rc::new(ImmutableDashStyle::new(Some(&[5.0, 3.5]), 0.0));
        let dashed_empty = Rc::new(ImmutableDashStyle::new(Some(&[4.0, 4.0]), 0.0));
        let empty_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xCCF59E0B));
        let tree_outer_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xDDA78BFA));
        let tree_inner_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0A0F1A));
        Self {
            bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0A0F1A)),
            minor_grid: Rc::new(ImmutablePen::from_uint32(0xFF111827, 1.0)),
            major_grid: Rc::new(ImmutablePen::from_uint32(0xFF1A253A, 1.0)),
            wire_empty: Rc::new(ImmutablePen::new(Some(empty_br), 1.8, Some(dashed_empty), PenLineCap::Round, Default::default(), 4.0)),
            wire_item: Rc::new(ImmutablePen::from_uint32(0xCC94A3B8, 2.2)),
            wire_list_outer: Rc::new(ImmutablePen::from_uint32(0xDD38BDF8, 4.8)),
            wire_list_inner: Rc::new(ImmutablePen::from_uint32(0xFF0A0F1A, 1.9)),
            wire_tree_outer: Rc::new(ImmutablePen::new(Some(tree_outer_br), 5.0, Some(dashed_tree.clone()), PenLineCap::Round, Default::default(), 4.0)),
            wire_tree_inner: Rc::new(ImmutablePen::new(Some(tree_inner_br), 1.9, Some(dashed_tree), PenLineCap::Round, Default::default(), 4.0)),
            wire_selected: Rc::new(ImmutablePen::from_uint32(0xFF10B981, 3.4)),
            wire_faint: Rc::new(ImmutablePen::from_uint32(0x4494A3B8, 1.2)),
            wire_preview_ok: Rc::new(ImmutablePen::from_uint32(0xFF34D399, 2.8)),
            wire_preview_bad: Rc::new(ImmutablePen::from_uint32(0xFFF43F5E, 2.8)),
            marquee_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x2238BDF8)),
            marquee_border: Rc::new(ImmutablePen::from_uint32(0xCC38BDF8, 1.4)),
            minimap_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xDE0B101D)),
            minimap_border: Rc::new(ImmutablePen::from_uint32(0xFF2B3B58, 1.2)),
            minimap_view: Rc::new(ImmutablePen::from_uint32(0xCC38BDF8, 1.5)),
            text_muted: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF94A3B8)),
            text_accent: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF7DD3FC)),
            text_primary: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFF1F5F9)),
            tooltip_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xF00F172A)),
            tooltip_border: Rc::new(ImmutablePen::from_uint32(0xFF38BDF8, 1.2)),
            snap_ok_pen: Rc::new(ImmutablePen::from_uint32(0xFF10B981, 2.4)),
            snap_bad_pen: Rc::new(ImmutablePen::from_uint32(0xFFF43F5E, 2.4)),
            node_dot: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF64748B)),
            node_dot_sel: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF38BDF8)),
        }
    }
}

#[repr(C)]
pub struct WireBackdropLayer {
    base: Control,
    state: RefCell<Option<VisualGraphState>>,
    drag: RefCell<BackdropDrag>,
    palette: WirePalette,
}

ferro_class!(WireBackdropLayer: Control);
ferro_impl_classes!(
    WireBackdropLayer: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(WireBackdropLayer { new: WireBackdropLayer::new });

impl ViewClass for WireBackdropLayer {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

impl WireBackdropLayer {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            state: RefCell::new(None),
            drag: RefCell::new(BackdropDrag::Idle),
            palette: WirePalette::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(true);
        this.set_focusable(true);
        this
    }

    pub fn bind(&self, state: VisualGraphState) {
        *state.wire_layer.borrow_mut() = Some(self.to_ref().downgrade());
        *self.state.borrow_mut() = Some(state);
        self.invalidate_visual();
    }

    fn minimap_rect(&self) -> Rect {
        let b = self.bounds();
        Rect::new((b.width - 178.0).max(8.0), (b.height - 114.0).max(8.0), 166.0, 102.0)
    }

    fn hit_test_wire_screen(&self, st: &VisualGraphState, screen_pt: Point) -> Option<WireId> {
        let g = st.graph.borrow();
        let thresh2 = 10.0 * 10.0;
        for (_, w) in &g.wires {
            let Some(p0) = st.port_screen_pos(self, w.from_node, PortDirection::Output, w.from_port) else { continue };
            let Some(p3) = st.port_screen_pos(self, w.to_node, PortDirection::Input, w.to_port) else { continue };
            let dx = ((p3.x - p0.x).abs() * 0.48).clamp(32.0, 180.0);
            let p1 = Point::new(p0.x + dx, p0.y);
            let p2 = Point::new(p3.x - dx, p3.y);
            for step in 0..=18 {
                let t = step as f64 / 18.0;
                let u = 1.0 - t;
                let bx = u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x;
                let by = u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y;
                if (bx - screen_pt.x).powi(2) + (by - screen_pt.y).powi(2) <= thresh2 {
                    return Some(w.id);
                }
            }
        }
        None
    }

    fn jump_minimap(&self, st: &VisualGraphState, screen_pt: Point) {
        let mm = self.minimap_rect();
        let b = self.bounds();
        let mut g = st.graph.borrow_mut();
        let Some([min_x, min_y, max_x, max_y]) = g.content_bounds() else { return };
        let pad = 120.0;
        let bx0 = min_x - pad;
        let by0 = min_y - pad;
        let bw = (max_x - min_x + pad * 2.0).max(220.0);
        let bh = (max_y - min_y + pad * 2.0).max(160.0);
        let nx = ((screen_pt.x - mm.x) / mm.width).clamp(0.0, 1.0);
        let ny = ((screen_pt.y - mm.y) / mm.height).clamp(0.0, 1.0);
        let cx = bx0 + nx * bw;
        let cy = by0 + ny * bh;
        g.camera_pan = [b.width * 0.5 - cx * g.camera_zoom, b.height * 0.5 - cy * g.camera_zoom];
        drop(g);
        st.sync_camera_transform();
    }
}

impl InputElementImpl for WireBackdropLayer {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        this.focus();
        let Some(st) = this.state.borrow().clone() else { return };
        st.quick_search.set(None);
        st.context_menu.set(None);

        let input: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&input));

        let pt = e.get_position(Some(this));
        let props = e.properties();
        let mods = e.key_modifiers();
        let is_right = props.is_right_button_pressed;
        let is_middle = props.is_middle_button_pressed;
        let alt = mods.contains(KeyModifiers::ALT);
        let shift = mods.contains(KeyModifiers::SHIFT);

        // Minimap jump
        let mm = this.minimap_rect();
        if !is_right && mm.contains(pt) && this.bounds().width > 340.0 {
            *this.drag.borrow_mut() = BackdropDrag::Minimap;
            this.jump_minimap(&st, pt);
            return;
        }

        // Right-click wire disconnects; otherwise right-drag pans!
        if is_right {
            if let Some(wid) = this.hit_test_wire_screen(&st, pt) {
                {
                    let mut g = st.graph.borrow_mut();
                    g.push_undo_checkpoint();
                    g.disconnect_wire(wid);
                }
                st.reevaluate_and_notify();
                return;
            }
            let start_pan = st.graph.borrow().camera_pan;
            *this.drag.borrow_mut() = BackdropDrag::Panning { start_screen: pt, start_pan };
            return;
        }

        if is_middle || alt {
            let start_pan = st.graph.borrow().camera_pan;
            *this.drag.borrow_mut() = BackdropDrag::Panning { start_screen: pt, start_pan };
            return;
        }

        // Click or double-click a wire (double-click inserts a Grasshopper Relay right on the wire!)
        if let Some(wid) = this.hit_test_wire_screen(&st, pt) {
            if e.click_count() >= 2 {
                let c = st.screen_to_canvas(pt);
                let inserted = st
                    .graph
                    .borrow_mut()
                    .insert_relay_on_wire(wid, c[0], c[1], &st.registry)
                    .is_some();
                if inserted {
                    st.notify_topology_changed();
                    st.status_msg.set("Inserted Grasshopper Relay node inline on wire!".into());
                }
                return;
            }
            {
                let mut g = st.graph.borrow_mut();
                g.selected_nodes.clear();
                g.selected_wires.clear();
                g.selected_wires.insert(wid);
            }
            st.notify_selection_changed();
            return;
        }

        // Double-click empty canvas opens the floating Grasshopper Quick-Search Popup!
        if e.click_count() >= 2 {
            let c = st.screen_to_canvas(pt);
            st.quick_search.set(Some((pt, c)));
            return;
        }

        // Empty space drag -> Marquee box selection!
        if !shift {
            let mut g = st.graph.borrow_mut();
            g.selected_nodes.clear();
            g.selected_wires.clear();
        }
        st.notify_selection_changed();
        *st.marquee.borrow_mut() = Some((pt, pt));
        *this.drag.borrow_mut() = BackdropDrag::Marquee { start_screen: pt };
        this.invalidate_visual();
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let Some(st) = this.state.borrow().clone() else { return };
        let pt = e.get_position(Some(this));
        match this.drag.borrow().clone() {
            BackdropDrag::Idle => {}
            BackdropDrag::Minimap => this.jump_minimap(&st, pt),
            BackdropDrag::Panning { start_screen, start_pan } => {
                {
                    let mut g = st.graph.borrow_mut();
                    g.camera_pan = [
                        start_pan[0] + (pt.x - start_screen.x),
                        start_pan[1] + (pt.y - start_screen.y),
                    ];
                }
                st.sync_camera_transform();
            }
            BackdropDrag::Marquee { start_screen } => {
                *st.marquee.borrow_mut() = Some((start_screen, pt));
                let c0 = st.screen_to_canvas(start_screen);
                let c1 = st.screen_to_canvas(pt);
                let x0 = c0[0].min(c1[0]);
                let y0 = c0[1].min(c1[1]);
                let x1 = c0[0].max(c1[0]);
                let y1 = c0[1].max(c1[1]);
                {
                    let mut g = st.graph.borrow_mut();
                    g.selected_nodes.clear();
                    let hits: Vec<NodeId> = g
                        .nodes
                        .values()
                        .filter(|n| {
                            n.position[0] + n.size[0] >= x0
                                && n.position[0] <= x1
                                && n.position[1] + n.size[1] >= y0
                                && n.position[1] <= y1
                        })
                        .map(|n| n.id)
                        .collect();
                    for id in hits {
                        g.selected_nodes.insert(id);
                    }
                }
                st.notify_selection_changed();
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        e.pointer().capture(None);
        *this.drag.borrow_mut() = BackdropDrag::Idle;
        if let Some(st) = this.state.borrow().as_ref() {
            if st.marquee.borrow_mut().take().is_some() {
                this.invalidate_visual();
            }
        }
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        if let Some(st) = this.state.borrow().as_ref() {
            st.zoom_at(e.get_position(Some(this)), e.delta().y);
            e.set_handled(true);
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let Some(st) = this.state.borrow().clone() else { return };
        let cmd = e.key_modifiers.contains(KeyModifiers::CONTROL)
            || e.key_modifiers.contains(KeyModifiers::META);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);

        match e.key {
            Key::Space => {
                let b = this.bounds();
                let center = Point::new(b.width * 0.5, b.height * 0.42);
                st.quick_search.set(Some((center, st.screen_to_canvas(center))));
                e.set_handled(true);
            }
            Key::Escape => {
                st.quick_search.set(None);
                st.context_menu.set(None);
                e.set_handled(true);
            }
            Key::Delete | Key::Back => {
                if st.graph.borrow_mut().delete_selection() {
                    st.notify_topology_changed();
                    st.notify_selection_changed();
                    e.set_handled(true);
                }
            }
            Key::Z if cmd && shift => {
                if st.graph.borrow_mut().redo() {
                    st.notify_topology_changed();
                    st.sync_all_node_positions();
                    e.set_handled(true);
                }
            }
            Key::Z if cmd => {
                if st.graph.borrow_mut().undo() {
                    st.notify_topology_changed();
                    st.sync_all_node_positions();
                    e.set_handled(true);
                }
            }
            Key::Y if cmd => {
                if st.graph.borrow_mut().redo() {
                    st.notify_topology_changed();
                    st.sync_all_node_positions();
                    e.set_handled(true);
                }
            }
            Key::D if cmd => {
                if !st.graph.borrow_mut().duplicate_selection().is_empty() {
                    st.notify_topology_changed();
                    st.notify_selection_changed();
                    e.set_handled(true);
                }
            }
            Key::G if cmd => {
                if st.graph.borrow_mut().group_selection("Group", 0x2238BDF8).is_some() {
                    st.invalidate_wires();
                    e.set_handled(true);
                }
            }
            Key::E if cmd => {
                st.graph.borrow_mut().toggle_disable_selection();
                st.reevaluate_and_notify();
                e.set_handled(true);
            }
            Key::F => {
                st.fit_view();
                e.set_handled(true);
            }
            Key::L => {
                st.graph.borrow_mut().auto_layout();
                st.sync_all_node_positions();
                st.fit_view();
                e.set_handled(true);
            }
            _ => {}
        }
    }
}

impl VisualImpl for WireBackdropLayer {
    fn render(this: &Self, dc: &mut DrawingContext) {
        let b = this.bounds();
        let w = b.width.max(1.0);
        let h = b.height.max(1.0);

        let clip = dc.push_clip(Rect::new(0.0, 0.0, w, h));
        let mut dc = dc.scope(clip);

        dc.fill_rectangle(&this.palette.bg, Rect::new(0.0, 0.0, w, h), 0.0);

        let Some(st) = this.state.borrow().clone() else { return };
        let g = st.graph.borrow();

        // 1. Adaptive Infinite Rhino Grasshopper Grid
        let z = g.camera_zoom.clamp(0.25, 2.8);
        let minor = 24.0 * z;
        let major = minor * 5.0;
        if minor >= 9.0 {
            let mut x = g.camera_pan[0].rem_euclid(minor);
            while x < w {
                dc.draw_line(&this.palette.minor_grid, Point::new(x, 0.0), Point::new(x, h));
                x += minor;
            }
            let mut y = g.camera_pan[1].rem_euclid(minor);
            while y < h {
                dc.draw_line(&this.palette.minor_grid, Point::new(0.0, y), Point::new(w, y));
                y += minor;
            }
        }
        let mut x = g.camera_pan[0].rem_euclid(major);
        while x < w {
            dc.draw_line(&this.palette.major_grid, Point::new(x, 0.0), Point::new(x, h));
            x += major;
        }
        let mut y = g.camera_pan[1].rem_euclid(major);
        while y < h {
            dc.draw_line(&this.palette.major_grid, Point::new(0.0, y), Point::new(w, y));
            y += major;
        }

        // 2. Colored Grasshopper Node Groups (measured from real mounted node visuals!)
        let node_vis_map = st.node_visuals.borrow();
        for group in g.groups.values() {
            let mut min_x = f64::INFINITY;
            let mut min_y = f64::INFINITY;
            let mut max_x = f64::NEG_INFINITY;
            let mut max_y = f64::NEG_INFINITY;
            let mut any = false;
            for nid in &group.members {
                if let Some(weak) = node_vis_map.get(nid) {
                    if let Some(vis) = weak.upgrade() {
                        let nb = vis.bounds();
                        if nb.width > 1.0 && nb.height > 1.0 {
                            if let (Some(tl), Some(br)) = (
                                vis.translate_point(Point::new(0.0, 0.0), this),
                                vis.translate_point(Point::new(nb.width, nb.height), this),
                            ) {
                                min_x = min_x.min(tl.x);
                                min_y = min_y.min(tl.y);
                                max_x = max_x.max(br.x);
                                max_y = max_y.max(br.y);
                                any = true;
                                continue;
                            }
                        }
                    }
                }
                if let Some(n) = g.nodes.get(nid) {
                    let sx0 = n.position[0] * z + g.camera_pan[0];
                    let sy0 = n.position[1] * z + g.camera_pan[1];
                    let sx1 = (n.position[0] + n.size[0]) * z + g.camera_pan[0];
                    let sy1 = (n.position[1] + n.size[1]) * z + g.camera_pan[1];
                    min_x = min_x.min(sx0);
                    min_y = min_y.min(sy0);
                    max_x = max_x.max(sx1);
                    max_y = max_y.max(sy1);
                    any = true;
                }
            }
            if any {
                let pad_x = 16.0 * z;
                let pad_top = 26.0 * z;
                let pad_bot = 16.0 * z;
                let rect = Rect::new(min_x - pad_x, min_y - pad_top, (max_x - min_x) + pad_x * 2.0, (max_y - min_y) + pad_top + pad_bot);
                let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(group.color_argb));
                let border_pen: Rc<dyn IPen> = Rc::new(ImmutablePen::from_uint32((group.color_argb & 0x00FF_FFFF) | 0x8800_0000, 1.5));
                dc.draw_rectangle(Some(&fill), Some(&border_pen), rect, 12.0, 12.0, &BoxShadows::default());
                draw_text(&mut dc, &group.title, Point::new(rect.x + 10.0, rect.y + 5.0), (11.5 * z).clamp(9.0, 15.0), &this.palette.text_accent);
            }
        }
        drop(node_vis_map);

        // 3. Pixel-Perfect Grasshopper Fancy Bezier Wires anchored to Real FerroUI Port Visuals!
        for wire in g.wires.values() {
            if wire.display == WireDisplay::Hidden {
                continue;
            }
            let Some(p0) = st.port_screen_pos(this, wire.from_node, PortDirection::Output, wire.from_port) else { continue };
            let Some(p3) = st.port_screen_pos(this, wire.to_node, PortDirection::Input, wire.to_port) else { continue };
            let is_sel = g.selected_wires.contains(&wire.id)
                || g.selected_nodes.contains(&wire.from_node)
                || g.selected_nodes.contains(&wire.to_node);
            let data_kind = g
                .nodes
                .get(&wire.from_node)
                .and_then(|n| n.output_tree(wire.from_port))
                .map(|t| t.wire_data_kind())
                .unwrap_or(WireDataKind::Empty);

            draw_fancy_wire(
                &mut dc,
                &this.palette,
                p0,
                p3,
                data_kind,
                g.fancy_wires,
                wire.display == WireDisplay::Faint,
                is_sel,
            );
        }

        // 4. Live Drag-to-Wire Preview + Magnetic Target Snap Halo
        if let Some(drag) = st.active_wire.borrow().as_ref() {
            if let Some(anchor) = st.port_screen_pos(this, drag.from_node, drag.from_dir, drag.from_port) {
                let (end_pt, ok) = match drag.snap_target {
                    Some((_, _, _, sp, compat)) => (sp, compat),
                    None => (drag.cursor_screen, true),
                };
                let (p0, p3) = match drag.from_dir {
                    PortDirection::Output => (anchor, end_pt),
                    PortDirection::Input => (end_pt, anchor),
                };
                let pen = if ok { &this.palette.wire_preview_ok } else { &this.palette.wire_preview_bad };
                draw_bezier(&mut dc, pen, p0, p3);
                if let Some((_, _, _, sp, compat)) = drag.snap_target {
                    let ring = if compat { &this.palette.snap_ok_pen } else { &this.palette.snap_bad_pen };
                    dc.draw_ellipse_at(None, Some(ring), sp, 10.0, 10.0);
                }
            }
        }

        // 5. Marquee Selection Rectangle
        if let Some((s0, s1)) = *st.marquee.borrow() {
            let rect = Rect::new(s0.x.min(s1.x), s0.y.min(s1.y), (s1.x - s0.x).abs(), (s1.y - s0.y).abs());
            dc.draw_rectangle(Some(&this.palette.marquee_fill), Some(&this.palette.marquee_border), rect, 3.0, 3.0, &BoxShadows::default());
        }

        // 6. Interactive Minimap (bottom-right)
        if w > 340.0 && h > 220.0 {
            let mm = this.minimap_rect();
            dc.draw_rectangle(Some(&this.palette.minimap_bg), Some(&this.palette.minimap_border), mm, 8.0, 8.0, &BoxShadows::default());
            if let Some([min_x, min_y, max_x, max_y]) = g.content_bounds() {
                let pad = 120.0;
                let bx0 = min_x - pad;
                let by0 = min_y - pad;
                let bw = (max_x - min_x + pad * 2.0).max(220.0);
                let bh = (max_y - min_y + pad * 2.0).max(160.0);
                let sx = mm.width / bw;
                let sy = mm.height / bh;
                for n in g.nodes.values() {
                    let nx = mm.x + (n.position[0] - bx0) * sx;
                    let ny = mm.y + (n.position[1] - by0) * sy;
                    let nw = (n.size[0] * sx).max(4.0);
                    let nh = (n.size[1] * sy).max(3.0);
                    let br = if g.selected_nodes.contains(&n.id) {
                        &this.palette.node_dot_sel
                    } else {
                        &this.palette.node_dot
                    };
                    dc.fill_rectangle(br, Rect::new(nx, ny, nw, nh), 1.5);
                }
                let vx0 = (-g.camera_pan[0]) / z;
                let vy0 = (-g.camera_pan[1]) / z;
                let rx = (mm.x + (vx0 - bx0) * sx).clamp(mm.x, mm.x + mm.width - 8.0);
                let ry = (mm.y + (vy0 - by0) * sy).clamp(mm.y, mm.y + mm.height - 8.0);
                let rw = ((w / z) * sx).clamp(8.0, mm.x + mm.width - rx);
                let rh = ((h / z) * sy).clamp(8.0, mm.y + mm.height - ry);
                dc.draw_rectangle_outline(&this.palette.minimap_view, Rect::new(rx, ry, rw, rh), 2.0);
            }
        }

        // 7. Live Port Hover DataTree Tooltip
        if let Some((nid, dir, pidx, spt)) = *st.hover_port.borrow() {
            if let Some(node) = g.nodes.get(&nid) {
                if let Some(comp) = st.registry.get(&node.type_id) {
                    let spec = match dir {
                        PortDirection::Input => comp.inputs().get(pidx).cloned(),
                        PortDirection::Output => comp.outputs().get(pidx).cloned(),
                    };
                    if let Some(spec) = spec {
                        let detail = match dir {
                            PortDirection::Output => node
                                .output_tree(pidx)
                                .map(|t| format!("{} br · {} items · {}", t.branch_count(), t.item_count(), t.format_panel(1)))
                                .unwrap_or_else(|| "<empty>".into()),
                            PortDirection::Input => {
                                let wcnt = g.incoming_wires(nid, pidx).len();
                                let m = node.input_modifiers.get(pidx).map(|x| x.badge()).unwrap_or_default();
                                if m.is_empty() {
                                    format!("{wcnt} wire(s) · default {}", spec.default_value.short_summary())
                                } else {
                                    format!("{wcnt} wire(s) · [{m}]")
                                }
                            }
                        };
                        let tx = (spt.x + 14.0).min(w - 258.0).max(8.0);
                        let ty = (spt.y - 48.0).clamp(8.0, h - 58.0);
                        dc.draw_rectangle(Some(&this.palette.tooltip_bg), Some(&this.palette.tooltip_border), Rect::new(tx, ty, 248.0, 44.0), 6.0, 6.0, &BoxShadows::default());
                        draw_text(&mut dc, &format!("{} ({}) · {}", spec.name, spec.nickname, spec.port_type.label()), Point::new(tx + 8.0, ty + 5.0), 10.5, &this.palette.text_accent);
                        let short: String = detail.chars().take(36).collect();
                        draw_text(&mut dc, &short, Point::new(tx + 8.0, ty + 22.0), 10.0, &this.palette.text_primary);
                    }
                }
            }
        }

        // 8. Bottom-left Rhino Grasshopper shortcut legend
        draw_text(
            &mut dc,
            &format!(
                "Zoom {:.0}% · Real FerroUI Visual Nodes · Double-click / Space Search · Double-click Wire = Relay · Right-drag Pan · Wheel Zoom · F Fit · L Layout",
                z * 100.0
            ),
            Point::new(12.0, h - 20.0),
            10.5,
            &this.palette.text_muted,
        );
    }
}

fn draw_bezier(dc: &mut DrawingContext, pen: &Rc<dyn IPen>, p0: Point, p3: Point) {
    let dx = ((p3.x - p0.x).abs() * 0.48).clamp(32.0, 180.0);
    let p1 = Point::new(p0.x + dx, p0.y);
    let p2 = Point::new(p3.x - dx, p3.y);
    let geom = StreamGeometry::new();
    {
        let mut ctx = geom.open();
        ctx.begin_figure(p0, false);
        ctx.cubic_bezier_to(p1, p2, p3, true);
        ctx.end_figure(false);
        ctx.dispose();
    }
    dc.draw_geometry(None, Some(pen), &geom.upcast());
}

#[allow(clippy::too_many_arguments)]
fn draw_fancy_wire(
    dc: &mut DrawingContext,
    pal: &WirePalette,
    p0: Point,
    p3: Point,
    kind: WireDataKind,
    fancy: bool,
    faint: bool,
    selected: bool,
) {
    if faint {
        draw_bezier(dc, &pal.wire_faint, p0, p3);
        return;
    }
    if selected {
        draw_bezier(dc, &pal.wire_selected, p0, p3);
        if fancy && matches!(kind, WireDataKind::List | WireDataKind::Tree) {
            draw_bezier(dc, &pal.wire_list_inner, p0, p3);
        }
        return;
    }
    if !fancy {
        draw_bezier(dc, &pal.wire_item, p0, p3);
        return;
    }
    match kind {
        WireDataKind::Empty => draw_bezier(dc, &pal.wire_empty, p0, p3),
        WireDataKind::Item => draw_bezier(dc, &pal.wire_item, p0, p3),
        WireDataKind::List => {
            draw_bezier(dc, &pal.wire_list_outer, p0, p3);
            draw_bezier(dc, &pal.wire_list_inner, p0, p3);
        }
        WireDataKind::Tree => {
            draw_bezier(dc, &pal.wire_tree_outer, p0, p3);
            draw_bezier(dc, &pal.wire_tree_inner, p0, p3);
        }
    }
}

fn draw_text(dc: &mut DrawingContext, text: &str, pos: Point, size: f64, brush: &Rc<dyn IBrush>) {
    let ft = FormattedText::new(
        text,
        locale!("en-US"),
        FlowDirection::LeftToRight,
        Typeface::default(),
        size,
        Some(brush.clone()),
    );
    dc.draw_text(&ft, pos);
}
