//! Hardware-rendered **Rhino Grasshopper-style `NodeGraphCanvas`** FerroUI custom `Control`.
//!
//! Features:
//! - Infinite pan (right/middle drag or Alt+drag) & cursor-anchored wheel zoom (`0.25x..=3.0x`)
//! - Adaptive Grasshopper coordinate grid, colored node groups, and interactive bottom-right Minimap
//! - **Fancy Wires** via `StreamGeometry::cubic_bezier_to` (single Item wire, double-line List wire,
//!   dashed double-line DataTree wire, amber Empty wire, and glowing selected/drag preview wire)
//! - Authentic **Grasshopper 3-column node capsules** + **inline canvas widgets**:
//!   draggable `NumberSlider`, clickable `BoolToggle`, `ColorSwatch`, multiline `Panel`,
//!   `ExpressionBox`, compact `Relay`, and frameless `Scribble` notes
//! - Live microsecond **Profiler pills**, port hover **DataTree tooltips**, **Marquee selection**,
//!   Shift-multi-wire connections, right-click port/wire disconnect, double-click quick search,
//!   and full keyboard shortcuts (`Del`, `Ctrl/Cmd+Z/Y`, `Ctrl/Cmd+D`, `Ctrl/Cmd+G`, `Ctrl/Cmd+E`, `F`, `L`).

use crate::eval::evaluate_graph;
use crate::graph::{NodeGraph, NodeInstance};
use crate::node::NodeRegistry;
use crate::types::{
    NodeId, NodeKind, NodeState, ParamWidget, PortDirection, WireDataKind, WireDisplay, WireId,
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
    PenLineCap, StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point,
    Rect, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_view::ViewClass;
use std::cell::RefCell;
use std::rc::Rc;

const SWATCH_PALETTE: [u32; 8] = [
    0xFF38BDF8, 0xFF34D399, 0xFFA78BFA, 0xFFF472B6, 0xFFFBBF24, 0xFFFB923C, 0xFFFB7185, 0xFF2DD4BF,
];

#[derive(Clone, Debug)]
enum InteractionMode {
    Idle,
    Panning {
        start_screen: Point,
        start_pan: [f64; 2],
    },
    DraggingNodes {
        start_canvas: [f64; 2],
        originals: Vec<(NodeId, [f64; 2])>,
    },
    DraggingSlider {
        node_id: NodeId,
        track_left: f64,
        track_width: f64,
    },
    Wiring {
        from_node: NodeId,
        from_port: usize,
        from_dir: PortDirection,
        cursor_canvas: [f64; 2],
        multi_wire: bool,
    },
    Marquee {
        start_canvas: [f64; 2],
        current_canvas: [f64; 2],
    },
    MinimapDrag,
}

#[derive(Clone, Debug)]
struct HoverTarget {
    node_id: NodeId,
    port_idx: usize,
    dir: PortDirection,
    screen_pt: Point,
}

struct CanvasPalette {
    bg: Rc<dyn IBrush>,
    minor_grid: Rc<dyn IPen>,
    major_grid: Rc<dyn IPen>,
    shadow: Rc<dyn IBrush>,
    node_body: Rc<dyn IBrush>,
    node_center: Rc<dyn IBrush>,
    node_selected_border: Rc<dyn IPen>,
    node_normal_border: Rc<dyn IPen>,
    node_warn_border: Rc<dyn IPen>,
    node_err_border: Rc<dyn IPen>,
    node_disabled_fill: Rc<dyn IBrush>,
    text_primary: Rc<dyn IBrush>,
    text_muted: Rc<dyn IBrush>,
    text_accent: Rc<dyn IBrush>,
    slider_track: Rc<dyn IBrush>,
    slider_fill: Rc<dyn IBrush>,
    slider_thumb: Rc<dyn IBrush>,
    panel_bg: Rc<dyn IBrush>,
    panel_border: Rc<dyn IPen>,
    wire_empty: Rc<dyn IPen>,
    wire_item: Rc<dyn IPen>,
    wire_list_outer: Rc<dyn IPen>,
    wire_list_inner: Rc<dyn IPen>,
    wire_tree_outer: Rc<dyn IPen>,
    wire_tree_inner: Rc<dyn IPen>,
    wire_selected: Rc<dyn IPen>,
    wire_faint: Rc<dyn IPen>,
    wire_preview: Rc<dyn IPen>,
    marquee_fill: Rc<dyn IBrush>,
    marquee_border: Rc<dyn IPen>,
    minimap_bg: Rc<dyn IBrush>,
    minimap_border: Rc<dyn IPen>,
    minimap_view: Rc<dyn IPen>,
    tooltip_bg: Rc<dyn IBrush>,
    tooltip_border: Rc<dyn IPen>,
    profiler_bg: Rc<dyn IBrush>,
    socket_ring: Rc<dyn IPen>,
}

impl CanvasPalette {
    fn new() -> Self {
        let dashed_tree = Rc::new(ImmutableDashStyle::new(Some(&[5.0, 3.5]), 0.0));
        let dashed_empty = Rc::new(ImmutableDashStyle::new(Some(&[4.0, 4.0]), 0.0));
        let empty_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xCCF59E0B));
        let tree_outer_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xDDA78BFA));
        let tree_inner_br: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0B1120));
        Self {
            bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0A0F1A)),
            minor_grid: Rc::new(ImmutablePen::from_uint32(0xFF111827, 1.0)),
            major_grid: Rc::new(ImmutablePen::from_uint32(0xFF1A2438, 1.0)),
            shadow: Rc::new(ImmutableSolidColorBrush::from_uint32(0x66000000)),
            node_body: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF161F33)),
            node_center: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0F1625)),
            node_selected_border: Rc::new(ImmutablePen::from_uint32(0xFF10B981, 2.2)),
            node_normal_border: Rc::new(ImmutablePen::from_uint32(0xFF334155, 1.2)),
            node_warn_border: Rc::new(ImmutablePen::from_uint32(0xFFF59E0B, 2.0)),
            node_err_border: Rc::new(ImmutablePen::from_uint32(0xFFF43F5E, 2.2)),
            node_disabled_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xAA0B101B)),
            text_primary: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFF1F5F9)),
            text_muted: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF94A3B8)),
            text_accent: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF7DD3FC)),
            slider_track: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0B1120)),
            slider_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0284C7)),
            slider_thumb: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF38BDF8)),
            panel_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF1C2318)),
            panel_border: Rc::new(ImmutablePen::from_uint32(0xFF65A30D, 1.3)),
            wire_empty: Rc::new(ImmutablePen::new(
                Some(empty_br),
                1.6,
                Some(dashed_empty),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_item: Rc::new(ImmutablePen::from_uint32(0xCC94A3B8, 2.0)),
            wire_list_outer: Rc::new(ImmutablePen::from_uint32(0xDD38BDF8, 4.4)),
            wire_list_inner: Rc::new(ImmutablePen::from_uint32(0xFF0B1120, 1.7)),
            wire_tree_outer: Rc::new(ImmutablePen::new(
                Some(tree_outer_br),
                4.6,
                Some(dashed_tree.clone()),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_tree_inner: Rc::new(ImmutablePen::new(
                Some(tree_inner_br),
                1.7,
                Some(dashed_tree),
                PenLineCap::Round,
                Default::default(),
                4.0,
            )),
            wire_selected: Rc::new(ImmutablePen::from_uint32(0xFF10B981, 3.2)),
            wire_faint: Rc::new(ImmutablePen::from_uint32(0x4494A3B8, 1.2)),
            wire_preview: Rc::new(ImmutablePen::from_uint32(0xFF38BDF8, 2.6)),
            marquee_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x2238BDF8)),
            marquee_border: Rc::new(ImmutablePen::from_uint32(0xCC38BDF8, 1.3)),
            minimap_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xD80B101D)),
            minimap_border: Rc::new(ImmutablePen::from_uint32(0xFF2B3B58, 1.2)),
            minimap_view: Rc::new(ImmutablePen::from_uint32(0xCC38BDF8, 1.4)),
            tooltip_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xEE0F172A)),
            tooltip_border: Rc::new(ImmutablePen::from_uint32(0xFF38BDF8, 1.2)),
            profiler_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xCC1E293B)),
            socket_ring: Rc::new(ImmutablePen::from_uint32(0xFF090D16, 1.5)),
        }
    }
}

/// Interactive Rhino Grasshopper-style FerroUI canvas control.
#[repr(C)]
pub struct NodeGraphCanvas {
    base: Control,
    graph: RefCell<Rc<RefCell<NodeGraph>>>,
    registry: RefCell<Rc<NodeRegistry>>,
    mode: RefCell<InteractionMode>,
    hover_port: RefCell<Option<HoverTarget>>,
    on_changed: RefCell<Option<Rc<dyn Fn(&NodeGraph)>>>,
    on_double_click_empty: RefCell<Option<Rc<dyn Fn(f64, f64)>>>,
    palette: CanvasPalette,
}

ferro_class!(NodeGraphCanvas: Control);
ferro_impl_classes!(
    NodeGraphCanvas: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(NodeGraphCanvas { new: NodeGraphCanvas::new });

impl ViewClass for NodeGraphCanvas {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

impl NodeGraphCanvas {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            graph: RefCell::new(Rc::new(RefCell::new(NodeGraph::default()))),
            registry: RefCell::new(Rc::new(NodeRegistry::with_standard_library())),
            mode: RefCell::new(InteractionMode::Idle),
            hover_port: RefCell::new(None),
            on_changed: RefCell::new(None),
            on_double_click_empty: RefCell::new(None),
            palette: CanvasPalette::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(true);
        this.set_focusable(true);
        this
    }

    /// Binds a shared [`NodeGraph`] and [`NodeRegistry`] to this canvas, plus a callback fired
    /// whenever the user drags a slider, wires/unwires nodes, moves, or edits the graph.
    pub fn bind(
        &self,
        graph: Rc<RefCell<NodeGraph>>,
        registry: Rc<NodeRegistry>,
        on_changed: Rc<dyn Fn(&NodeGraph)>,
    ) {
        *self.graph.borrow_mut() = graph;
        *self.registry.borrow_mut() = registry;
        *self.on_changed.borrow_mut() = Some(on_changed);
        self.reevaluate_and_notify();
        self.invalidate_visual();
    }

    pub fn set_on_double_click_empty(&self, cb: Rc<dyn Fn(f64, f64)>) {
        *self.on_double_click_empty.borrow_mut() = Some(cb);
    }

    pub fn reevaluate_and_notify(&self) {
        let g_rc = self.graph.borrow().clone();
        let reg = self.registry.borrow().clone();
        {
            let mut g = g_rc.borrow_mut();
            let _ = evaluate_graph(&mut g, &reg, 0.0);
        }
        if let Some(cb) = self.on_changed.borrow().as_ref() {
            cb(&g_rc.borrow());
        }
        self.invalidate_visual();
    }

    pub fn fit_view(&self) {
        let b = self.bounds();
        let w = b.width.max(400.0);
        let h = b.height.max(300.0);
        self.graph.borrow().borrow_mut().fit_to_view(w, h);
        self.invalidate_visual();
    }

    fn screen_to_canvas(&self, pt: Point) -> [f64; 2] {
        let g = self.graph.borrow();
        let g = g.borrow();
        let z = g.camera_zoom.max(0.1);
        [(pt.x - g.camera_pan[0]) / z, (pt.y - g.camera_pan[1]) / z]
    }

    fn port_position_canvas(
        node: &NodeInstance,
        reg: &NodeRegistry,
        dir: PortDirection,
        port_idx: usize,
    ) -> [f64; 2] {
        let Some(comp) = reg.get(&node.type_id) else {
            return [node.position[0], node.position[1] + node.size[1] * 0.5];
        };
        let count = match dir {
            PortDirection::Input => comp.inputs().len(),
            PortDirection::Output => comp.outputs().len(),
        };
        let x = match dir {
            PortDirection::Input => node.position[0],
            PortDirection::Output => node.position[0] + node.size[0],
        };
        if count <= 1 {
            return [x, node.position[1] + node.size[1] * 0.5];
        }
        let pad_top = 14.0;
        let pad_bot = 14.0;
        let usable = (node.size[1] - pad_top - pad_bot).max(1.0);
        let t = port_idx as f64 / (count - 1) as f64;
        [x, node.position[1] + pad_top + usable * t]
    }

    fn hit_test_port(&self, canvas_pt: [f64; 2]) -> Option<(NodeId, usize, PortDirection)> {
        let g_rc = self.graph.borrow();
        let g = g_rc.borrow();
        let reg = self.registry.borrow();
        let hit_r = 12.0 / g.camera_zoom.clamp(0.45, 2.0);
        let hit_r2 = hit_r * hit_r;

        for (_, node) in g.nodes.iter().rev() {
            let Some(comp) = reg.get(&node.type_id) else { continue };
            for i in 0..comp.outputs().len() {
                let p = Self::port_position_canvas(node, &reg, PortDirection::Output, i);
                let d2 = (p[0] - canvas_pt[0]).powi(2) + (p[1] - canvas_pt[1]).powi(2);
                if d2 <= hit_r2 {
                    return Some((node.id, i, PortDirection::Output));
                }
            }
            for i in 0..comp.inputs().len() {
                let p = Self::port_position_canvas(node, &reg, PortDirection::Input, i);
                let d2 = (p[0] - canvas_pt[0]).powi(2) + (p[1] - canvas_pt[1]).powi(2);
                if d2 <= hit_r2 {
                    return Some((node.id, i, PortDirection::Input));
                }
            }
        }
        None
    }

    fn hit_test_node(&self, canvas_pt: [f64; 2]) -> Option<NodeId> {
        let g_rc = self.graph.borrow();
        let g = g_rc.borrow();
        for (_, node) in g.nodes.iter().rev() {
            if canvas_pt[0] >= node.position[0]
                && canvas_pt[0] <= node.position[0] + node.size[0]
                && canvas_pt[1] >= node.position[1]
                && canvas_pt[1] <= node.position[1] + node.size[1]
            {
                return Some(node.id);
            }
        }
        None
    }

    fn hit_test_wire(&self, canvas_pt: [f64; 2]) -> Option<WireId> {
        let g_rc = self.graph.borrow();
        let g = g_rc.borrow();
        let reg = self.registry.borrow();
        let thresh2 = (9.0 / g.camera_zoom.clamp(0.45, 2.0)).powi(2);

        for (_, w) in &g.wires {
            let (Some(n0), Some(n1)) = (g.nodes.get(&w.from_node), g.nodes.get(&w.to_node)) else {
                continue;
            };
            let p0 = Self::port_position_canvas(n0, &reg, PortDirection::Output, w.from_port);
            let p3 = Self::port_position_canvas(n1, &reg, PortDirection::Input, w.to_port);
            let dx = ((p3[0] - p0[0]).abs() * 0.48).clamp(36.0, 180.0);
            let p1 = [p0[0] + dx, p0[1]];
            let p2 = [p3[0] - dx, p3[1]];

            for step in 0..=16 {
                let t = step as f64 / 16.0;
                let u = 1.0 - t;
                let bx = u * u * u * p0[0]
                    + 3.0 * u * u * t * p1[0]
                    + 3.0 * u * t * t * p2[0]
                    + t * t * t * p3[0];
                let by = u * u * u * p0[1]
                    + 3.0 * u * u * t * p1[1]
                    + 3.0 * u * t * t * p2[1]
                    + t * t * t * p3[1];
                let d2 = (bx - canvas_pt[0]).powi(2) + (by - canvas_pt[1]).powi(2);
                if d2 <= thresh2 {
                    return Some(w.id);
                }
            }
        }
        None
    }

    fn minimap_rect(&self) -> Rect {
        let b = self.bounds();
        Rect::new((b.width - 178.0).max(8.0), (b.height - 114.0).max(8.0), 166.0, 102.0)
    }

    fn jump_camera_from_minimap(&self, screen_pt: Point) {
        let mm = self.minimap_rect();
        let b = self.bounds();
        let g_rc = self.graph.borrow();
        let mut g = g_rc.borrow_mut();
        let Some([min_x, min_y, max_x, max_y]) = g.content_bounds() else { return };
        let pad = 120.0;
        let bx0 = min_x - pad;
        let by0 = min_y - pad;
        let bw = (max_x - min_x + pad * 2.0).max(200.0);
        let bh = (max_y - min_y + pad * 2.0).max(160.0);

        let nx = ((screen_pt.x - mm.x) / mm.width).clamp(0.0, 1.0);
        let ny = ((screen_pt.y - mm.y) / mm.height).clamp(0.0, 1.0);
        let target_cx = bx0 + nx * bw;
        let target_cy = by0 + ny * bh;
        g.camera_pan = [
            b.width * 0.5 - target_cx * g.camera_zoom,
            b.height * 0.5 - target_cy * g.camera_zoom,
        ];
    }

    fn apply_slider_drag(&self, node_id: NodeId, track_left: f64, track_width: f64, canvas_x: f64) {
        let t = ((canvas_x - track_left) / track_width.max(1.0)).clamp(0.0, 1.0);
        let mut changed = false;
        {
            let g_rc = self.graph.borrow();
            let mut g = g_rc.borrow_mut();
            if let Some(node) = g.nodes.get_mut(&node_id) {
                if let ParamWidget::Slider { ref mut value, min, max, step, integer } = node.widget {
                    let mut raw = min + t * (max - min);
                    if step > 0.0 {
                        raw = ((raw - min) / step).round() * step + min;
                    }
                    if integer {
                        raw = raw.round();
                    }
                    raw = raw.clamp(min, max);
                    if (raw - *value).abs() > 1e-9 {
                        *value = raw;
                        changed = true;
                    }
                }
            }
            if changed {
                g.mark_dirty(node_id);
            }
        }
        if changed {
            self.reevaluate_and_notify();
        }
    }
}

impl InputElementImpl for NodeGraphCanvas {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        this.focus();
        let input: Ref<InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&input));

        let pt = e.get_position(Some(this));
        let props = e.properties();
        let mods = e.key_modifiers();
        let is_right = props.is_right_button_pressed;
        let is_middle = props.is_middle_button_pressed;
        let shift = mods.contains(KeyModifiers::SHIFT);
        let alt = mods.contains(KeyModifiers::ALT);

        // Minimap navigation click
        let mm = this.minimap_rect();
        if !is_right && mm.contains(pt) && this.bounds().width > 320.0 {
            *this.mode.borrow_mut() = InteractionMode::MinimapDrag;
            this.jump_camera_from_minimap(pt);
            this.invalidate_visual();
            return;
        }

        let canvas_pt = this.screen_to_canvas(pt);

        // Right-click on a port disconnects all wires on that input port; right-click wire removes wire
        if is_right {
            if let Some((nid, pidx, PortDirection::Input)) = this.hit_test_port(canvas_pt) {
                let removed = {
                    let g_rc = this.graph.borrow();
                    let mut g = g_rc.borrow_mut();
                    g.push_undo_checkpoint();
                    g.disconnect_input_port(nid, pidx)
                };
                if removed > 0 {
                    this.reevaluate_and_notify();
                    return;
                }
            }
            if let Some(wid) = this.hit_test_wire(canvas_pt) {
                {
                    let g_rc = this.graph.borrow();
                    let mut g = g_rc.borrow_mut();
                    g.push_undo_checkpoint();
                    g.disconnect_wire(wid);
                }
                this.reevaluate_and_notify();
                return;
            }
            // Otherwise right-drag pans the canvas smoothly (classic Rhino/Grasshopper!)
            let start_pan = this.graph.borrow().borrow().camera_pan;
            *this.mode.borrow_mut() = InteractionMode::Panning { start_screen: pt, start_pan };
            return;
        }

        if is_middle || alt {
            let start_pan = this.graph.borrow().borrow().camera_pan;
            *this.mode.borrow_mut() = InteractionMode::Panning { start_screen: pt, start_pan };
            return;
        }

        // 1. Port hit -> start drawing a wire!
        if let Some((nid, pidx, dir)) = this.hit_test_port(canvas_pt) {
            *this.mode.borrow_mut() = InteractionMode::Wiring {
                from_node: nid,
                from_port: pidx,
                from_dir: dir,
                cursor_canvas: canvas_pt,
                multi_wire: shift,
            };
            this.invalidate_visual();
            return;
        }

        // 2. Node hit -> handle inline widget interaction (Slider / Toggle / ColorSwatch) or drag node!
        if let Some(nid) = this.hit_test_node(canvas_pt) {
            let (kind, pos, size) = {
                let g_rc = this.graph.borrow();
                let g = g_rc.borrow();
                let reg = this.registry.borrow();
                let n = &g.nodes[&nid];
                let k = reg.get(&n.type_id).map(|c| c.kind()).unwrap_or(NodeKind::Component);
                (k, n.position, n.size)
            };

            // Inline NumberSlider track drag (bottom half of slider capsule!)
            if kind == NodeKind::NumberSlider && canvas_pt[1] >= pos[1] + 19.0 {
                {
                    let g_rc = this.graph.borrow();
                    let mut g = g_rc.borrow_mut();
                    g.push_undo_checkpoint();
                    g.selected_nodes.clear();
                    g.selected_nodes.insert(nid);
                }
                let track_left = pos[0] + 12.0;
                let track_width = (size[0] - 26.0).max(40.0);
                *this.mode.borrow_mut() =
                    InteractionMode::DraggingSlider { node_id: nid, track_left, track_width };
                this.apply_slider_drag(nid, track_left, track_width, canvas_pt[0]);
                return;
            }

            // Inline BoolToggle click!
            if kind == NodeKind::BoolToggle && canvas_pt[0] >= pos[0] + 54.0 {
                {
                    let g_rc = this.graph.borrow();
                    let mut g = g_rc.borrow_mut();
                    g.push_undo_checkpoint();
                    if let Some(n) = g.nodes.get_mut(&nid) {
                        if let ParamWidget::Toggle { ref mut value } = n.widget {
                            *value = !*value;
                        }
                    }
                    g.selected_nodes.clear();
                    g.selected_nodes.insert(nid);
                    g.mark_dirty(nid);
                }
                this.reevaluate_and_notify();
                return;
            }

            // Inline ColorSwatch click cycles vibrant colors!
            if kind == NodeKind::ColorSwatch && canvas_pt[0] >= pos[0] + 46.0 {
                {
                    let g_rc = this.graph.borrow();
                    let mut g = g_rc.borrow_mut();
                    g.push_undo_checkpoint();
                    if let Some(n) = g.nodes.get_mut(&nid) {
                        if let ParamWidget::Color { ref mut argb } = n.widget {
                            let idx = SWATCH_PALETTE.iter().position(|&c| c == *argb).unwrap_or(0);
                            *argb = SWATCH_PALETTE[(idx + 1) % SWATCH_PALETTE.len()];
                        }
                    }
                    g.selected_nodes.clear();
                    g.selected_nodes.insert(nid);
                    g.mark_dirty(nid);
                }
                this.reevaluate_and_notify();
                return;
            }

            // Standard node select & drag
            let originals = {
                let g_rc = this.graph.borrow();
                let mut g = g_rc.borrow_mut();
                g.selected_wires.clear();
                if shift {
                    if g.selected_nodes.contains(&nid) {
                        g.selected_nodes.remove(&nid);
                    } else {
                        g.selected_nodes.insert(nid);
                    }
                } else if !g.selected_nodes.contains(&nid) {
                    g.selected_nodes.clear();
                    g.selected_nodes.insert(nid);
                }
                g.push_undo_checkpoint();
                g.selected_nodes
                    .iter()
                    .filter_map(|id| g.nodes.get(id).map(|n| (*id, n.position)))
                    .collect::<Vec<_>>()
            };
            *this.mode.borrow_mut() =
                InteractionMode::DraggingNodes { start_canvas: canvas_pt, originals };
            if let Some(cb) = this.on_changed.borrow().as_ref() {
                cb(&this.graph.borrow().borrow());
            }
            this.invalidate_visual();
            return;
        }

        // 3. Wire hit -> select wire
        if let Some(wid) = this.hit_test_wire(canvas_pt) {
            {
                let g_rc = this.graph.borrow();
                let mut g = g_rc.borrow_mut();
                g.selected_nodes.clear();
                g.selected_wires.clear();
                g.selected_wires.insert(wid);
            }
            this.invalidate_visual();
            return;
        }

        // 4. Double-click empty canvas -> Grasshopper Quick-Search!
        if e.click_count() >= 2 {
            if let Some(cb) = this.on_double_click_empty.borrow().as_ref() {
                cb(canvas_pt[0], canvas_pt[1]);
            }
            return;
        }

        // 5. Empty space drag -> Marquee box selection
        {
            let g_rc = this.graph.borrow();
            let mut g = g_rc.borrow_mut();
            if !shift {
                g.selected_nodes.clear();
                g.selected_wires.clear();
            }
        }
        *this.mode.borrow_mut() =
            InteractionMode::Marquee { start_canvas: canvas_pt, current_canvas: canvas_pt };
        this.invalidate_visual();
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let pt = e.get_position(Some(this));
        let canvas_pt = this.screen_to_canvas(pt);
        let cur_mode = this.mode.borrow().clone();

        match cur_mode {
            InteractionMode::Idle => {
                let prev = this.hover_port.borrow().is_some();
                let hit = this.hit_test_port(canvas_pt).map(|(node_id, port_idx, dir)| HoverTarget {
                    node_id,
                    port_idx,
                    dir,
                    screen_pt: pt,
                });
                let now_has = hit.is_some();
                *this.hover_port.borrow_mut() = hit;
                if prev || now_has {
                    this.invalidate_visual();
                }
            }
            InteractionMode::MinimapDrag => {
                this.jump_camera_from_minimap(pt);
                this.invalidate_visual();
            }
            InteractionMode::Panning { start_screen, start_pan } => {
                let g_rc = this.graph.borrow();
                let mut g = g_rc.borrow_mut();
                g.camera_pan = [
                    start_pan[0] + (pt.x - start_screen.x),
                    start_pan[1] + (pt.y - start_screen.y),
                ];
                this.invalidate_visual();
            }
            InteractionMode::DraggingSlider { node_id, track_left, track_width } => {
                this.apply_slider_drag(node_id, track_left, track_width, canvas_pt[0]);
            }
            InteractionMode::DraggingNodes { start_canvas, originals } => {
                let dx = canvas_pt[0] - start_canvas[0];
                let dy = canvas_pt[1] - start_canvas[1];
                let g_rc = this.graph.borrow();
                let mut g = g_rc.borrow_mut();
                let snap = g.snap_to_grid;
                for (id, orig) in originals {
                    if let Some(n) = g.nodes.get_mut(&id) {
                        let nx = orig[0] + dx;
                        let ny = orig[1] + dy;
                        n.position = if snap {
                            [(nx / 20.0).round() * 20.0, (ny / 20.0).round() * 20.0]
                        } else {
                            [nx, ny]
                        };
                    }
                }
                this.invalidate_visual();
            }
            InteractionMode::Wiring { from_node, from_port, from_dir, multi_wire, .. } => {
                *this.mode.borrow_mut() = InteractionMode::Wiring {
                    from_node,
                    from_port,
                    from_dir,
                    cursor_canvas: canvas_pt,
                    multi_wire: multi_wire || e.key_modifiers().contains(KeyModifiers::SHIFT),
                };
                this.invalidate_visual();
            }
            InteractionMode::Marquee { start_canvas, .. } => {
                *this.mode.borrow_mut() =
                    InteractionMode::Marquee { start_canvas, current_canvas: canvas_pt };
                let x0 = start_canvas[0].min(canvas_pt[0]);
                let y0 = start_canvas[1].min(canvas_pt[1]);
                let x1 = start_canvas[0].max(canvas_pt[0]);
                let y1 = start_canvas[1].max(canvas_pt[1]);
                {
                    let g_rc = this.graph.borrow();
                    let mut g = g_rc.borrow_mut();
                    g.selected_nodes.clear();
                    let matched: Vec<NodeId> = g
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
                    for id in matched {
                        g.selected_nodes.insert(id);
                    }
                }
                this.invalidate_visual();
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        e.pointer().capture(None);
        let pt = e.get_position(Some(this));
        let canvas_pt = this.screen_to_canvas(pt);
        let prev = std::mem::replace(&mut *this.mode.borrow_mut(), InteractionMode::Idle);

        if let InteractionMode::Wiring { from_node, from_port, from_dir, multi_wire, .. } = prev {
            let multi = multi_wire || e.key_modifiers().contains(KeyModifiers::SHIFT);
            if let Some((hit_node, hit_port, hit_dir)) = this.hit_test_port(canvas_pt) {
                if hit_dir != from_dir {
                    let (src_n, src_p, dst_n, dst_p) = match from_dir {
                        PortDirection::Output => (from_node, from_port, hit_node, hit_port),
                        PortDirection::Input => (hit_node, hit_port, from_node, from_port),
                    };
                    let ok = {
                        let g_rc = this.graph.borrow();
                        let mut g = g_rc.borrow_mut();
                        g.push_undo_checkpoint();
                        g.connect(src_n, src_p, dst_n, dst_p, multi).is_ok()
                    };
                    if ok {
                        this.reevaluate_and_notify();
                        return;
                    }
                }
            }
        }
        this.invalidate_visual();
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        let pt = e.get_position(Some(this));
        let dy = e.delta().y;
        let factor = if dy > 0.0 { 1.12 } else if dy < 0.0 { 1.0 / 1.12 } else { 1.0 };
        {
            let g_rc = this.graph.borrow();
            let mut g = g_rc.borrow_mut();
            let old_z = g.camera_zoom;
            let new_z = (old_z * factor).clamp(0.25, 2.8);
            // Cursor-centered zoom (just like Rhino / Grasshopper!)
            let world_x = (pt.x - g.camera_pan[0]) / old_z;
            let world_y = (pt.y - g.camera_pan[1]) / old_z;
            g.camera_zoom = new_z;
            g.camera_pan = [pt.x - world_x * new_z, pt.y - world_y * new_z];
        }
        e.set_handled(true);
        this.invalidate_visual();
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let cmd = e.key_modifiers.contains(KeyModifiers::CONTROL)
            || e.key_modifiers.contains(KeyModifiers::META);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);

        match e.key {
            Key::Delete | Key::Back => {
                if this.graph.borrow().borrow_mut().delete_selection() {
                    this.reevaluate_and_notify();
                    e.set_handled(true);
                }
            }
            Key::Z if cmd && shift => {
                if this.graph.borrow().borrow_mut().redo() {
                    this.reevaluate_and_notify();
                    e.set_handled(true);
                }
            }
            Key::Z if cmd => {
                if this.graph.borrow().borrow_mut().undo() {
                    this.reevaluate_and_notify();
                    e.set_handled(true);
                }
            }
            Key::Y if cmd => {
                if this.graph.borrow().borrow_mut().redo() {
                    this.reevaluate_and_notify();
                    e.set_handled(true);
                }
            }
            Key::D if cmd => {
                if !this.graph.borrow().borrow_mut().duplicate_selection().is_empty() {
                    this.reevaluate_and_notify();
                    e.set_handled(true);
                }
            }
            Key::G if cmd => {
                if this
                    .graph
                    .borrow()
                    .borrow_mut()
                    .group_selection("Group", 0x2238BDF8)
                    .is_some()
                {
                    this.invalidate_visual();
                    e.set_handled(true);
                }
            }
            Key::E if cmd => {
                this.graph.borrow().borrow_mut().toggle_disable_selection();
                this.reevaluate_and_notify();
                e.set_handled(true);
            }
            Key::F => {
                this.fit_view();
                e.set_handled(true);
            }
            Key::L => {
                this.graph.borrow().borrow_mut().auto_layout();
                this.fit_view();
                e.set_handled(true);
            }
            _ => {}
        }
    }
}

impl VisualImpl for NodeGraphCanvas {
    fn render(this: &Self, dc: &mut DrawingContext) {
        let bounds = this.bounds();
        let w = bounds.width.max(1.0);
        let h = bounds.height.max(1.0);

        let clip = dc.push_clip(Rect::new(0.0, 0.0, w, h));
        let mut dc = dc.scope(clip);

        // 1. Background
        dc.fill_rectangle(&this.palette.bg, Rect::new(0.0, 0.0, w, h), 0.0);

        let g_rc = this.graph.borrow();
        let g = g_rc.borrow();
        let reg = this.registry.borrow();

        // 2. Adaptive Grasshopper Coordinate Grid
        let z = g.camera_zoom.clamp(0.25, 3.0);
        let minor = 24.0 * z;
        let major = minor * 5.0;
        if minor >= 8.0 {
            let start_x = g.camera_pan[0].rem_euclid(minor);
            let mut x = start_x;
            while x < w {
                dc.draw_line(&this.palette.minor_grid, Point::new(x, 0.0), Point::new(x, h));
                x += minor;
            }
            let start_y = g.camera_pan[1].rem_euclid(minor);
            let mut y = start_y;
            while y < h {
                dc.draw_line(&this.palette.minor_grid, Point::new(0.0, y), Point::new(w, y));
                y += minor;
            }
        }
        let mj_x0 = g.camera_pan[0].rem_euclid(major);
        let mut x = mj_x0;
        while x < w {
            dc.draw_line(&this.palette.major_grid, Point::new(x, 0.0), Point::new(x, h));
            x += major;
        }
        let mj_y0 = g.camera_pan[1].rem_euclid(major);
        let mut y = mj_y0;
        while y < h {
            dc.draw_line(&this.palette.major_grid, Point::new(0.0, y), Point::new(w, y));
            y += major;
        }

        // 3. World-space transformed layer (Groups -> Wires -> Nodes -> Drag Wire -> Marquee)
        {
            let xf = Matrix::create_scale(z, z) * Matrix::create_translation(g.camera_pan[0], g.camera_pan[1]);
            let xf_state = dc.push_transform(xf);
            let mut wdc = dc.scope(xf_state);

            // 3a. Colored Grasshopper Node Groups
            for group in g.groups.values() {
                if let Some([x0, y0, x1, y1]) = g.group_bounds(group) {
                    let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(group.color_argb));
                    let border_col = (group.color_argb & 0x00FF_FFFF) | 0x8800_0000;
                    let border_pen: Rc<dyn IPen> = Rc::new(ImmutablePen::from_uint32(border_col, 1.5));
                    let rect = Rect::new(x0, y0, x1 - x0, y1 - y0);
                    wdc.draw_rectangle(Some(&fill), Some(&border_pen), rect, 12.0, 12.0, &BoxShadows::default());
                    draw_text(&mut wdc, &group.title, Point::new(x0 + 10.0, y0 + 6.0), 11.5, &this.palette.text_accent);
                }
            }

            // 3b. Grasshopper Fancy Bezier Wires
            for w_conn in g.wires.values() {
                if w_conn.display == WireDisplay::Hidden {
                    continue;
                }
                let (Some(n0), Some(n1)) = (g.nodes.get(&w_conn.from_node), g.nodes.get(&w_conn.to_node)) else {
                    continue;
                };
                let p0 = Self::port_position_canvas(n0, &reg, PortDirection::Output, w_conn.from_port);
                let p3 = Self::port_position_canvas(n1, &reg, PortDirection::Input, w_conn.to_port);
                let is_sel = g.selected_wires.contains(&w_conn.id)
                    || g.selected_nodes.contains(&w_conn.from_node)
                    || g.selected_nodes.contains(&w_conn.to_node);
                let data_kind = n0
                    .output_tree(w_conn.from_port)
                    .map(|t| t.wire_data_kind())
                    .unwrap_or(WireDataKind::Empty);

                draw_fancy_wire(
                    &mut wdc,
                    &this.palette,
                    p0,
                    p3,
                    data_kind,
                    g.fancy_wires,
                    w_conn.display == WireDisplay::Faint,
                    is_sel,
                );
            }

            // 3c. Live Wire Preview while dragging
            if let InteractionMode::Wiring { from_node, from_port, from_dir, cursor_canvas, .. } =
                &*this.mode.borrow()
            {
                if let Some(n0) = g.nodes.get(from_node) {
                    let p_anchor = Self::port_position_canvas(n0, &reg, *from_dir, *from_port);
                    let (p0, p3) = match from_dir {
                        PortDirection::Output => (p_anchor, *cursor_canvas),
                        PortDirection::Input => (*cursor_canvas, p_anchor),
                    };
                    draw_bezier(&mut wdc, &this.palette.wire_preview, p0, p3);
                }
            }

            // 3d. Grasshopper Nodes & Inline Widgets
            for node in g.nodes.values() {
                this.render_node(&mut wdc, &g, &reg, node);
            }

            // 3e. Marquee Selection Rectangle
            if let InteractionMode::Marquee { start_canvas, current_canvas } = &*this.mode.borrow() {
                let x0 = start_canvas[0].min(current_canvas[0]);
                let y0 = start_canvas[1].min(current_canvas[1]);
                let mw = (current_canvas[0] - start_canvas[0]).abs();
                let mh = (current_canvas[1] - start_canvas[1]).abs();
                wdc.draw_rectangle(
                    Some(&this.palette.marquee_fill),
                    Some(&this.palette.marquee_border),
                    Rect::new(x0, y0, mw, mh),
                    3.0,
                    3.0,
                    &BoxShadows::default(),
                );
            }
        }

        // 4. Screen-space HUD: Interactive Minimap (bottom-right)
        if w > 320.0 && h > 220.0 {
            this.render_minimap(&mut dc, &g, w, h);
        }

        // 5. Screen-space HUD: Port Hover Tooltip
        if let Some(hover) = this.hover_port.borrow().as_ref() {
            this.render_port_tooltip(&mut dc, &g, &reg, hover, w, h);
        }

        // 6. Subtle bottom-left shortcut legend
        draw_text(
            &mut dc,
            &format!(
                "Zoom {:.0}% · Right-drag Pan · Wheel Zoom · Shift-wire Multi-input · F Fit · L Auto-Layout · Double-click Search",
                g.camera_zoom * 100.0
            ),
            Point::new(12.0, h - 20.0),
            10.5,
            &this.palette.text_muted,
        );
    }
}

impl NodeGraphCanvas {
    fn render_node(
        &self,
        dc: &mut DrawingContext,
        graph: &NodeGraph,
        reg: &NodeRegistry,
        node: &NodeInstance,
    ) {
        let Some(comp) = reg.get(&node.type_id) else { return };
        let rect = Rect::new(node.position[0], node.position[1], node.size[0], node.size[1]);
        let is_selected = graph.selected_nodes.contains(&node.id);
        let kind = comp.kind();

        // Scribble: frameless typography annotation
        if kind == NodeKind::Scribble {
            let text = match &node.widget {
                ParamWidget::Text { text } => text.as_str(),
                _ => "Note",
            };
            if is_selected {
                dc.draw_rectangle_outline(&self.palette.node_selected_border, rect, 6.0);
            }
            draw_text(dc, text, Point::new(rect.x + 6.0, rect.y + 6.0), 16.0, &self.palette.text_accent);
            return;
        }

        // Soft drop shadow
        dc.fill_rectangle(
            &self.palette.shadow,
            Rect::new(rect.x + 3.0, rect.y + 4.0, rect.width, rect.height),
            8.0,
        );

        let border_pen = if is_selected {
            &self.palette.node_selected_border
        } else {
            match &node.state {
                NodeState::Error(_) => &self.palette.node_err_border,
                NodeState::Warning(_) => &self.palette.node_warn_border,
                _ => &self.palette.node_normal_border,
            }
        };

        let title = node.custom_title.as_deref().unwrap_or_else(|| comp.nickname());
        let accent: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(comp.category().accent_argb()));

        match kind {
            NodeKind::NumberSlider => {
                dc.draw_rectangle(Some(&self.palette.node_body), Some(border_pen), rect, 8.0, 8.0, &BoxShadows::default());
                dc.fill_rectangle(&accent, Rect::new(rect.x, rect.y + 4.0, 4.0, rect.height - 8.0), 2.0);
                if let ParamWidget::Slider { value, min, max, integer, .. } = node.widget {
                    let val_str = if integer {
                        format!("{:.0}", value.round())
                    } else {
                        format!("{value:.2}")
                    };
                    draw_text(dc, title, Point::new(rect.x + 11.0, rect.y + 4.0), 10.5, &self.palette.text_muted);
                    draw_text(dc, &val_str, Point::new(rect.x + rect.width - 54.0, rect.y + 4.0), 11.0, &self.palette.text_accent);

                    let track = Rect::new(rect.x + 12.0, rect.y + 24.0, rect.width - 26.0, 10.0);
                    dc.fill_rectangle(&self.palette.slider_track, track, 5.0);
                    let t = ((value - min) / (max - min).max(1e-9)).clamp(0.0, 1.0);
                    let fill_w = (track.width * t).max(8.0);
                    dc.fill_rectangle(&self.palette.slider_fill, Rect::new(track.x, track.y, fill_w, track.height), 5.0);
                    let thumb_x = track.x + track.width * t;
                    dc.draw_ellipse_at(
                        Some(&self.palette.slider_thumb),
                        Some(&self.palette.socket_ring),
                        Point::new(thumb_x, track.y + 5.0),
                        6.5,
                        6.5,
                    );
                }
            }
            NodeKind::BoolToggle => {
                dc.draw_rectangle(Some(&self.palette.node_body), Some(border_pen), rect, 8.0, 8.0, &BoxShadows::default());
                dc.fill_rectangle(&accent, Rect::new(rect.x, rect.y + 4.0, 4.0, rect.height - 8.0), 2.0);
                draw_text(dc, title, Point::new(rect.x + 10.0, rect.y + 12.0), 10.5, &self.palette.text_primary);
                let val = matches!(node.widget, ParamWidget::Toggle { value: true });
                let pill_col = if val { 0xFF059669 } else { 0xFF334155 };
                let pill_br: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(pill_col));
                let pill_rect = Rect::new(rect.x + rect.width - 56.0, rect.y + 9.0, 44.0, 22.0);
                dc.fill_rectangle(&pill_br, pill_rect, 11.0);
                draw_text(
                    dc,
                    if val { "True" } else { "False" },
                    Point::new(pill_rect.x + 8.0, pill_rect.y + 4.0),
                    10.5,
                    &self.palette.text_primary,
                );
            }
            NodeKind::ColorSwatch => {
                dc.draw_rectangle(Some(&self.palette.node_body), Some(border_pen), rect, 8.0, 8.0, &BoxShadows::default());
                draw_text(dc, title, Point::new(rect.x + 10.0, rect.y + 13.0), 10.5, &self.palette.text_primary);
                let argb = match node.widget {
                    ParamWidget::Color { argb } => argb,
                    _ => 0xFF38BDF8,
                };
                let sw: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(argb));
                dc.fill_rectangle(&sw, Rect::new(rect.x + rect.width - 54.0, rect.y + 9.0, 40.0, 26.0), 6.0);
            }
            NodeKind::Panel => {
                dc.draw_rectangle(
                    Some(&self.palette.panel_bg),
                    Some(if is_selected { &self.palette.node_selected_border } else { &self.palette.panel_border }),
                    rect,
                    7.0,
                    7.0,
                    &BoxShadows::default(),
                );
                draw_text(dc, title, Point::new(rect.x + 10.0, rect.y + 5.0), 10.0, &self.palette.text_accent);
                let body = node
                    .output_tree(0)
                    .map(|t| t.format_panel(6))
                    .unwrap_or_else(|| "<empty>".into());
                for (i, line) in body.lines().take(6).enumerate() {
                    let clipped: String = line.chars().take(28).collect();
                    draw_text(
                        dc,
                        &clipped,
                        Point::new(rect.x + 10.0, rect.y + 22.0 + i as f64 * 15.0),
                        10.0,
                        &self.palette.text_primary,
                    );
                }
            }
            NodeKind::Relay => {
                dc.draw_rectangle(Some(&self.palette.node_center), Some(border_pen), rect, 14.0, 14.0, &BoxShadows::default());
                draw_text(dc, "•", Point::new(rect.x + rect.width * 0.5 - 3.0, rect.y + 6.0), 12.0, &self.palette.text_accent);
            }
            NodeKind::ExpressionBox => {
                dc.draw_rectangle(Some(&self.palette.node_body), Some(border_pen), rect, 8.0, 8.0, &BoxShadows::default());
                dc.fill_rectangle(&accent, Rect::new(rect.x + 28.0, rect.y + 1.0, rect.width - 56.0, 4.0), 2.0);
                draw_text(dc, title, Point::new(rect.x + 36.0, rect.y + 9.0), 10.5, &self.palette.text_accent);
                let formula = match &node.widget {
                    ParamWidget::Text { text } => text.as_str(),
                    _ => "x",
                };
                let formula_box = Rect::new(rect.x + 32.0, rect.y + 28.0, rect.width - 64.0, 26.0);
                dc.fill_rectangle(&self.palette.node_center, formula_box, 5.0);
                let short_f: String = formula.chars().take(24).collect();
                draw_text(dc, &short_f, Point::new(formula_box.x + 6.0, formula_box.y + 6.0), 10.0, &self.palette.text_primary);
                self.render_port_labels(dc, reg, node);
            }
            NodeKind::Component => {
                // Authentic Rhino Grasshopper 3-column capsule: [Inputs | Center Dark Badge | Outputs]
                dc.draw_rectangle(Some(&self.palette.node_body), Some(border_pen), rect, 8.0, 8.0, &BoxShadows::default());

                let center_w = (rect.width * 0.44).clamp(44.0, 84.0);
                let center_x = rect.x + (rect.width - center_w) * 0.5;
                let center_rect = Rect::new(center_x, rect.y + 4.0, center_w, rect.height - 8.0);
                dc.fill_rectangle(&self.palette.node_center, center_rect, 5.0);
                // Category color bar at top of center badge
                dc.fill_rectangle(&accent, Rect::new(center_x + 4.0, rect.y + 5.0, center_w - 8.0, 3.0), 1.5);

                // Nickname centered in dark middle badge
                let nick_x = center_x + ((center_w - title.len() as f64 * 6.2) * 0.5).max(4.0);
                draw_text(
                    dc,
                    title,
                    Point::new(nick_x, rect.y + rect.height * 0.5 - 7.0),
                    11.0,
                    &self.palette.text_primary,
                );
                self.render_port_labels(dc, reg, node);
            }
            NodeKind::Scribble => {}
        }

        // Disabled overlay wash
        if node.disabled {
            dc.fill_rectangle(&self.palette.node_disabled_fill, rect, 8.0);
        }

        // Input & Output Port Sockets
        let inputs = comp.inputs();
        for (i, spec) in inputs.iter().enumerate() {
            let p = Self::port_position_canvas(node, reg, PortDirection::Input, i);
            let sock_br: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(spec.port_type.color_argb()));
            dc.draw_ellipse_at(Some(&sock_br), Some(&self.palette.socket_ring), Point::new(p[0], p[1]), 5.2, 5.2);
        }
        let outputs = comp.outputs();
        for (i, spec) in outputs.iter().enumerate() {
            let p = Self::port_position_canvas(node, reg, PortDirection::Output, i);
            let sock_br: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(spec.port_type.color_argb()));
            dc.draw_ellipse_at(Some(&sock_br), Some(&self.palette.socket_ring), Point::new(p[0], p[1]), 5.2, 5.2);
        }

        // Grasshopper Profiler Pill beneath node!
        if graph.show_profiler && !matches!(kind, NodeKind::Relay | NodeKind::Scribble) {
            let label = if node.eval_us >= 1000 {
                format!("{:.2} ms", node.eval_us as f64 / 1000.0)
            } else {
                format!("{} µs", node.eval_us)
            };
            let pill = Rect::new(rect.x + rect.width * 0.5 - 24.0, rect.y + rect.height + 3.0, 48.0, 14.0);
            dc.fill_rectangle(&self.palette.profiler_bg, pill, 4.0);
            draw_text(dc, &label, Point::new(pill.x + 6.0, pill.y + 1.0), 9.0, &self.palette.text_muted);
        }
    }

    fn render_port_labels(&self, dc: &mut DrawingContext, reg: &NodeRegistry, node: &NodeInstance) {
        let Some(comp) = reg.get(&node.type_id) else { return };
        for (i, spec) in comp.inputs().iter().enumerate() {
            let p = Self::port_position_canvas(node, reg, PortDirection::Input, i);
            let mod_badge = node.input_modifiers.get(i).map(|m| m.badge()).unwrap_or_default();
            let text = if mod_badge.is_empty() {
                spec.nickname.clone()
            } else {
                format!("{}·{mod_badge}", spec.nickname)
            };
            draw_text(dc, &text, Point::new(p[0] + 8.0, p[1] - 6.5), 9.5, &self.palette.text_muted);
        }
        for (i, spec) in comp.outputs().iter().enumerate() {
            let p = Self::port_position_canvas(node, reg, PortDirection::Output, i);
            let approx_w = spec.nickname.len() as f64 * 5.8;
            draw_text(
                dc,
                &spec.nickname,
                Point::new(p[0] - 9.0 - approx_w, p[1] - 6.5),
                9.5,
                &self.palette.text_muted,
            );
        }
    }

    fn render_minimap(&self, dc: &mut DrawingContext, g: &NodeGraph, vw: f64, vh: f64) {
        let mm = self.minimap_rect();
        dc.draw_rectangle(
            Some(&self.palette.minimap_bg),
            Some(&self.palette.minimap_border),
            mm,
            8.0,
            8.0,
            &BoxShadows::default(),
        );
        let Some([min_x, min_y, max_x, max_y]) = g.content_bounds() else { return };
        let pad = 120.0;
        let bx0 = min_x - pad;
        let by0 = min_y - pad;
        let bw = (max_x - min_x + pad * 2.0).max(200.0);
        let bh = (max_y - min_y + pad * 2.0).max(160.0);

        let sx = mm.width / bw;
        let sy = mm.height / bh;

        for n in g.nodes.values() {
            let nx = mm.x + (n.position[0] - bx0) * sx;
            let ny = mm.y + (n.position[1] - by0) * sy;
            let nw = (n.size[0] * sx).max(4.0);
            let nh = (n.size[1] * sy).max(3.0);
            let br = if g.selected_nodes.contains(&n.id) {
                &self.palette.slider_thumb
            } else {
                &self.palette.text_muted
            };
            dc.fill_rectangle(br, Rect::new(nx, ny, nw, nh), 1.5);
        }

        // Current viewport indicator rectangle
        let vx0 = (-g.camera_pan[0]) / g.camera_zoom;
        let vy0 = (-g.camera_pan[1]) / g.camera_zoom;
        let vcw = vw / g.camera_zoom;
        let vch = vh / g.camera_zoom;
        let rx = (mm.x + (vx0 - bx0) * sx).clamp(mm.x, mm.x + mm.width - 8.0);
        let ry = (mm.y + (vy0 - by0) * sy).clamp(mm.y, mm.y + mm.height - 8.0);
        let rw = (vcw * sx).clamp(8.0, mm.x + mm.width - rx);
        let rh = (vch * sy).clamp(8.0, mm.y + mm.height - ry);
        dc.draw_rectangle_outline(&self.palette.minimap_view, Rect::new(rx, ry, rw, rh), 2.0);
    }

    fn render_port_tooltip(
        &self,
        dc: &mut DrawingContext,
        g: &NodeGraph,
        reg: &NodeRegistry,
        hover: &HoverTarget,
        vw: f64,
        vh: f64,
    ) {
        let Some(node) = g.nodes.get(&hover.node_id) else { return };
        let Some(comp) = reg.get(&node.type_id) else { return };
        let spec = match hover.dir {
            PortDirection::Input => comp.inputs().get(hover.port_idx).cloned(),
            PortDirection::Output => comp.outputs().get(hover.port_idx).cloned(),
        };
        let Some(spec) = spec else { return };

        let preview = match hover.dir {
            PortDirection::Output => node
                .output_tree(hover.port_idx)
                .map(|t| format!("{} branches · {} items · {}", t.branch_count(), t.item_count(), t.format_panel(1)))
                .unwrap_or_else(|| "<empty>".into()),
            PortDirection::Input => {
                let wires = g.incoming_wires(hover.node_id, hover.port_idx).len();
                let m = node.input_modifiers.get(hover.port_idx).map(|x| x.badge()).unwrap_or_default();
                if m.is_empty() {
                    format!("{wires} wire(s) · default: {}", spec.default_value.short_summary())
                } else {
                    format!("{wires} wire(s) · mod [{m}]")
                }
            }
        };

        let tx = (hover.screen_pt.x + 14.0).min(vw - 250.0).max(8.0);
        let ty = (hover.screen_pt.y - 46.0).clamp(8.0, vh - 56.0);
        let box_rect = Rect::new(tx, ty, 242.0, 44.0);
        dc.draw_rectangle(
            Some(&self.palette.tooltip_bg),
            Some(&self.palette.tooltip_border),
            box_rect,
            6.0,
            6.0,
            &BoxShadows::default(),
        );
        draw_text(
            dc,
            &format!("{} ({}) · {}", spec.name, spec.nickname, spec.port_type.label()),
            Point::new(tx + 8.0, ty + 5.0),
            10.5,
            &self.palette.text_accent,
        );
        let short_p: String = preview.chars().take(36).collect();
        draw_text(dc, &short_p, Point::new(tx + 8.0, ty + 22.0), 10.0, &self.palette.text_primary);
    }
}

fn draw_bezier(dc: &mut DrawingContext, pen: &Rc<dyn IPen>, p0: [f64; 2], p3: [f64; 2]) {
    let dx = ((p3[0] - p0[0]).abs() * 0.48).clamp(36.0, 180.0);
    let p1 = Point::new(p0[0] + dx, p0[1]);
    let p2 = Point::new(p3[0] - dx, p3[1]);
    let geom = StreamGeometry::new();
    {
        let mut ctx = geom.open();
        ctx.begin_figure(Point::new(p0[0], p0[1]), false);
        ctx.cubic_bezier_to(p1, p2, Point::new(p3[0], p3[1]), true);
        ctx.end_figure(false);
        ctx.dispose();
    }
    dc.draw_geometry(None, Some(pen), &geom.upcast());
}

#[allow(clippy::too_many_arguments)]
fn draw_fancy_wire(
    dc: &mut DrawingContext,
    pal: &CanvasPalette,
    p0: [f64; 2],
    p3: [f64; 2],
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
