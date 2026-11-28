//! **Rhino Grasshopper Visual-Tree Coordinator & Hardware Vector Wire/Grid Layer**.
//!
//! Automatically adapts to FerroUI's stock current application theme (`ThemeVariant`)
//! with zero custom theme overrides:
//! - Adaptive drafting grid + origin crosshair rendered cleanly over the stock surface
//! - Node Groups measured directly from mounted real node visuals
//! - **Signature Grasshopper Fancy Wires**: single Item wire, double-line List wire,
//!   dashed double-line DataTree wire, selected wires, orange Empty wire
//! - **Rhino Window vs Crossing Marquee** (left→right Blue Window Select, right→left Green Crossing Select!)
//! - Magnetic port snap halo + `+` Shift multi-wire badge, interactive minimap, and DataTree tooltips!

use crate::eval::evaluate_graph;
use crate::graph::{NodeGraph, NodeInstance};
use crate::node::NodeRegistry;
use crate::serialization::try_graph_to_json;
use crate::types::{NodeId, PortDirection, WireDataKind, WireDisplay, WireId};
use ferroui_base::input::platform::ClipboardExtensions;
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
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point,
    Rect, Ref, StyledElementImpl, Visual, VisualImpl, WeakRef,
};
use ferroui_controls::{Application, Control, ControlImpl, TopLevel};
use ferroui_view::prelude::*;
use ferroui_view::ViewClass;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

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

/// Shared reactive coordinator connecting real FerroUI node visuals, port sockets,
/// GPU `MatrixTransform` camera, and [`WireBackdropLayer`].
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
    pub topology_rev: Signal<u64>,
    /// Recreates inline widget state after undo/redo while retaining keyed views for normal edits.
    pub history_rev: Signal<u32>,
    pub eval_rev: Signal<u64>,
    pub selection_rev: Signal<u64>,
    /// Tracks whether the host application's stock `ThemeVariant` is currently dark.
    pub is_dark_theme: Signal<bool>,
    pub quick_search: Signal<Option<(Point, [f64; 2])>>,
    pub context_menu: Signal<Option<(NodeId, Point)>>,
    pub status_msg: Signal<String>,
    /// Monotonic clock shared by all views of this editor instance.
    pub solution_clock: Rc<Instant>,
}

impl VisualGraphState {
    pub fn new(
        graph: Rc<RefCell<NodeGraph>>,
        registry: Rc<NodeRegistry>,
        on_evaluated: Rc<dyn Fn(&NodeGraph)>,
    ) -> Self {
        let init_dark = Application::current()
            .and_then(|app| app.actual_theme_variant())
            .is_some_and(|v| v == ThemeVariant::dark());
        Self {
            graph,
            registry,
            solution_clock: Rc::new(Instant::now()),
            socket_visuals: Rc::new(RefCell::new(FxHashMap::default())),
            node_visuals: Rc::new(RefCell::new(FxHashMap::default())),
            world_transform: Rc::new(RefCell::new(None)),
            wire_layer: Rc::new(RefCell::new(None)),
            active_wire: Rc::new(RefCell::new(None)),
            marquee: Rc::new(RefCell::new(None)),
            hover_port: Rc::new(RefCell::new(None)),
            on_evaluated,
            topology_rev: signal(0),
            history_rev: signal(0),
            eval_rev: signal(0),
            selection_rev: signal(0),
            is_dark_theme: signal(init_dark),
            quick_search: signal(None),
            context_menu: signal(None),
            status_msg: signal(String::from(
                "Double-click / Space = Search · Double-click wire = Relay · Shift-wire = Multi-input · Right-drag = Pan · Wheel = Zoom",
            )),
        }
    }

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
        {
            let graph = self.graph.borrow();
            self.node_visuals
                .borrow_mut()
                .retain(|id, _| graph.nodes.contains_key(id));
            self.socket_visuals
                .borrow_mut()
                .retain(|(id, _, _), _| graph.nodes.contains_key(id));
        }
        if self.topology_rev.is_alive() {
            self.topology_rev.update(|v| *v = v.wrapping_add(1));
        }
        self.reevaluate_and_notify();
    }

    pub fn notify_history_restored(&self) {
        if self.history_rev.is_alive() {
            self.history_rev
                .update(|revision| *revision = revision.wrapping_add(1));
        }
        self.notify_topology_changed();
        self.notify_selection_changed();
    }

    pub fn notify_selection_changed(&self) {
        if self.selection_rev.is_alive() {
            self.selection_rev.update(|v| *v = v.wrapping_add(1));
        }
        self.invalidate_wires();
    }

    pub fn reevaluate_and_notify(&self) {
        self.reevaluate_at(self.solution_clock.elapsed().as_secs_f64());
    }

    /// Evaluates with an explicit clock value for externally driven editors.
    pub fn reevaluate_at(&self, time_secs: f64) {
        {
            let mut g = self.graph.borrow_mut();
            let _ = evaluate_graph(&mut g, &self.registry, time_secs);
        }
        if self.eval_rev.is_alive() {
            self.eval_rev.update(|v| *v = v.wrapping_add(1));
        }
        (self.on_evaluated)(&self.graph.borrow());
        self.invalidate_wires();
    }

    pub fn needs_clock_tick(&self) -> bool {
        let graph = self.graph.borrow();
        !graph.solution_locked
            && graph.nodes.values().any(|node| {
                !node.disabled
                    && (self
                        .registry
                        .get(&node.type_id)
                        .is_some_and(|component| component.is_time_dependent(&node.widget))
                        || (node.dirty
                            && matches!(node.widget, crate::types::ParamWidget::Button { .. })))
            })
    }

    pub fn copy_definition_to_clipboard(&self) {
        let json = match try_graph_to_json(&self.graph.borrow()) {
            Ok(json) => json,
            Err(error) => {
                self.status_msg.set(format!("Cannot export: {error}"));
                return;
            }
        };
        let clipboard = self
            .wire_layer
            .borrow()
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .and_then(|layer| TopLevel::get_top_level(Some(&layer)))
            .and_then(|top_level| top_level.clipboard());
        let Some(clipboard) = clipboard else {
            self.status_msg
                .set("Clipboard is unavailable on this platform".into());
            return;
        };
        let status = self.status_msg;
        drop(Dispatcher::ui_thread().spawn_local(async move {
            let message = match clipboard.set_text_async(Some(&json)).await {
                Ok(()) => format!("Copied graph JSON to clipboard ({} bytes)", json.len()),
                Err(error) => format!("Could not copy graph: {error}"),
            };
            if status.is_alive() {
                status.set(message);
            }
        }));
    }

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

    pub fn sync_measured_node_sizes(&self) {
        let mut g = self.graph.borrow_mut();
        let visuals = self.node_visuals.borrow();
        for (nid, node) in &mut g.nodes {
            if let Some(weak) = visuals.get(nid) {
                if let Some(vis) = weak.upgrade() {
                    let b = vis.bounds();
                    if b.width > 12.0 && b.height > 12.0 {
                        node.size = [b.width, b.height];
                    }
                }
            }
        }
    }

    pub fn screen_to_canvas(&self, pt: Point) -> [f64; 2] {
        let g = self.graph.borrow();
        let z = g.camera_zoom.max(0.1);
        [(pt.x - g.camera_pan[0]) / z, (pt.y - g.camera_pan[1]) / z]
    }

    pub fn fit_view(&self) {
        self.sync_measured_node_sizes();
        let (w, h) = self
            .wire_layer
            .borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
            .map(|l| (l.bounds().width, l.bounds().height))
            .unwrap_or((0.0, 0.0));
        if w > 120.0 && h > 100.0 {
            self.graph.borrow_mut().fit_to_view(w, h);
            self.sync_camera_transform();
        }
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
        let g = self.graph.borrow();
        let node = g.nodes.get(&node_id)?;
        let c = fallback_port_canvas(node, &self.registry, dir, port_idx);
        Some(Point::new(
            c[0] * g.camera_zoom + g.camera_pan[0],
            c[1] * g.camera_zoom + g.camera_pan[1],
        ))
    }

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
        let max_d2 = 26.0 * 26.0;

        for (&nid, node) in &g.nodes {
            if nid == from_node {
                continue;
            }
            let Some(comp) = self.registry.get(&node.type_id) else {
                continue;
            };
            let count = match want_dir {
                PortDirection::Input => comp.inputs().len(),
                PortDirection::Output => comp.outputs().len(),
            };
            for pidx in 0..count {
                let Some(sp) = self.port_screen_pos(&layer, nid, want_dir, pidx) else {
                    continue;
                };
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
        let usable = (node.size[1] - 24.0).max(1.0);
        [
            x,
            node.position[1] + 12.0 + usable * (port_idx as f64 / (count - 1) as f64),
        ]
    }
}

// =============================================================================
// Authentic Rhino Grasshopper Vector Wire, Grid, Group & Minimap Backdrop
// =============================================================================

#[derive(Clone, Debug)]
enum BackdropDrag {
    Idle,
    Panning {
        start_screen: Point,
        start_pan: [f64; 2],
    },
    Marquee {
        start_screen: Point,
    },
    Minimap,
}

struct ThemeBrushes {
    bg: Rc<dyn IBrush>,
    minor_grid: Rc<dyn IPen>,
    major_grid: Rc<dyn IPen>,
    origin_pen: Rc<dyn IPen>,
    wire_empty: Rc<dyn IPen>,
    wire_item: Rc<dyn IPen>,
    wire_list_outer: Rc<dyn IPen>,
    wire_list_inner: Rc<dyn IPen>,
    wire_tree_outer: Rc<dyn IPen>,
    wire_tree_inner: Rc<dyn IPen>,
    wire_selected: Rc<dyn IPen>,
    wire_selected_inner: Rc<dyn IPen>,
    wire_faint: Rc<dyn IPen>,
    wire_preview_ok: Rc<dyn IPen>,
    wire_preview_bad: Rc<dyn IPen>,
    window_fill: Rc<dyn IBrush>,
    window_border: Rc<dyn IPen>,
    crossing_fill: Rc<dyn IBrush>,
    crossing_border: Rc<dyn IPen>,
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
    badge_bg: Rc<dyn IBrush>,
}

impl ThemeBrushes {
    /// Neutral vector strokes/grid brushes for stock Light `ThemeVariant`.
    fn stock_light() -> Self {
        let dashed_tree = Rc::new(ImmutableDashStyle::new(Some(&[5.0, 3.5]), 0.0));
        let dashed_empty = Rc::new(ImmutableDashStyle::new(Some(&[4.0, 3.5]), 0.0));
        let dashed_cross = Rc::new(ImmutableDashStyle::new(Some(&[5.0, 3.5]), 0.0));
        let empty_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFE07A00));
        let tree_outer_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF3A3A3A));
        let tree_inner_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFF3F3F3));
        let cross_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF16A34A));
        Self {
            bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0x00000000)),
            minor_grid: Rc::new(ImmutablePen::from_uint32(0x14000000, 1.0)),
            major_grid: Rc::new(ImmutablePen::from_uint32(0x24000000, 1.0)),
            origin_pen: Rc::new(ImmutablePen::from_uint32(0x3C000000, 1.4)),
            wire_empty: Rc::new(ImmutablePen::new(
                Some(empty_br),
                1.9,
                Some(dashed_empty),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_item: Rc::new(ImmutablePen::from_uint32(0xCC3A3A3A, 2.0)),
            wire_list_outer: Rc::new(ImmutablePen::from_uint32(0xDD343434, 4.6)),
            wire_list_inner: Rc::new(ImmutablePen::from_uint32(0xFFF3F3F3, 1.8)),
            wire_tree_outer: Rc::new(ImmutablePen::new(
                Some(tree_outer_br),
                5.0,
                Some(dashed_tree.clone()),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_tree_inner: Rc::new(ImmutablePen::new(
                Some(tree_inner_br),
                1.8,
                Some(dashed_tree),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_selected: Rc::new(ImmutablePen::from_uint32(0xFF16A34A, 3.6)),
            wire_selected_inner: Rc::new(ImmutablePen::from_uint32(0xFFDCFCE7, 1.3)),
            wire_faint: Rc::new(ImmutablePen::from_uint32(0x44484848, 1.2)),
            wire_preview_ok: Rc::new(ImmutablePen::from_uint32(0xFF16A34A, 2.8)),
            wire_preview_bad: Rc::new(ImmutablePen::from_uint32(0xFFDC2626, 2.8)),
            window_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x220078D4)),
            window_border: Rc::new(ImmutablePen::from_uint32(0xCC0078D4, 1.4)),
            crossing_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x2216A34A)),
            crossing_border: Rc::new(ImmutablePen::new(
                Some(cross_br),
                1.5,
                Some(dashed_cross),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            minimap_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xDCF3F3F3)),
            minimap_border: Rc::new(ImmutablePen::from_uint32(0x38000000, 1.0)),
            minimap_view: Rc::new(ImmutablePen::from_uint32(0xFF0078D4, 1.5)),
            text_muted: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF5A5A5A)),
            text_accent: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF1F1F1F)),
            text_primary: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF1A1A1A)),
            tooltip_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xF5FFFFFF)),
            tooltip_border: Rc::new(ImmutablePen::from_uint32(0x38000000, 1.0)),
            snap_ok_pen: Rc::new(ImmutablePen::from_uint32(0xFF16A34A, 2.6)),
            snap_bad_pen: Rc::new(ImmutablePen::from_uint32(0xFFDC2626, 2.6)),
            node_dot: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF666666)),
            node_dot_sel: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF16A34A)),
            badge_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF16A34A)),
        }
    }

    /// Neutral vector strokes/grid brushes for stock Dark `ThemeVariant`.
    fn stock_dark() -> Self {
        let dashed_tree = Rc::new(ImmutableDashStyle::new(Some(&[5.0, 3.5]), 0.0));
        let dashed_empty = Rc::new(ImmutableDashStyle::new(Some(&[4.0, 3.5]), 0.0));
        let dashed_cross = Rc::new(ImmutableDashStyle::new(Some(&[5.0, 3.5]), 0.0));
        let empty_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xCCF59E0B));
        let tree_outer_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xDD93C5FD));
        let tree_inner_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF202020));
        let cross_br: Rc<dyn IImmutableBrush> =
            Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF34D399));
        Self {
            bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0x00000000)),
            minor_grid: Rc::new(ImmutablePen::from_uint32(0x10FFFFFF, 1.0)),
            major_grid: Rc::new(ImmutablePen::from_uint32(0x1FFFFFFF, 1.0)),
            origin_pen: Rc::new(ImmutablePen::from_uint32(0x34FFFFFF, 1.4)),
            wire_empty: Rc::new(ImmutablePen::new(
                Some(empty_br),
                1.8,
                Some(dashed_empty),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_item: Rc::new(ImmutablePen::from_uint32(0xCCA8A8A8, 2.2)),
            wire_list_outer: Rc::new(ImmutablePen::from_uint32(0xDD60CDFF, 4.8)),
            wire_list_inner: Rc::new(ImmutablePen::from_uint32(0xFF202020, 1.9)),
            wire_tree_outer: Rc::new(ImmutablePen::new(
                Some(tree_outer_br),
                5.0,
                Some(dashed_tree.clone()),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_tree_inner: Rc::new(ImmutablePen::new(
                Some(tree_inner_br),
                1.9,
                Some(dashed_tree),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_selected: Rc::new(ImmutablePen::from_uint32(0xFF22C55E, 3.6)),
            wire_selected_inner: Rc::new(ImmutablePen::from_uint32(0xFF202020, 1.3)),
            wire_faint: Rc::new(ImmutablePen::from_uint32(0x44A8A8A8, 1.2)),
            wire_preview_ok: Rc::new(ImmutablePen::from_uint32(0xFF34D399, 2.8)),
            wire_preview_bad: Rc::new(ImmutablePen::from_uint32(0xFFF43F5E, 2.8)),
            window_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x2260CDFF)),
            window_border: Rc::new(ImmutablePen::from_uint32(0xCC60CDFF, 1.4)),
            crossing_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x2234D399)),
            crossing_border: Rc::new(ImmutablePen::new(
                Some(cross_br),
                1.5,
                Some(dashed_cross),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            minimap_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xDC242424)),
            minimap_border: Rc::new(ImmutablePen::from_uint32(0x2EFFFFFF, 1.0)),
            minimap_view: Rc::new(ImmutablePen::from_uint32(0xCC60CDFF, 1.5)),
            text_muted: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFB4B4B4)),
            text_accent: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF60CDFF)),
            text_primary: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFF3F3F3)),
            tooltip_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xF02B2B2B)),
            tooltip_border: Rc::new(ImmutablePen::from_uint32(0x38FFFFFF, 1.0)),
            snap_ok_pen: Rc::new(ImmutablePen::from_uint32(0xFF10B981, 2.6)),
            snap_bad_pen: Rc::new(ImmutablePen::from_uint32(0xFFF43F5E, 2.6)),
            node_dot: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF888888)),
            node_dot_sel: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF22C55E)),
            badge_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF10B981)),
        }
    }
}

#[repr(C)]
pub struct WireBackdropLayer {
    base: Control,
    state: RefCell<Option<VisualGraphState>>,
    drag: RefCell<BackdropDrag>,
    light: ThemeBrushes,
    dark: ThemeBrushes,
}

ferro_class!(WireBackdropLayer: Control);
ferro_impl_classes!(
    WireBackdropLayer: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(WireBackdropLayer {
    new: WireBackdropLayer::new
});

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
            light: ThemeBrushes::stock_light(),
            dark: ThemeBrushes::stock_dark(),
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
        Rect::new(
            (b.width - 178.0).max(8.0),
            (b.height - 114.0).max(8.0),
            166.0,
            102.0,
        )
    }

    fn hit_test_wire_screen(&self, st: &VisualGraphState, screen_pt: Point) -> Option<WireId> {
        let g = st.graph.borrow();
        let thresh2 = 10.0 * 10.0;
        for (_, w) in &g.wires {
            let Some(p0) =
                st.port_screen_pos(self, w.from_node, PortDirection::Output, w.from_port)
            else {
                continue;
            };
            let Some(p3) = st.port_screen_pos(self, w.to_node, PortDirection::Input, w.to_port)
            else {
                continue;
            };
            let dx = ((p3.x - p0.x).abs() * 0.48).clamp(32.0, 180.0);
            let p1 = Point::new(p0.x + dx, p0.y);
            let p2 = Point::new(p3.x - dx, p3.y);
            for step in 0..=18 {
                let t = step as f64 / 18.0;
                let u = 1.0 - t;
                let bx = u * u * u * p0.x
                    + 3.0 * u * u * t * p1.x
                    + 3.0 * u * t * t * p2.x
                    + t * t * t * p3.x;
                let by = u * u * u * p0.y
                    + 3.0 * u * u * t * p1.y
                    + 3.0 * u * t * t * p2.y
                    + t * t * t * p3.y;
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
        let Some([min_x, min_y, max_x, max_y]) = g.content_bounds() else {
            return;
        };
        let pad = 120.0;
        let bx0 = min_x - pad;
        let by0 = min_y - pad;
        let bw = (max_x - min_x + pad * 2.0).max(220.0);
        let bh = (max_y - min_y + pad * 2.0).max(160.0);
        let nx = ((screen_pt.x - mm.x) / mm.width).clamp(0.0, 1.0);
        let ny = ((screen_pt.y - mm.y) / mm.height).clamp(0.0, 1.0);
        let cx = bx0 + nx * bw;
        let cy = by0 + ny * bh;
        g.camera_pan = [
            b.width * 0.5 - cx * g.camera_zoom,
            b.height * 0.5 - cy * g.camera_zoom,
        ];
        drop(g);
        st.sync_camera_transform();
    }
}

impl InputElementImpl for WireBackdropLayer {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        this.focus();
        let Some(st) = this.state.borrow().clone() else {
            return;
        };
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

        let mm = this.minimap_rect();
        if !is_right && mm.contains(pt) && this.bounds().width > 340.0 {
            *this.drag.borrow_mut() = BackdropDrag::Minimap;
            this.jump_minimap(&st, pt);
            return;
        }

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
            *this.drag.borrow_mut() = BackdropDrag::Panning {
                start_screen: pt,
                start_pan,
            };
            return;
        }

        if is_middle || alt {
            let start_pan = st.graph.borrow().camera_pan;
            *this.drag.borrow_mut() = BackdropDrag::Panning {
                start_screen: pt,
                start_pan,
            };
            return;
        }

        // Click or double-click wire (double-click drops inline Grasshopper Relay!)
        if let Some(wid) = this.hit_test_wire_screen(&st, pt) {
            if e.click_count() >= 2 {
                let c = st.screen_to_canvas(pt);
                if st
                    .graph
                    .borrow_mut()
                    .insert_relay_on_wire(wid, c[0], c[1], &st.registry)
                    .is_some()
                {
                    st.notify_topology_changed();
                    st.status_msg
                        .set("Inserted Grasshopper Relay node inline on wire.".into());
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

        if e.click_count() >= 2 {
            let c = st.screen_to_canvas(pt);
            st.quick_search.set(Some((pt, c)));
            return;
        }

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
        let Some(st) = this.state.borrow().clone() else {
            return;
        };
        let pt = e.get_position(Some(this));
        match this.drag.borrow().clone() {
            BackdropDrag::Idle => {}
            BackdropDrag::Minimap => this.jump_minimap(&st, pt),
            BackdropDrag::Panning {
                start_screen,
                start_pan,
            } => {
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
                let crossing = pt.x < start_screen.x; // Rhino CAD convention: Right→Left = Crossing!
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
                            let nx0 = n.position[0];
                            let ny0 = n.position[1];
                            let nx1 = nx0 + n.size[0];
                            let ny1 = ny0 + n.size[1];
                            if crossing {
                                nx1 >= x0 && nx0 <= x1 && ny1 >= y0 && ny0 <= y1
                            } else {
                                nx0 >= x0 && nx1 <= x1 && ny0 >= y0 && ny1 <= y1
                            }
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
        let Some(st) = this.state.borrow().clone() else {
            return;
        };
        let cmd = e.key_modifiers.contains(KeyModifiers::CONTROL)
            || e.key_modifiers.contains(KeyModifiers::META);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);

        match e.key {
            Key::Space => {
                let b = this.bounds();
                let center = Point::new(b.width * 0.5 - 124.0, b.height * 0.34);
                st.quick_search
                    .set(Some((center, st.screen_to_canvas(center))));
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
                    st.notify_history_restored();
                    st.sync_all_node_positions();
                    e.set_handled(true);
                }
            }
            Key::Z if cmd => {
                if st.graph.borrow_mut().undo() {
                    st.notify_history_restored();
                    st.sync_all_node_positions();
                    e.set_handled(true);
                }
            }
            Key::Y if cmd => {
                if st.graph.borrow_mut().redo() {
                    st.notify_history_restored();
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
                if st
                    .graph
                    .borrow_mut()
                    .group_selection("Group", 0x337C3AED)
                    .is_some()
                {
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

        let is_dark = this
            .get_value(ThemeVariant::actual_theme_variant_property())
            .or_else(|| Application::current().and_then(|app| app.actual_theme_variant()))
            .is_some_and(|v| v == ThemeVariant::dark());

        let Some(st) = this.state.borrow().clone() else {
            return;
        };
        if st.is_dark_theme.is_alive() && st.is_dark_theme.get_untracked() != is_dark {
            st.is_dark_theme.set(is_dark);
        }
        let g = st.graph.borrow();
        let pal = if is_dark { &this.dark } else { &this.light };

        // 1. Rhino Grasshopper Drafting Canvas & Adaptive Grid
        dc.fill_rectangle(&pal.bg, Rect::new(0.0, 0.0, w, h), 0.0);
        let z = g.camera_zoom.clamp(0.25, 2.8);
        let minor = 24.0 * z;
        let major = minor * 5.0;
        if minor >= 9.0 {
            let mut x = g.camera_pan[0].rem_euclid(minor);
            while x < w {
                dc.draw_line(&pal.minor_grid, Point::new(x, 0.0), Point::new(x, h));
                x += minor;
            }
            let mut y = g.camera_pan[1].rem_euclid(minor);
            while y < h {
                dc.draw_line(&pal.minor_grid, Point::new(0.0, y), Point::new(w, y));
                y += minor;
            }
        }
        let mut x = g.camera_pan[0].rem_euclid(major);
        while x < w {
            dc.draw_line(&pal.major_grid, Point::new(x, 0.0), Point::new(x, h));
            x += major;
        }
        let mut y = g.camera_pan[1].rem_euclid(major);
        while y < h {
            dc.draw_line(&pal.major_grid, Point::new(0.0, y), Point::new(w, y));
            y += major;
        }
        // Subtle canvas origin axes (0, 0)
        if g.camera_pan[0] >= 0.0 && g.camera_pan[0] <= w {
            dc.draw_line(
                &pal.origin_pen,
                Point::new(g.camera_pan[0], 0.0),
                Point::new(g.camera_pan[0], h),
            );
        }
        if g.camera_pan[1] >= 0.0 && g.camera_pan[1] <= h {
            dc.draw_line(
                &pal.origin_pen,
                Point::new(0.0, g.camera_pan[1]),
                Point::new(w, g.camera_pan[1]),
            );
        }

        // 2. Authentic Rhino Grasshopper Pastel Group Envelopes
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
                let pad_x = 18.0 * z;
                let pad_top = 28.0 * z;
                let pad_bot = 16.0 * z;
                let rect = Rect::new(
                    min_x - pad_x,
                    min_y - pad_top,
                    (max_x - min_x) + pad_x * 2.0,
                    (max_y - min_y) + pad_top + pad_bot,
                );
                let fill_col = if is_dark {
                    (group.color_argb & 0x00FF_FFFF) | 0x2600_0000
                } else {
                    (group.color_argb & 0x00FF_FFFF) | 0x2000_0000
                };
                let border_col = (group.color_argb & 0x00FF_FFFF) | 0x9900_0000;
                let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(fill_col));
                let border_pen: Rc<dyn IPen> = Rc::new(ImmutablePen::from_uint32(border_col, 1.4));
                dc.draw_rectangle(
                    Some(&fill),
                    Some(&border_pen),
                    rect,
                    14.0,
                    14.0,
                    &BoxShadows::default(),
                );
                draw_text(
                    &mut dc,
                    &group.title,
                    Point::new(rect.x + 12.0, rect.y + 6.0),
                    (12.0 * z).clamp(9.5, 16.0),
                    &pal.text_accent,
                );
            }
        }
        drop(node_vis_map);

        // 3. Signature Grasshopper Fancy Bezier Wires anchored to real FerroUI port sockets!
        for wire in g.wires.values() {
            if wire.display == WireDisplay::Hidden {
                continue;
            }
            let Some(p0) =
                st.port_screen_pos(this, wire.from_node, PortDirection::Output, wire.from_port)
            else {
                continue;
            };
            let Some(p3) =
                st.port_screen_pos(this, wire.to_node, PortDirection::Input, wire.to_port)
            else {
                continue;
            };
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
                pal,
                p0,
                p3,
                data_kind,
                g.fancy_wires,
                wire.display == WireDisplay::Faint,
                is_sel,
            );
        }

        // 4. Live Drag-to-Wire Preview + Magnetic Target Halo + Shift `+` Badge
        if let Some(drag) = st.active_wire.borrow().as_ref() {
            if let Some(anchor) =
                st.port_screen_pos(this, drag.from_node, drag.from_dir, drag.from_port)
            {
                let (end_pt, ok) = match drag.snap_target {
                    Some((_, _, _, sp, compat)) => (sp, compat),
                    None => (drag.cursor_screen, true),
                };
                let (p0, p3) = match drag.from_dir {
                    PortDirection::Output => (anchor, end_pt),
                    PortDirection::Input => (end_pt, anchor),
                };
                let pen = if ok {
                    &pal.wire_preview_ok
                } else {
                    &pal.wire_preview_bad
                };
                draw_bezier(&mut dc, pen, p0, p3);
                if let Some((_, _, _, sp, compat)) = drag.snap_target {
                    let ring = if compat {
                        &pal.snap_ok_pen
                    } else {
                        &pal.snap_bad_pen
                    };
                    dc.draw_ellipse_at(None, Some(ring), sp, 9.5, 9.5);
                }
                if drag.multi_wire {
                    let badge = Rect::new(end_pt.x + 10.0, end_pt.y - 10.0, 18.0, 18.0);
                    dc.fill_rectangle(&pal.badge_bg, badge, 9.0);
                    draw_text(
                        &mut dc,
                        "+",
                        Point::new(badge.x + 5.0, badge.y + 1.0),
                        12.0,
                        &this.dark.text_primary,
                    );
                }
            }
        }

        // 5. Authentic Rhino Window (Blue, L→R) vs Crossing (Green Dashed, R→L) Marquee!
        if let Some((s0, s1)) = *st.marquee.borrow() {
            let crossing = s1.x < s0.x;
            let rect = Rect::new(
                s0.x.min(s1.x),
                s0.y.min(s1.y),
                (s1.x - s0.x).abs(),
                (s1.y - s0.y).abs(),
            );
            if crossing {
                dc.draw_rectangle(
                    Some(&pal.crossing_fill),
                    Some(&pal.crossing_border),
                    rect,
                    2.0,
                    2.0,
                    &BoxShadows::default(),
                );
            } else {
                dc.draw_rectangle(
                    Some(&pal.window_fill),
                    Some(&pal.window_border),
                    rect,
                    2.0,
                    2.0,
                    &BoxShadows::default(),
                );
            }
        }

        // 6. Interactive Minimap (bottom-right)
        if w > 340.0 && h > 220.0 {
            let mm = this.minimap_rect();
            dc.draw_rectangle(
                Some(&pal.minimap_bg),
                Some(&pal.minimap_border),
                mm,
                7.0,
                7.0,
                &BoxShadows::default(),
            );
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
                        &pal.node_dot_sel
                    } else {
                        &pal.node_dot
                    };
                    dc.fill_rectangle(br, Rect::new(nx, ny, nw, nh), 1.5);
                }
                let vx0 = (-g.camera_pan[0]) / z;
                let vy0 = (-g.camera_pan[1]) / z;
                let rx = (mm.x + (vx0 - bx0) * sx).clamp(mm.x, mm.x + mm.width - 8.0);
                let ry = (mm.y + (vy0 - by0) * sy).clamp(mm.y, mm.y + mm.height - 8.0);
                let rw = ((w / z) * sx).clamp(8.0, mm.x + mm.width - rx);
                let rh = ((h / z) * sy).clamp(8.0, mm.y + mm.height - ry);
                dc.draw_rectangle_outline(&pal.minimap_view, Rect::new(rx, ry, rw, rh), 2.0);
            }
        }

        // 7. Crisp Grasshopper Port Hover DataTree Tooltip
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
                                .map(|t| {
                                    format!(
                                        "{} br · {} items · {}",
                                        t.branch_count(),
                                        t.item_count(),
                                        t.format_panel(1)
                                    )
                                })
                                .unwrap_or_else(|| "Empty parameter".into()),
                            PortDirection::Input => {
                                let wcnt = g.incoming_wires(nid, pidx).len();
                                let m = node
                                    .input_modifiers
                                    .get(pidx)
                                    .map(|x| x.badge())
                                    .unwrap_or_default();
                                if m.is_empty() {
                                    format!(
                                        "{wcnt} source(s) · default {}",
                                        spec.default_value.short_summary()
                                    )
                                } else {
                                    format!("{wcnt} source(s) · modifier [{m}]")
                                }
                            }
                        };
                        let tx = (spt.x + 14.0).min(w - 262.0).max(8.0);
                        let ty = (spt.y - 48.0).clamp(8.0, h - 58.0);
                        dc.draw_rectangle(
                            Some(&pal.tooltip_bg),
                            Some(&pal.tooltip_border),
                            Rect::new(tx, ty, 252.0, 44.0),
                            5.0,
                            5.0,
                            &BoxShadows::default(),
                        );
                        draw_text(
                            &mut dc,
                            &format!(
                                "{} ({}) · {}",
                                spec.name,
                                spec.nickname,
                                spec.port_type.label()
                            ),
                            Point::new(tx + 8.0, ty + 5.0),
                            10.5,
                            &pal.text_accent,
                        );
                        let short: String = detail.chars().take(38).collect();
                        draw_text(
                            &mut dc,
                            &short,
                            Point::new(tx + 8.0, ty + 22.0),
                            10.0,
                            &pal.text_primary,
                        );
                    }
                }
            }
        }

        // 8. Subtle bottom-left status watermark
        draw_text(
            &mut dc,
            &format!(
                "{:.0}% · Double-click / Space Search · Double-click Wire = Relay · L→R Window / R→L Crossing Select · Right-drag Pan · Wheel Zoom",
                z * 100.0
            ),
            Point::new(12.0, h - 20.0),
            10.0,
            &pal.text_muted,
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
    pal: &ThemeBrushes,
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
            draw_bezier(dc, &pal.wire_selected_inner, p0, p3);
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
