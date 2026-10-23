//! Interactive retained-mode surface. Artwork uses only FerroUI's public DrawingContext API.
mod network;
mod network_bucket;
mod grids;
mod image_crop;
mod layout_drag;
mod selection;
mod input_observations;
mod vector_width;
mod corner_radius;
mod gradient_edit;
use crate::{
    icons::brush,
    renderer::{RenderStats, ScenePainter},
};
use ferrodesign_core::{
    self as core, Color, Editor, Node, NodeId, NodeKind, PathPointId, PathPointKind, PathSegment,
    Point, Rect, VectorPath, Viewport,
};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IPen, StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point as UiPoint,
    Rect as UiRect, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl, TopLevel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Select,
    Frame,
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Star,
    Pen,
    Pencil,
    Text,
    Hand,
    Comment,
    Section,
}
impl Tool {
    pub fn name(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Frame => "Frame",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Line => "Line",
            Self::Polygon => "Polygon",
            Self::Star => "Star",
            Self::Pen => "Pen",
            Self::Pencil => "Pencil",
            Self::Text => "Text",
            Self::Hand => "Hand",
            Self::Comment => "Comment",
            Self::Section => "Section",
        }
    }
    fn kind(self) -> NodeKind {
        match self {
            Self::Frame => NodeKind::Frame,
            Self::Ellipse => NodeKind::Ellipse,
            Self::Line => NodeKind::Line,
            Self::Polygon => NodeKind::Polygon,
            Self::Star => NodeKind::Star,
            Self::Pen | Self::Pencil => NodeKind::Vector,
            Self::Text => NodeKind::Text,
            Self::Section => NodeKind::Section,
            _ => NodeKind::Rectangle,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VectorTool { #[default] Point, Bend, Erase, Connect, Bucket }

#[derive(Clone)]
pub enum CanvasEvent {
    Selection,
    Document,
    Draft,
    Camera,
    Tool,
    EditText(NodeId),
    Comment(Point),
    ContextMenu,
    Notice(String),
    PrototypeTrigger(NodeId, core::Trigger),
    PrototypeKey(String),
    PrototypeScroll(NodeId, Point),
    PrototypePointerMoved(Point),
    PrototypePointerExited,
    ExitPresentation,
    Action(&'static str),
}
enum Gesture {
    None,
    Pan {
        start: Point,
        original: Point,
    },
    Move {
        start: Point,
        nodes: Vec<Node>,
        bounds: Option<Rect>,
        exclude: Vec<NodeId>,
        moved: bool,
    },
    Resize {
        node: Node,
        corner: usize,
    },
    Rotate {
        node: Node,
        center: Point,
        angle: f64,
    },
    Marquee {
        start: Point,
        current: Point,
        additive: bool,
    },
    Draw {
        start: Point,
        id: NodeId,
    },
    Pencil {
        id: NodeId,
        origin: Point,
    },
    PenHandle {
        id: NodeId,
        segment: usize,
        anchor: Point,
        start: Point,
    },
    NetworkPoint { node: NodeId, point: core::NetworkPointId, start: Point, original: Point },
    NetworkBend { node: NodeId, edge: core::VectorEdgeId, t: f64 },
    VectorBend { node: NodeId, segment: usize, t: f64 },
    VectorPoint {
        node: NodeId,
        point: PathPointId,
        start: Point,
        original: Point,
    },
}
#[repr(C)]
pub struct DesignCanvas {
    base: Control,
    pub editor: Rc<RefCell<Editor>>,
    pub viewport: Cell<Viewport>,
    pub tool: Cell<Tool>,
    gesture: RefCell<Gesture>,
    painter: RefCell<ScenePainter>,
    callback: RefCell<Option<Rc<dyn Fn(CanvasEvent)>>>,
    hover: Cell<Option<NodeId>>,
    prototype_press: Cell<Option<(NodeId, Point, bool)>>,
    space: Cell<bool>,
    initialized: Cell<bool>,
    pub show_grid: Cell<bool>,
    pub show_rulers: Cell<bool>,
    pub dark: Cell<bool>,
    pub presentation: Cell<Option<NodeId>>,
    pub overlays: RefCell<Vec<(NodeId, Point)>>,
    pub prototype_scene: RefCell<Option<core::PrototypeScene>>,
    pub frames: Cell<u64>,
    pub snap: Cell<bool>,
    snap_guides: Cell<(Option<f64>, Option<f64>)>,
    pen_node: Cell<Option<NodeId>>,
    pen_tangent: Cell<Option<Point>>,
    pub vector_node: Cell<Option<NodeId>>,
    pub vector_point: Cell<Option<PathPointId>>,
    pub vector_tool: Cell<VectorTool>,
    pub network_point: Cell<Option<core::NetworkPointId>>,
    pub network_vertices: RefCell<Vec<core::VectorVertexId>>,
    pub network_region: Cell<Option<core::VectorRegionId>>,
    network_bucket: RefCell<network_bucket::BucketState>,
    layout_grids: RefCell<grids::LayoutGridOverlay>,
    image_crop: Cell<Option<image_crop::ImageCropMode>>,
    image_crop_gesture: RefCell<Option<image_crop::ImageCropGesture>>,
    layout_drag: RefCell<Option<layout_drag::LayoutDragGesture>>,
    free_layout_probe: RefCell<Option<layout_drag::FreeLayoutProbe>>,
    pub select_groups: Cell<bool>,
    selection_context: RefCell<Option<selection::CachedSelectionContext>>,
    prototype_selection: RefCell<Option<selection::CachedPrototypeSelection>>,
    selection_pointer: Cell<Option<Point>>,
    pointer_presses: RefCell<input_observations::PointerPresses>,
    stroke_width: RefCell<vector_width::WidthState>,
    corner_radius: RefCell<corner_radius::CornerRadiusState>,
    linear_gradient: RefCell<gradient_edit::GradientState>,
}
ferro_class!(DesignCanvas:Control);
ferro_impl_classes!(DesignCanvas:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
impl DesignCanvas {
    pub fn new(editor: Rc<RefCell<Editor>>) -> Ref<Self> {
        let this = instantiate(Self {
            base: Control::construct(),
            editor,
            viewport: Cell::new(Viewport::default()),
            tool: Cell::new(Tool::Select),
            gesture: RefCell::new(Gesture::None),
            painter: RefCell::new(ScenePainter::default()),
            callback: RefCell::new(None),
            hover: Cell::new(None),
            prototype_press: Cell::new(None),
            space: Cell::new(false),
            initialized: Cell::new(false),
            show_grid: Cell::new(false),
            show_rulers: Cell::new(false),
            dark: Cell::new(false),
            presentation: Cell::new(None),
            overlays: RefCell::new(Vec::new()),
            prototype_scene: RefCell::new(None),
            frames: Cell::new(0),
            snap: Cell::new(true),
            snap_guides: Cell::new((None, None)),
            pen_node: Cell::new(None),
            pen_tangent: Cell::new(None),
            vector_node: Cell::new(None),
            vector_point: Cell::new(None),
            vector_tool: Cell::new(VectorTool::Point),
            network_point: Cell::new(None),
            network_vertices: RefCell::new(Vec::new()),
            network_region: Cell::new(None),
            network_bucket: RefCell::new(network_bucket::BucketState::default()),
            layout_grids: RefCell::new(grids::LayoutGridOverlay::default()),
            image_crop: Cell::new(None),
            image_crop_gesture: RefCell::new(None),
            layout_drag: RefCell::new(None),
            free_layout_probe: RefCell::new(None),
            select_groups: Cell::new(false),
            selection_context: RefCell::new(None),
            prototype_selection: RefCell::new(None),
            selection_pointer: Cell::new(None),
            pointer_presses: RefCell::new(input_observations::PointerPresses::default()),
            stroke_width: RefCell::new(vector_width::WidthState::default()),
            corner_radius: RefCell::new(corner_radius::CornerRadiusState::default()),
            linear_gradient: RefCell::new(gradient_edit::GradientState::default()),
        });
        this.set_focusable(true);
        this.set_clip_to_bounds(true);
        this
    }
    pub fn on_change(&self, callback: impl Fn(CanvasEvent) + 'static) {
        *self.callback.borrow_mut() = Some(Rc::new(callback));
    }
    /// Bounded observations of real routed input; reading this never changes
    /// selection, focus, capture, history, layout or the render schedule.
    pub fn pointer_press_observations(&self) -> serde_json::Value {
        self.pointer_presses.borrow().json()
    }
    fn emit(&self, event: CanvasEvent) {
        let callback = self.callback.borrow().clone();
        if let Some(f) = callback {
            f(event)
        }
    }
    fn prototype_pick(&self, world: Point) -> Option<NodeId> {
        if self.prototype_scene.borrow().is_some() { return None; }
        let editor = self.editor.borrow();
        let current = self.presentation.get()?;
        let mut point = world;
        let mut root = current;
        if let Some(&(overlay, position)) = self.overlays.borrow().last() {
            // The top overlay owns input, including its empty regions.
            let current_bounds = editor.document.world_bounds(current)?;
            let bounds = editor.document.world_bounds(overlay)?;
            point = Point::new(
                world.x - current_bounds.x - position.x + bounds.x,
                world.y - current_bounds.y - position.y + bounds.y,
            );
            root = overlay;
        }
        self.prototype_pick_in_root(&editor, root, point)
    }
    /// Activation changes cannot reuse a press or hover from an earlier frame.
    pub fn reset_presentation_input(&self) {
        self.leave_linear_gradient_edit();
        self.cancel_corner_radius_gesture();
        self.clear_corner_radius_hover();
        self.leave_stroke_width_edit();
        self.cancel_layout_drag();
        self.prototype_press.set(None);
        self.hover.set(None);
        self.space.set(false);
    }
    pub fn set_tool(&self, tool: Tool) {
        self.leave_linear_gradient_edit();
        self.cancel_corner_radius_gesture();
        self.clear_corner_radius_hover();
        self.leave_stroke_width_edit();
        self.cancel_network_bucket_gesture();
        self.clear_bucket_hover();
        self.cancel_layout_drag();
        self.finish_image_crop();
        self.finish_pen();
        if tool != Tool::Select {
            self.leave_vector_edit();
        }
        self.tool.set(tool);
        self.focus();
        self.invalidate_visual();
        self.emit(CanvasEvent::Tool);
    }
    pub fn reset_fit(&self) {
        self.cancel_linear_gradient_gesture();
        self.clear_gradient_hover();
        self.cancel_corner_radius_gesture();
        self.clear_corner_radius_hover();
        self.cancel_layout_drag();
        self.initialized.set(false);
        self.invalidate_visual();
    }
    pub fn zoom(&self, factor: f64) {
        self.cancel_linear_gradient_gesture();
        self.clear_gradient_hover();
        self.cancel_corner_radius_gesture();
        self.clear_corner_radius_hover();
        self.cancel_layout_drag();
        let mut v = self.viewport.get();
        v.zoom_at(Point::new(v.width / 2.0, v.height / 2.0), v.zoom * factor);
        self.viewport.set(v);
        self.invalidate_visual();
        self.emit(CanvasEvent::Camera);
    }
    pub fn zoom_to_selection(&self) {
        self.cancel_linear_gradient_gesture();
        self.clear_gradient_hover();
        self.cancel_corner_radius_gesture();
        self.clear_corner_radius_hover();
        self.cancel_layout_drag();
        let editor = self.editor.borrow();
        let bounds = editor
            .selection
            .iter()
            .filter_map(|&id| editor.document.world_bounds(id))
            .reduce(|a, b| a.union(b));
        if let Some(bounds) = bounds {
            let mut v = self.viewport.get();
            v.fit(bounds, 90.0);
            self.viewport.set(v);
            self.invalidate_visual();
        }
        drop(editor);
        self.emit(CanvasEvent::Camera);
    }
    pub fn render_stats(&self) -> RenderStats {
        self.painter.borrow().stats()
    }
    pub fn register_image_data(&self, href: &str) -> std::io::Result<(f64, f64)> {
        self.painter.borrow_mut().register_image_data(href)
    }
    pub fn export_png(&self) -> Result<Vec<u8>, String> {
        use ferroui_base::media::imaging::{
            BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap,
        };
        use ferroui_base::PixelSize;
        let editor = self.editor.borrow();
        let roots = if editor.selection.is_empty() {
            editor
                .document
                .page(editor.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        } else {
            editor.root_selection()
        };
        let bounds = self
            .painter
            .borrow_mut()
            .rendered_editor_bounds(&editor, &roots)
            .ok_or_else(|| "There are no layers to export.".to_string())?;
        let width = bounds.width.ceil().max(1.0);
        let height = bounds.height.ceil().max(1.0);
        if width > 16384.0 || height > 16384.0 || width * height > 32_000_000.0 {
            return Err("Select a smaller region; PNG export supports up to 32 megapixels.".into());
        }
        let bitmap = RenderTargetBitmap::new(PixelSize::new(width as i32, height as i32));
        {
            let mut context = bitmap.create_drawing_context();
            let transform =
                context.push_transform(Matrix::create_translation(-bounds.x, -bounds.y));
            let mut painter = self.painter.borrow_mut();
            // This export has one output pixel per document unit. Canvas zoom
            // and the window's display density must not affect the saved PNG.
            painter.set_raster_scale(1.0);
            painter.render_editor(&mut context, &editor, &roots, bounds);
            context.pop(transform);
            context.dispose();
        }
        let mut bytes = Vec::new();
        let result = bitmap.save(
            &mut bytes,
            &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default()),
        );
        bitmap.dispose();
        result.map_err(|e| e.to_string())?;
        Ok(bytes)
    }
    pub fn invalidate_scene(&self) {
        self.painter.borrow_mut().invalidate();
        self.invalidate_visual();
    }
    pub fn set_editing_text(&self, node: Option<NodeId>) {
        self.painter.borrow_mut().set_editing_text(node);
        self.invalidate_visual();
    }
    pub fn finish_pen(&self) {
        if matches!(*self.gesture.borrow(), Gesture::PenHandle { .. }) {
            *self.gesture.borrow_mut() = Gesture::None;
        }
        self.pen_tangent.set(None);
        if let Some(id) = self.pen_node.take() {
            let mut editor = self.editor.borrow_mut();
            normalize_drawn_vector(&mut editor, id);
            editor.commit_transaction();
            drop(editor);
            self.invalidate_visual();
            self.emit(CanvasEvent::Document);
        }
    }
    /// Undo during a live pointer edit cancels that transaction and its gesture.
    /// Later pointer events must never continue writing after history restored it.
    pub fn cancel_active_edit(&self) -> bool {
        if self.cancel_linear_gradient_gesture() { return true; }
        if self.cancel_corner_radius_gesture() { return true; }
        if self.cancel_stroke_width_gesture() { return true; }
        if self.cancel_network_bucket_gesture() { return true; }
        if self.cancel_layout_drag() { return true; }
        if self.cancel_image_crop_gesture() { return true; }
        let active =
            self.pen_node.get().is_some() || !matches!(*self.gesture.borrow(), Gesture::None);
        if active {
            self.cancel();
        }
        active
    }
    pub fn cancel(&self) {
        self.layout_grids.borrow_mut().clear_snap();
        if self.cancel_linear_gradient_gesture() { return; }
        self.clear_gradient_hover();
        if self.cancel_corner_radius_gesture() { return; }
        self.clear_corner_radius_hover();
        if self.cancel_stroke_width_gesture() { return; }
        if self.cancel_network_bucket_gesture() { return; }
        // This preview owns no transaction. In particular, Escape must not
        // cancel an unrelated edit opened after its immutable snapshot.
        if self.cancel_layout_drag() { return; }
        self.cancel_image_crop();
        self.editor.borrow_mut().cancel_transaction();
        self.pen_node.set(None);
        self.pen_tangent.set(None);
        *self.gesture.borrow_mut() = Gesture::None;
        self.snap_guides.set((None, None));
        self.space.set(false);
        self.invalidate_visual();
        self.emit(CanvasEvent::Document);
    }
    /// Enter direct path editing without replacing the retained document node.
    pub fn edit_vector(&self) -> bool {
        self.leave_linear_gradient_edit();
        self.cancel_corner_radius_gesture();
        self.clear_corner_radius_hover();
        self.leave_stroke_width_edit();
        self.cancel_network_bucket_gesture();
        self.cancel_layout_drag();
        self.finish_image_crop();
        self.finish_pen();
        let editor = self.editor.borrow();
        let id = editor.selection.first().copied().filter(|id| {
            editor.selection.len() == 1
                && editor.document.nodes.get(id).is_some_and(|n| {
                    n.kind == NodeKind::Vector && !n.locked && (!n.path.segments.is_empty() || n.vector_network.is_some())
                })
        });
        drop(editor);
        if let Some(id) = id {
            self.tool.set(Tool::Select);
            self.vector_node.set(Some(id));
            self.vector_point.set(None);
            self.focus();
            self.invalidate_visual();
            self.emit(CanvasEvent::Selection);
            true
        } else {
            false
        }
    }
    pub fn leave_vector_edit(&self) {
        self.cancel_network_bucket_gesture();
        self.clear_bucket_hover();
        self.vector_node.set(None);
        self.vector_point.set(None);
        self.network_region.set(None);
        self.network_point.set(None);
        self.network_vertices.borrow_mut().clear();
        self.invalidate_visual();
    }
    pub fn reconcile_vector_edit(&self) {
        self.reconcile_linear_gradient();
        self.reconcile_corner_radius();
        self.reconcile_stroke_width();
        self.reconcile_network_bucket();
        let editor = self.editor.borrow();
        let valid = self.vector_node.get().is_some_and(|id| {
            editor.selection.as_slice() == [id]
                && editor
                    .document
                    .nodes
                    .get(&id)
                    .is_some_and(|n| n.kind == NodeKind::Vector && !n.locked && n.visible)
        });
        if !valid {
            drop(editor);
            self.leave_vector_edit();
        } else if let Some(network) = self.vector_node.get().and_then(|id| editor.document.nodes.get(&id)).and_then(|n| n.vector_network.as_ref()) {
            if self.network_point.get().is_some_and(|point| network.point_position(point).is_none()) { self.network_point.set(None); }
            self.network_vertices.borrow_mut().retain(|vertex| network.vertices.contains_key(vertex));
        } else if let (Some(id), Some(point)) = (self.vector_node.get(), self.vector_point.get()) {
            if editor.document.nodes[&id]
                .path
                .point_position(point)
                .is_none()
            {
                self.vector_point.set(None);
            }
        }
    }
    pub fn convert_vector_point(&self, mode: core::AnchorMode) -> bool {
        let Some(id) = self.vector_node.get() else {
            return false;
        };
        if let Some(point) = self.network_point.get() {
            let vertex = self.editor.borrow().document.nodes.get(&id).and_then(|node| node.vector_network.as_ref()).and_then(|network| network.point_vertex(point));
            if let Some(vertex) = vertex {
                let changed = self.editor.borrow_mut().set_network_vertex_mode(id, vertex, mode);
                if changed { self.emit(CanvasEvent::Document); }
                return changed;
            }
        }
        let Some(point) = self.vector_point.get() else {
            return false;
        };
        let anchor = self
            .editor
            .borrow()
            .document
            .nodes
            .get(&id)
            .and_then(|n| n.path.edit_points().into_iter().find(|p| p.id == point))
            .map(|p| p.anchor_id);
        let Some(anchor) = anchor else {
            return false;
        };
        let changed = self
            .editor
            .borrow_mut()
            .convert_vector_anchor(id, anchor.segment, mode);
        self.vector_point.set(Some(anchor));
        if changed {
            self.invalidate_visual();
            self.emit(CanvasEvent::Document);
        }
        changed
    }
    fn vector_press(&self, world: Point, clicks: i32, additive: bool) -> bool {
        let Some(id) = self.vector_node.get() else {
            return false;
        };
        let editor = self.editor.borrow();
        let Some(node) = editor.document.nodes.get(&id) else {
            return false;
        };
        if node.vector_network.is_some() {
            drop(editor);
            return self.network_press(world, clicks, additive);
        }
        let Some(transform) = editor.document.world_transform(id) else {
            return false;
        };
        let viewport = self.viewport.get();
        let pointer = viewport.world_to_screen(world);
        if self.vector_tool.get() != VectorTool::Point {
            let hit = node.path.transformed(transform).nearest_segment(world)
                .filter(|hit| viewport.world_to_screen(hit.position).distance(pointer) <= 8.0);
            drop(editor);
            if let Some(hit) = hit {
                self.vector_point.set(None);
                if self.vector_tool.get() == VectorTool::Erase {
                    if self.editor.borrow_mut().erase_vector_segment(id, hit.segment) {
                        self.emit(CanvasEvent::Document);
                    }
                } else {
                    self.editor.borrow_mut().begin_node_transaction(&[id], "Bend vector segment");
                    *self.gesture.borrow_mut() = Gesture::VectorBend { node: id, segment: hit.segment, t: hit.t };
                }
                self.invalidate_visual();
            }
            return true;
        }
        let selected = self.vector_point.get();
        let points = node.path.edit_points();
        let active_anchor = selected
            .and_then(|id| points.iter().find(|p| p.id == id))
            .map(|p| p.anchor_id);
        // Hide unrelated handles and prioritize anchors where retracted handles overlap.
        let hit = points
            .iter()
            .filter(|p| p.id.kind == PathPointKind::Anchor || Some(p.anchor_id) == active_anchor)
            .filter_map(|p| {
                let distance = viewport
                    .world_to_screen(transform.apply(p.position))
                    .distance(pointer);
                (distance <= 8.0).then_some((p, distance))
            })
            .min_by(|(a, da), (b, db)| {
                da.total_cmp(db).then_with(|| {
                    (a.id.kind != PathPointKind::Anchor).cmp(&(b.id.kind != PathPointKind::Anchor))
                })
            })
            .map(|(p, _)| (p.id, transform.apply(p.position)));
        if let Some((point, original)) = hit {
            drop(editor);
            self.vector_point.set(Some(point));
            self.editor
                .borrow_mut()
                .begin_node_transaction(&[id], "Move vector point");
            *self.gesture.borrow_mut() = Gesture::VectorPoint {
                node: id,
                point,
                start: world,
                original,
            };
            self.invalidate_visual();
            self.emit(CanvasEvent::Selection);
            return true;
        }
        // Measure in world space so nonuniform scale and shear do not distort hit tolerance.
        let nearest = node
            .path
            .transformed(transform)
            .nearest_segment(world)
            .filter(|hit| viewport.world_to_screen(hit.position).distance(pointer) <= 7.0);
        drop(editor);
        if let Some(hit) = nearest {
            if clicks >= 2 {
                let added = self
                    .editor
                    .borrow_mut()
                    .split_vector_segment(id, hit.segment, hit.t);
                self.vector_point.set(added);
                self.emit(CanvasEvent::Document);
            } else {
                self.vector_point.set(None);
                self.emit(CanvasEvent::Selection);
            }
            self.invalidate_visual();
            return true;
        }
        self.leave_vector_edit();
        false
    }
    fn world(&self, p: UiPoint) -> Point {
        self.viewport.get().screen_to_world(Point::new(p.x, p.y))
    }
    fn snap_point(&self, p: Point) -> Point {
        if self.snap.get() {
            Point::new(p.x.round(), p.y.round())
        } else {
            p
        }
    }
    fn handle_at(&self, p: Point) -> Option<usize> {
        let editor = self.editor.borrow();
        if editor.selection.len() != 1 {
            return None;
        }
        let id = editor.selection[0];
        let node = editor.document.nodes.get(&id)?;
        if node.locked {
            return None;
        }
        let transform = editor.document.world_transform(id)?;
        let corners = node.local_bounds().corners().map(|p| transform.apply(p));
        let tolerance = 8.0 / self.viewport.get().zoom;
        for (i, c) in corners.iter().enumerate() {
            if c.distance(p) < tolerance {
                return Some(i);
            }
        }
        let top = (corners[0] + corners[1]) * 0.5;
        let center = transform.apply(Point::new(node.width / 2.0, node.height / 2.0));
        let direction = top - center;
        let length = direction.distance(Point::new(0.0, 0.0)).max(0.001);
        if (top + direction * (24.0 / self.viewport.get().zoom / length)).distance(p) < tolerance {
            return Some(4);
        }
        None
    }
    fn new_node(&self, tool: Tool, p: Point) -> NodeId {
        let mut editor = self.editor.borrow_mut();
        editor.begin_transaction(format!("Draw {}", tool.name()));
        let mut node = Node::new(tool.kind(), tool.name(), p.x, p.y, 1.0, 1.0)
            .with_fill(Color::rgb(123, 97, 255));
        if tool == Tool::Frame {
            node = node.with_fill(Color::WHITE);
            node.clip_content = true;
        }
        if tool == Tool::Section {
            node = node.with_fill(Color::rgba(216, 209, 255, 48));
            node.width = 400.0;
            node.height = 300.0;
        }
        if tool == Tool::Text {
            node = node.with_text("Text", 24.0, Color::rgb(30, 30, 30));
            node.width = 160.0;
            node.height = 36.0;
        }
        if matches!(tool, Tool::Line | Tool::Pen | Tool::Pencil) {
            node.fills.clear();
            node.strokes = vec![core::Stroke::new(Color::rgb(30, 30, 30), 2.0)];
        }
        if tool == Tool::Star {
            node.polygon_sides = 5;
        }
        let parent = editor.active_page;
        let id = editor
            .document
            .insert_node(parent, node)
            .expect("valid page");
        editor.selection = vec![id];
        editor.refresh();
        id
    }
}
impl InputElementImpl for DesignCanvas {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);
        this.layout_grids.borrow_mut().clear_snap();
        this.cancel_linear_gradient_gesture();
        this.cancel_corner_radius_gesture();
        this.cancel_stroke_width_gesture();
        this.cancel_network_bucket_gesture();
        this.cancel_layout_drag();
        this.focus();
        e.set_handled(true);
        let screen = e.get_position(Some(this));
        let screen = Point::new(screen.x, screen.y);
        this.selection_pointer.set(Some(screen));
        let world = this.viewport.get().screen_to_world(screen);
        this.pointer_presses.borrow_mut().record(e, screen, world, this.viewport.get(),
            this.editor.borrow().selection.len(), this.frames.get());
        if e.properties().is_right_button_pressed {
            if this.presentation.get().is_none() {
                this.emit(CanvasEvent::ContextMenu);
            }
            return;
        }
        e.pointer()
            .capture(Some(&this.to_ref().upcast::<InputElement>()));
        if this.presentation.get().is_some() {
            if let Some(id) = this.prototype_pick(world) {
                this.prototype_press.set(Some((id, screen, false)));
                this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::Press));
            }
            return;
        }
        if this.space.get()
            || this.tool.get() == Tool::Hand
            || e.properties().is_middle_button_pressed
        {
            *this.gesture.borrow_mut() = Gesture::Pan {
                start: screen,
                original: this.viewport.get().pan,
            };
            return;
        }
        if this.linear_gradient_press(world, e.click_count()) { return; }
        if this.image_crop_press(world) { return; }
        if this.stroke_width_press(world) { return; }
        let additive = e.key_modifiers().contains(KeyModifiers::SHIFT);
        match this.tool.get() {
            Tool::Comment => this.emit(CanvasEvent::Comment(world)),
            Tool::Select => {
                let deep = e.key_modifiers().intersects(KeyModifiers::CONTROL | KeyModifiers::META);
                if this.corner_radius_press(world, e.key_modifiers()) { return; }
                this.clear_corner_radius_hover();
                if e.key_modifiers().contains(KeyModifiers::ALT) && !deep && this.vector_node.get().is_none() {
                    this.cycle_selection_at(world, if additive { core::SelectionCycle::InFront } else { core::SelectionCycle::Behind });
                    return;
                }
                if this.vector_press(world, e.click_count(), additive) {
                    return;
                }
                if let Some(corner) = if deep { None } else { this.handle_at(world) } {
                    let mut editor = this.editor.borrow_mut();
                    let id = editor.selection[0];
                    let node = editor.document.nodes[&id].clone();
                    if corner == 4 {
                        let center = editor
                            .document
                            .world_transform(id)
                            .unwrap()
                            .apply(Point::new(node.width / 2.0, node.height / 2.0));
                        let angle = (world.y - center.y).atan2(world.x - center.x).to_degrees();
                        editor.begin_node_transaction(&[id], "Rotate");
                        *this.gesture.borrow_mut() = Gesture::Rotate {
                            node,
                            center,
                            angle,
                        };
                    } else {
                        editor.begin_transaction("Resize");
                        *this.gesture.borrow_mut() = Gesture::Resize { node, corner };
                    }
                } else if let Some(id) = this.pick_with_modifiers(world, e.key_modifiers(), e.click_count() == 2) {
                    let mut editor = this.editor.borrow_mut();
                    if additive {
                        if editor.selection.contains(&id) {
                            editor.selection.retain(|x| *x != id);
                        } else {
                            editor.selection.push(id);
                        }
                    } else if !editor.selection.contains(&id) {
                        editor.selection = vec![id];
                    }
                    if e.click_count() == 2 && editor.document.nodes[&id].kind == NodeKind::Text {
                        drop(editor);
                        this.emit(CanvasEvent::EditText(id));
                        return;
                    }
                    if e.click_count() == 2 && editor.document.nodes[&id].kind == NodeKind::Vector {
                        drop(editor);
                        this.edit_vector();
                        return;
                    }
                    if this.layout_drag_press(&editor, id, world, screen) {
                        drop(editor);
                        this.emit(CanvasEvent::Selection);
                        this.invalidate_visual();
                        return;
                    }
                    let exclude = editor.root_selection();
                    let bounds = exclude
                        .iter()
                        .filter_map(|&id| editor.document.world_bounds(id))
                        .reduce(|a, b| a.union(b));
                    let nodes = exclude
                        .iter()
                        .filter_map(|id| editor.document.nodes.get(id).cloned())
                        .collect();
                    editor.begin_node_transaction(&exclude, "Move layers");
                    if let Some(probe) = this.free_layout_probe.borrow_mut().as_mut() {
                        probe.record_transaction(&editor);
                    }
                    *this.gesture.borrow_mut() = Gesture::Move {
                        start: world,
                        nodes,
                        bounds,
                        exclude,
                        moved: false,
                    };
                    drop(editor);
                    this.emit(CanvasEvent::Selection);
                } else {
                    if !additive {
                        this.editor.borrow_mut().selection.clear();
                    }
                    *this.gesture.borrow_mut() = Gesture::Marquee {
                        start: world,
                        current: world,
                        additive,
                    };
                    this.emit(CanvasEvent::Selection);
                }
            }
            Tool::Pen => {
                let mut p = this.snap_point(world);
                if this.snap.get() {
                    let editor = this.editor.borrow();
                    let exclude: Vec<_> = this.pen_node.get().into_iter().collect();
                    if let Some(snap) = this.layout_grids.borrow_mut().snap(&editor, &[p],
                        5.0 / this.viewport.get().zoom, &exclude, None) { p = p + snap.delta; }
                }
                let id = if let Some(id) = this.pen_node.get() {
                    id
                } else {
                    let id = this.new_node(Tool::Pen, p);
                    this.pen_node.set(Some(id));
                    id
                };
                let mut close = false;
                let mut handle = None;
                {
                    let mut editor = this.editor.borrow_mut();
                    let node = editor.document.nodes.get_mut(&id).unwrap();
                    let local = Point::new(p.x - node.x, p.y - node.y);
                    if let Some(PathSegment::MoveTo { point }) = node.path.segments.first() {
                        if node.path.segments.len() > 2
                            && local.distance(*point) < 8.0 / this.viewport.get().zoom
                        {
                            if let Some(control1) = this.pen_tangent.take() {
                                let start = *point;
                                node.path.segments.push(PathSegment::CubicTo {
                                    control1,
                                    control2: start,
                                    point: start,
                                });
                            }
                            node.path.segments.push(PathSegment::Close);
                            node.fills = vec![core::Fill::solid(Color::rgb(123, 97, 255))];
                            close = true;
                        } else {
                            let segment = if let Some(control1) = this.pen_tangent.take() {
                                PathSegment::CubicTo {
                                    control1,
                                    control2: local,
                                    point: local,
                                }
                            } else {
                                PathSegment::LineTo { point: local }
                            };
                            node.path.segments.push(segment);
                        }
                    } else {
                        node.path = VectorPath::default();
                        node.path
                            .segments
                            .push(PathSegment::MoveTo { point: local });
                    }
                    if !close {
                        handle = Some(Gesture::PenHandle {
                            id,
                            segment: node.path.segments.len() - 1,
                            anchor: local,
                            start: p,
                        });
                    }
                    node.width = node.width.max(local.x);
                    node.height = node.height.max(local.y);
                    editor.refresh();
                }
                if let Some(handle) = handle {
                    *this.gesture.borrow_mut() = handle;
                }
                if close {
                    this.finish_pen();
                    this.tool.set(Tool::Select);
                }
                this.emit(CanvasEvent::Selection);
            }
            tool => {
                let mut start = this.snap_point(world);
                if this.snap.get() {
                    let editor = this.editor.borrow();
                    if let Some(snap) = this.layout_grids.borrow_mut().snap(&editor, &[start],
                        5.0 / this.viewport.get().zoom, &[], None) { start = start + snap.delta; }
                }
                let id = this.new_node(tool, start);
                if tool == Tool::Text {
                    this.editor.borrow_mut().commit_transaction();
                    this.tool.set(Tool::Select);
                    this.emit(CanvasEvent::Document);
                    this.emit(CanvasEvent::EditText(id));
                } else if tool == Tool::Pencil {
                    this.editor
                        .borrow_mut()
                        .document
                        .nodes
                        .get_mut(&id)
                        .unwrap()
                        .path
                        .segments
                        .push(PathSegment::MoveTo {
                            point: Point::new(0.0, 0.0),
                        });
                    *this.gesture.borrow_mut() = Gesture::Pencil { id, origin: start };
                } else {
                    *this.gesture.borrow_mut() = Gesture::Draw { start, id };
                }
                this.emit(CanvasEvent::Selection);
            }
        }
        this.invalidate_visual();
    }
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);
        let screen = e.get_position(Some(this));
        this.selection_pointer.set(Some(Point::new(screen.x, screen.y)));
        let world = this.world(screen);
        if this.presentation.get().is_some() {
            this.emit(CanvasEvent::PrototypePointerMoved(Point::new(screen.x, screen.y)));
            let world = this.world(screen);
            let hover = this.prototype_pick(world);
            let previous = this.hover.replace(hover);
            if previous != hover {
                if let Some(id) = previous {
                    this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::MouseLeave));
                }
                if let Some(id) = hover {
                    this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::Hover));
                }
            }
            if let Some((id, start, dragged)) = this.prototype_press.get() {
                if !dragged && start.distance(Point::new(screen.x, screen.y)) >= 5.0 {
                    this.prototype_press.set(Some((id, start, true)));
                    this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::Drag));
                }
            }
            return;
        }
        if this.linear_gradient_move(world) { return; }
        if this.corner_radius_move(world) { return; }
        if this.image_crop_move(world) { return; }
        if this.stroke_width_move(world) { return; }
        if this.network_bucket_move(world, e.key_modifiers().contains(KeyModifiers::SHIFT)) { return; }
        let shift = e.key_modifiers().contains(KeyModifiers::SHIFT);
        if this.layout_drag_move_constrained(world, Point::new(screen.x, screen.y), shift) { return; }
        if this.corner_radius_hover(world, e.key_modifiers()) {
            if this.hover.take().is_some() { this.invalidate_visual(); }
            return;
        }
        let mut camera_changed = false;
        let mut gesture = this.gesture.borrow_mut();
        match &mut *gesture {
            Gesture::None => {
                let hover = this.pick_with_modifiers(world, e.key_modifiers(), false);
                if this.hover.replace(hover) != hover {
                    this.invalidate_visual();
                }
                return;
            }
            Gesture::Pan { start, original } => {
                let mut v = this.viewport.get();
                v.pan = *original + Point::new(screen.x - start.x, screen.y - start.y);
                this.viewport.set(v);
                camera_changed = true;
            }
            Gesture::Move {
                start,
                nodes,
                bounds,
                exclude,
                moved,
            } => {
                let mut delta = world - *start;
                if delta.distance(Point::new(0.0, 0.0)) < 0.5 / this.viewport.get().zoom && !*moved
                {
                    return;
                }
                if shift {
                    if delta.x.abs() > delta.y.abs() {
                        delta.y = 0.0
                    } else {
                        delta.x = 0.0
                    }
                }
                if this.snap.get() {
                    delta.x = delta.x.round();
                    delta.y = delta.y.round();
                }
                let mut editor = this.editor.borrow_mut();
                this.snap_guides.set((None, None));
                this.layout_grids.borrow_mut().clear_snap();
                if this.snap.get() {
                    if let Some(bounds) = bounds {
                        let proposed = Rect::new(
                            bounds.x + delta.x,
                            bounds.y + delta.y,
                            bounds.width,
                            bounds.height,
                        );
                        // An axis-constrained drag must stay on its original axis.
                        let snap_x = !shift || delta.x != 0.0;
                        let snap_y = !shift || delta.y != 0.0;
                        let direction = shift.then_some(if snap_x { Point::new(1.0, 0.0) } else { Point::new(0.0, 1.0) });
                        let guide = this.layout_grids.borrow_mut().snap_bounds(&editor, proposed,
                            5.0 / this.viewport.get().zoom, exclude, direction);
                        if let Some(guide) = guide {
                            delta = delta + guide.delta;
                        } else {
                            let snap = editor.smart_snap(proposed, 5.0 / this.viewport.get().zoom, exclude);
                            if snap_x { delta.x += snap.delta.x; }
                            if snap_y { delta.y += snap.delta.y; }
                            this.snap_guides.set((
                                if snap_x { snap.vertical_guide } else { None },
                                if snap_y { snap.horizontal_guide } else { None },
                            ));
                        }
                    }
                }
                for original in nodes.iter() {
                    if editor.document.effective_locked(original.id) { continue; }
                    let parent = original
                        .parent
                        .and_then(|id| editor.document.world_transform(id))
                        .unwrap_or(core::Transform::IDENTITY);
                    let local = parent
                        .inverse()
                        .unwrap_or(core::Transform::IDENTITY)
                        .apply_vector(delta);
                    if let Some(node) = editor.document.nodes.get_mut(&original.id) {
                        node.x = original.x + local.x;
                        node.y = original.y + local.y;
                    }
                }
                editor.refresh_preview_nodes(exclude);
                if let Some(probe) = this.free_layout_probe.borrow_mut().as_mut() {
                    probe.record_revision(editor.revision);
                }
                *moved = true;
            }
            Gesture::Resize { node, corner } => {
                let mut editor = this.editor.borrow_mut();
                this.layout_grids.borrow_mut().clear_snap();
                if editor.document.effective_locked(node.id) { return; }
                let parent = node
                    .parent
                    .and_then(|id| editor.document.world_transform(id))
                    .unwrap_or(core::Transform::IDENTITY);
                let transform = parent * node.local_transform();
                let local = transform
                    .inverse()
                    .unwrap_or(core::Transform::IDENTITY)
                    .apply(world);
                let opposite = match *corner {
                    0 => Point::new(node.width, node.height),
                    1 => Point::new(0.0, node.height),
                    2 => Point::new(0.0, 0.0),
                    _ => Point::new(node.width, 0.0),
                };
                let mut width = (local.x - opposite.x).abs().max(1.0);
                let mut height = (local.y - opposite.y).abs().max(1.0);
                if shift {
                    let ratio = node.width / node.height.max(0.001);
                    if width / height > ratio {
                        height = width / ratio
                    } else {
                        width = height * ratio
                    }
                }
                if this.snap.get() {
                    width = width.round().max(1.0);
                    height = height.round().max(1.0);
                    // Project the final handle after pixel/aspect calculations.
                    // A later rounding pass must not undo the guide alignment.
                    let sign = match *corner {
                        0 => Point::new(-1.0, -1.0), 1 => Point::new(1.0, -1.0),
                        2 => Point::new(1.0, 1.0), _ => Point::new(-1.0, 1.0),
                    };
                    let handle = transform.apply(opposite + Point::new(sign.x * width, sign.y * height));
                    let direction = shift.then_some(transform.apply_vector(Point::new(sign.x * width, sign.y * height)));
                    let guide = this.layout_grids.borrow_mut().snap(&editor, &[handle],
                        5.0 / this.viewport.get().zoom, &[node.id], direction);
                    if let (Some(guide), Some(inverse)) = (guide, transform.inverse()) {
                        let local = inverse.apply(handle + guide.delta);
                        let adjusted_width = sign.x * (local.x - opposite.x);
                        let adjusted_height = sign.y * (local.y - opposite.y);
                        if adjusted_width >= 1.0 && adjusted_height >= 1.0 {
                            width = adjusted_width; height = adjusted_height;
                        } else { this.layout_grids.borrow_mut().clear_snap(); }
                    }
                }
                let new_opposite = match *corner {
                    0 => Point::new(width, height),
                    1 => Point::new(0.0, height),
                    2 => Point::new(0.0, 0.0),
                    _ => Point::new(width, 0.0),
                };
                let mut updated = node.clone();
                updated.x = 0.0;
                updated.y = 0.0;
                updated.width = width;
                updated.height = height;
                let anchor_in_parent = node.local_transform().apply(opposite);
                let offset = updated.local_transform().apply(new_opposite);
                editor.resize_node_and_position(
                    node.id,
                    width,
                    height,
                    Point::new(anchor_in_parent.x - offset.x, anchor_in_parent.y - offset.y),
                );
            }
            Gesture::Rotate {
                node,
                center,
                angle,
            } => {
                let mut rotation = node.rotation
                    + (world.y - center.y).atan2(world.x - center.x).to_degrees()
                    - *angle;
                if shift {
                    rotation = (rotation / 15.0).round() * 15.0
                }
                let mut editor = this.editor.borrow_mut();
                if let Some(n) = editor.document.nodes.get_mut(&node.id) {
                    n.rotation = rotation;
                }
                editor.refresh_preview_nodes(&[node.id]);
            }
            Gesture::Marquee { current, .. } => *current = world,
            Gesture::Draw { start, id } => {
                let mut end = this.snap_point(world);
                if shift {
                    let size = (end.x - start.x).abs().max((end.y - start.y).abs());
                    end = Point::new(
                        start.x + size * (end.x - start.x).signum(),
                        start.y + size * (end.y - start.y).signum(),
                    );
                }
                this.layout_grids.borrow_mut().clear_snap();
                if this.snap.get() {
                    let editor = this.editor.borrow();
                    let direction = shift.then_some(end - *start);
                    if let Some(snap) = this.layout_grids.borrow_mut().snap(&editor, &[end],
                        5.0 / this.viewport.get().zoom, &[*id], direction) { end = end + snap.delta; }
                }
                let bounds = Rect::from_points(*start, end);
                let mut editor = this.editor.borrow_mut();
                if let Some(n) = editor.document.nodes.get_mut(id) {
                    n.x = bounds.x;
                    n.y = bounds.y;
                    n.width = bounds.width.max(1.0);
                    n.height = bounds.height.max(1.0);
                    if n.kind == NodeKind::Line {
                        n.flip_y = (end.x - start.x) * (end.y - start.y) < 0.0;
                    }
                }
                editor.refresh();
            }
            Gesture::Pencil { id, origin } => {
                let local = world - *origin;
                let mut editor = this.editor.borrow_mut();
                if let Some(n) = editor.document.nodes.get_mut(id) {
                    // Sample at approximately one screen pixel to avoid redundant path segments.
                    let last = n.path.segments.last().and_then(|s| match s {
                        PathSegment::MoveTo { point } | PathSegment::LineTo { point } => {
                            Some(*point)
                        }
                        _ => None,
                    });
                    if last.is_none_or(|p| p.distance(local) > 0.8 / this.viewport.get().zoom) {
                        n.path.segments.push(PathSegment::LineTo { point: local });
                        n.width = n.width.max(local.x);
                        n.height = n.height.max(local.y);
                    }
                }
                editor.refresh();
            }
            Gesture::PenHandle {
                id,
                segment,
                anchor,
                start,
            } => {
                let mut delta = world - *start;
                if delta.distance(Point::default()) < 2.0 / this.viewport.get().zoom {
                    return;
                }
                if shift {
                    let angle = (delta.y.atan2(delta.x) / std::f64::consts::FRAC_PI_4).round()
                        * std::f64::consts::FRAC_PI_4;
                    let length = delta.distance(Point::default());
                    delta = Point::new(angle.cos() * length, angle.sin() * length);
                }
                let mut editor = this.editor.borrow_mut();
                if let Some(node) = editor.document.nodes.get_mut(id) {
                    let incoming = *anchor - delta;
                    let previous = node
                        .path
                        .segments
                        .get(segment.saturating_sub(1))
                        .and_then(|s| match s {
                            PathSegment::MoveTo { point }
                            | PathSegment::LineTo { point }
                            | PathSegment::QuadraticTo { point, .. }
                            | PathSegment::CubicTo { point, .. } => Some(*point),
                            PathSegment::Close => None,
                        })
                        .unwrap_or(*anchor);
                    match node.path.segments.get_mut(*segment) {
                        Some(s @ PathSegment::LineTo { .. }) => {
                            *s = PathSegment::CubicTo {
                                control1: previous,
                                control2: incoming,
                                point: *anchor,
                            };
                        }
                        Some(PathSegment::CubicTo { control2, .. }) => *control2 = incoming,
                        _ => {}
                    }
                    this.pen_tangent.set(Some(*anchor + delta));
                }
                editor.refresh();
            }
            Gesture::NetworkPoint { node, point, start, original } => {
                let mut delta = world - *start;
                if shift { if delta.x.abs() > delta.y.abs() { delta.y = 0.0; } else { delta.x = 0.0; } }
                let position = if matches!(point, core::NetworkPointId::Vertex(_)) && this.snap.get() { this.snap_point(*original + delta) } else { *original + delta };
                this.editor.borrow_mut().move_network_point(*node, *point, position);
            }
            Gesture::NetworkBend { node, edge, t } => {
                this.editor.borrow_mut().bend_network_edge(*node, *edge, *t, world);
            }
            Gesture::VectorBend { node, segment, t } => {
                this.editor.borrow_mut().bend_vector_segment(*node, *segment, *t, world);
            }
            Gesture::VectorPoint {
                node,
                point,
                start,
                original,
            } => {
                let mut delta = world - *start;
                if shift {
                    if delta.x.abs() > delta.y.abs() {
                        delta.y = 0.0;
                    } else {
                        delta.x = 0.0;
                    }
                }
                let position = if point.kind == PathPointKind::Anchor && this.snap.get() {
                    this.snap_point(*original + delta)
                } else {
                    *original + delta
                };
                this.editor
                    .borrow_mut()
                    .move_vector_point(*node, *point, position);
            }
        }
        drop(gesture);
        this.invalidate_visual();
        if camera_changed {
            this.emit(CanvasEvent::Camera);
        }
    }
    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);
        if this.presentation.get().is_some() {
            let press = this.prototype_press.take();
            e.pointer().capture(None);
            if let Some((id, _, _)) = press {
                this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::Release));
            }
            let point = this.world(e.get_position(Some(this)));
            if let Some((id, _, false)) = press {
                if this.prototype_pick(point) == Some(id) {
                    this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::Click));
                }
            }
            e.set_handled(true);
            return;
        }
        if this.linear_gradient_active() {
            this.linear_gradient_move(this.world(e.get_position(Some(this))));
        }
        if this.finish_linear_gradient_edit() {
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        if this.corner_radius_active() {
            this.corner_radius_move(this.world(e.get_position(Some(this))));
        }
        if this.finish_corner_radius_edit() {
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        if this.image_crop_release() {
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        if this.stroke_width_active() {
            this.stroke_width_move(this.world(e.get_position(Some(this))));
        }
        if this.finish_stroke_width_edit() {
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        if this.network_bucket_state().1 {
            this.network_bucket_move(this.world(e.get_position(Some(this))), e.key_modifiers().contains(KeyModifiers::SHIFT));
        }
        if this.finish_network_bucket_gesture(true) {
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        let release_screen = e.get_position(Some(this));
        if this.layout_drag_release_constrained(this.world(release_screen), Point::new(release_screen.x, release_screen.y),
            e.key_modifiers().contains(KeyModifiers::SHIFT)) {
            e.pointer().capture(None);
            e.set_handled(true);
            return;
        }
        this.snap_guides.set((None, None));
        this.layout_grids.borrow_mut().clear_snap();
        let gesture = std::mem::replace(&mut *this.gesture.borrow_mut(), Gesture::None);
        // Clear the gesture before releasing capture: capture-lost is synchronous.
        e.pointer().capture(None);
        match gesture {
            Gesture::Marquee {
                start,
                current,
                additive,
            } => {
                this.editor
                    .borrow_mut()
                    .select_rect(Rect::from_points(start, current), additive);
                this.emit(CanvasEvent::Selection);
            }
            Gesture::Move { moved: false, .. } => {
                this.editor.borrow_mut().cancel_transaction();
            }
            Gesture::Pencil { id, .. } => {
                let mut editor = this.editor.borrow_mut();
                normalize_drawn_vector(&mut editor, id);
                editor.commit_transaction();
                drop(editor);
                this.tool.set(Tool::Select);
                this.emit(CanvasEvent::Document);
            }
            Gesture::Move { .. }
            | Gesture::Resize { .. }
            | Gesture::Rotate { .. }
            | Gesture::VectorPoint { .. }
            | Gesture::VectorBend { .. }
            | Gesture::NetworkPoint { .. }
            | Gesture::NetworkBend { .. }
            | Gesture::Draw { .. } => {
                this.editor.borrow_mut().commit_transaction();
                this.tool.set(Tool::Select);
                this.emit(CanvasEvent::Document);
            }
            _ => {}
        }
        this.invalidate_visual();
    }
    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);
        if this.cancel_linear_gradient_gesture() { return; }
        if this.cancel_corner_radius_gesture() { return; }
        if this.cancel_stroke_width_gesture() { return; }
        if this.cancel_network_bucket_gesture() { return; }
        if this.cancel_layout_drag() { return; }
        if this.cancel_image_crop_gesture() { return; }
        if let Some((id, _, _)) = this.prototype_press.take() {
            if this.presentation.get().is_some() {
                this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::Release));
            }
        }
        if !matches!(*this.gesture.borrow(), Gesture::None) {
            this.cancel();
        }
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.clear_gradient_hover();
        this.clear_corner_radius_hover();
        this.clear_stroke_width_hover();
        this.clear_bucket_hover();
        if this.presentation.get().is_some() {
            this.emit(CanvasEvent::PrototypePointerExited);
            if let Some(id) = this.hover.take() {
                this.emit(CanvasEvent::PrototypeTrigger(id, core::Trigger::MouseLeave));
            }
        }
    }
    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        this.cancel_linear_gradient_gesture();
        this.clear_gradient_hover();
        this.cancel_corner_radius_gesture();
        this.clear_corner_radius_hover();
        this.cancel_stroke_width_gesture();
        this.clear_stroke_width_hover();
        this.cancel_network_bucket_gesture();
        this.clear_bucket_hover();
        e.set_handled(true);
        if let Some(root) = this.presentation.get() {
            let world = this.world(e.get_position(Some(this)));
            let target = this.prototype_pick(world).unwrap_or(root);
            this.emit(CanvasEvent::PrototypeScroll(target, Point::new(-e.delta().x * 36.0, -e.delta().y * 36.0)));
            return;
        }
        let mut v = this.viewport.get();
        this.cancel_layout_drag();
        if e.key_modifiers()
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
        {
            let p = e.get_position(Some(this));
            v.zoom_at(Point::new(p.x, p.y), v.zoom * (e.delta().y * 0.16).exp());
        } else {
            v.pan = v.pan + Point::new(e.delta().x * 36.0, e.delta().y * 36.0);
        }
        this.viewport.set(v);
        this.invalidate_visual();
        this.emit(CanvasEvent::Camera);
    }
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if this.presentation.get().is_some() {
            this.emit(CanvasEvent::PrototypeKey(prototype_key_chord(
                e.key,
                e.key_modifiers,
                e.key_symbol.as_deref(),
            )));
            e.set_handled(true);
            return;
        }
        if this.linear_gradient_key(e.key, e.key_modifiers) { e.set_handled(true); return; }
        if e.key == Key::Escape && this.cancel_corner_radius_gesture() {
            e.set_handled(true);
            return;
        }
        if this.image_crop_key(e.key) { e.set_handled(true); return; }
        if this.stroke_width_key(e.key, e.key_modifiers) { e.set_handled(true); return; }
        if e.key == Key::Escape && this.cancel_network_bucket_gesture() {
            e.set_handled(true);
            return;
        }
        if e.key == Key::Space {
            this.cancel_linear_gradient_gesture();
            this.clear_gradient_hover();
            this.cancel_corner_radius_gesture();
            this.clear_corner_radius_hover();
            this.cancel_stroke_width_gesture();
            this.clear_stroke_width_hover();
            this.cancel_network_bucket_gesture();
            this.clear_bucket_hover();
            this.space.set(true);
            e.set_handled(true);
            return;
        }
        if e.key == Key::Escape {
            if this.presentation.get().is_some() {
                this.emit(CanvasEvent::ExitPresentation)
            } else if this.vector_node.get().is_some()
                && matches!(*this.gesture.borrow(), Gesture::None)
            {
                this.leave_vector_edit();
                this.emit(CanvasEvent::Selection);
            } else {
                if !this.cancel_active_edit() && !this.editor.borrow().has_active_transaction() {
                    this.editor.borrow_mut().clear_selection();
                    this.hover.set(None);
                    this.invalidate_visual();
                    this.emit(CanvasEvent::Selection);
                }
            }
            e.set_handled(true);
            return;
        }
        if e.key == Key::Enter && this.pen_node.get().is_some() {
            this.finish_pen();
            this.tool.set(Tool::Select);
            this.emit(CanvasEvent::Tool);
            e.set_handled(true);
            return;
        }
        let command = e
            .key_modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
        let shift = e.key_modifiers.contains(KeyModifiers::SHIFT);
        let alt = e.key_modifiers.contains(KeyModifiers::ALT);
        if matches!(e.key, Key::Left | Key::Right | Key::Up | Key::Down | Key::Enter | Key::Tab | Key::Delete | Key::Back) {
            this.finish_corner_radius_edit();
            this.finish_network_bucket();
        }
        if this.selection_key(e) {
            e.set_handled(true);
            return;
        }
        if !command && matches!(e.key, Key::Delete | Key::Back) && this.vector_node.get().is_some()
        {
            if this.delete_network_selection() { e.set_handled(true); return; }
            if let (Some(id), Some(point)) = (this.vector_node.get(), this.vector_point.get()) {
                if point.kind == PathPointKind::Anchor {
                    this.editor
                        .borrow_mut()
                        .delete_vector_anchor(id, point.segment);
                    this.vector_point.set(None);
                    this.emit(CanvasEvent::Document);
                }
            }
            e.set_handled(true);
            return;
        }
        let action = if command {
            match e.key {
                Key::Z => Some(if shift { "redo" } else { "undo" }),
                Key::Y => Some("redo"),
                Key::C => Some("copy"),
                Key::X => Some("cut"),
                Key::V => Some("paste"),
                Key::D => Some("duplicate"),
                Key::A => Some(if shift { "select-inverse" } else { "select-all" }),
                Key::G => Some(if alt {
                    "frame-selection"
                } else if shift {
                    "ungroup"
                } else {
                    "group"
                }),
                Key::S => Some("save"),
                Key::O => Some("open"),
                Key::K => Some(if shift {
                    "place-image"
                } else if alt {
                    "component"
                } else {
                    "actions"
                }),
                _ => None,
            }
        } else {
            match e.key {
                Key::Delete | Key::Back => Some("delete"),
                Key::V => Some("select"),
                Key::F => Some(if shift { "section" } else { "frame" }),
                Key::R => Some("rectangle"),
                Key::O => Some("ellipse"),
                Key::L => Some("line"),
                Key::P => Some(if shift { "pencil" } else { "pen" }),
                Key::B if shift && this.vector_node.get().is_some() => Some("network-bucket"),
                Key::T => Some("text"),
                Key::H => Some("hand"),
                Key::C => Some("comment"),
                Key::A if shift => Some("auto-layout"),
                Key::S if shift => Some("section"),
                Key::D1 => Some("fit"),
                Key::D2 => Some("fit-selection"),
                Key::K => Some("actions"),
                _ => None,
            }
        };
        if let Some(action) = action {
            if !matches!(action, "undo" | "redo") { this.cancel_layout_drag(); }
            this.emit(CanvasEvent::Action(action));
            e.set_handled(true);
            return;
        }
        let step = if shift { 10.0 } else { 1.0 };
        let delta = match e.key {
            Key::Left => Some((-step, 0.0)),
            Key::Right => Some((step, 0.0)),
            Key::Up => Some((0.0, -step)),
            Key::Down => Some((0.0, step)),
            _ => None,
        };
        if let Some((x, y)) = delta {
            this.cancel_layout_drag();
            if let (Some(id), Some(point)) = (this.vector_node.get(), this.vector_point.get()) {
                let mut editor = this.editor.borrow_mut();
                let world = editor
                    .document
                    .nodes
                    .get(&id)
                    .and_then(|n| n.path.point_position(point))
                    .zip(editor.document.world_transform(id))
                    .map(|(p, t)| t.apply(p));
                if let Some(world) = world {
                    editor.move_vector_point(id, point, world + Point::new(x, y));
                }
            } else {
                this.editor.borrow_mut().move_selection(x, y);
            }
            this.emit(CanvasEvent::Document);
            this.invalidate_visual();
            e.set_handled(true);
        }
    }
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_up(this, e);
        if e.key == Key::Space {
            this.space.set(false);
            e.set_handled(true);
        }
    }
}
fn prototype_key_chord(key: Key, modifiers: KeyModifiers, symbol: Option<&str>) -> String {
    let mut chord = String::new();
    for (modifier, name) in [
        (KeyModifiers::CONTROL, "Ctrl+"),
        (KeyModifiers::ALT, "Alt+"),
        (KeyModifiers::SHIFT, "Shift+"),
        (KeyModifiers::META, "Meta+"),
    ] {
        if modifiers.contains(modifier) {
            chord.push_str(name);
        }
    }
    let fallback = key.to_string();
    let punctuation = symbol.filter(|symbol| {
        symbol.chars().count() == 1
            && symbol.chars().next().is_some_and(|symbol| {
                !symbol.is_alphanumeric() && !symbol.is_whitespace() && !symbol.is_control()
            })
    });
    let name = if key == Key::Back {
        "Backspace"
    } else if let Some(symbol) = punctuation {
        // Use the typed symbol instead of a layout-specific Oem virtual-key
        // name. The core absorbs redundant Shift in symbolic chords.
        symbol
    } else if fallback.len() == 2
        && fallback.starts_with('D')
        && fallback.as_bytes()[1].is_ascii_digit()
    {
        &fallback[1..]
    } else {
        &fallback
    };
    chord.push_str(name);
    chord
}

#[cfg(test)]
mod prototype_key_tests {
    use super::*;

    #[test]
    fn native_key_adapter_preserves_symbols_backspace_and_named_shift_chords() {
        for (key, modifiers, symbol, expected) in [
            (
                Key::OemPlus,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                Some("+"),
                "ctrl++",
            ),
            (Key::OemPlus, KeyModifiers::CONTROL, Some("+"), "ctrl++"),
            (Key::D1, KeyModifiers::SHIFT, Some("!"), "!"),
            (Key::Back, KeyModifiers::CONTROL, None, "ctrl+backspace"),
            (
                Key::A,
                KeyModifiers::CONTROL | KeyModifiers::SHIFT,
                Some("A"),
                "ctrl+shift+a",
            ),
            (Key::F1, KeyModifiers::SHIFT, None, "shift+f1"),
            (Key::D1, KeyModifiers::CONTROL, Some("1"), "ctrl+1"),
        ] {
            let chord = prototype_key_chord(key, modifiers, symbol);
            assert_eq!(
                core::normalize_prototype_key(&chord).as_deref(),
                Some(expected)
            );
        }
        // The shell's unmatched-Escape fallback consumes the original named
        // event before core normalization; retain that spelling.
        assert_eq!(
            prototype_key_chord(Key::Escape, KeyModifiers::empty(), None),
            "Escape"
        );
    }
}

fn normalize_drawn_vector(editor: &mut Editor, id: NodeId) {
    if let Some(node) = editor.document.nodes.get_mut(&id) {
        core::normalize_vector_bounds(node);
    }
    editor.refresh();
}

fn ui(p: Point) -> UiPoint {
    UiPoint::new(p.x, p.y)
}
fn rect(r: Rect) -> UiRect {
    UiRect::new(r.x, r.y, r.width, r.height)
}
fn pen(color: u32, width: f64) -> Rc<dyn IPen> {
    Rc::new(ImmutablePen::from_uint32(color, width))
}
fn label(ctx: &mut DrawingContext, text: &str, point: Point, size: f64, color: &str) {
    let text = FormattedText::new(
        text,
        CultureInfo::invariant_culture(),
        FlowDirection::LeftToRight,
        Typeface::from_name("fonts:Inter#Inter"),
        size,
        Some(brush(color)),
    );
    ctx.draw_text(&text, ui(point));
}
fn render_vector_editor(
    ctx: &mut DrawingContext,
    path: &VectorPath,
    transform: core::Transform,
    viewport: Viewport,
    selected: Option<PathPointId>,
) {
    let to_screen = |point| ui(viewport.world_to_screen(transform.apply(point)));
    let geometry = StreamGeometry::new();
    {
        let mut context = geometry.open();
        let mut figure = false;
        for segment in &path.segments {
            match *segment {
                PathSegment::MoveTo { point } => {
                    if figure {
                        context.end_figure(false);
                    }
                    context.begin_figure(to_screen(point), false);
                    figure = true;
                }
                PathSegment::LineTo { point } if figure => context.line_to(to_screen(point), true),
                PathSegment::QuadraticTo { control, point } if figure => {
                    context.quadratic_bezier_to(to_screen(control), to_screen(point), true);
                }
                PathSegment::CubicTo {
                    control1,
                    control2,
                    point,
                } if figure => {
                    context.cubic_bezier_to(
                        to_screen(control1),
                        to_screen(control2),
                        to_screen(point),
                        true,
                    );
                }
                PathSegment::Close => {
                    if figure {
                        context.end_figure(true);
                    }
                    figure = false;
                }
                _ => {}
            }
        }
        if figure {
            context.end_figure(false);
        }
    }
    let blue = pen(0xFF0D99FF, 1.0);
    ctx.draw_geometry(None, Some(&blue), &geometry.upcast());
    let blue_fill = brush("#0D99FF");
    let white = brush("#FFFFFF");
    let points = path.edit_points();
    let anchor = selected.and_then(|id| points.iter().find(|p| p.id == id).map(|p| p.anchor_id));
    for point in points
        .iter()
        .filter(|p| p.id.kind != PathPointKind::Anchor && Some(p.anchor_id) == anchor)
    {
        let position = to_screen(point.position);
        ctx.draw_line(&blue, to_screen(point.anchor), position);
        ctx.draw_ellipse_at(
            Some(if selected == Some(point.id) {
                &blue_fill
            } else {
                &white
            }),
            Some(&blue),
            position,
            3.5,
            3.5,
        );
    }
    for point in points.iter().filter(|p| p.id.kind == PathPointKind::Anchor) {
        let position = to_screen(point.position);
        ctx.draw_rectangle(
            Some(if Some(point.id) == anchor {
                &blue_fill
            } else {
                &white
            }),
            Some(&blue),
            UiRect::new(position.x - 3.5, position.y - 3.5, 7.0, 7.0),
            0.0,
            0.0,
            &BoxShadows::default(),
        );
    }
}
impl VisualImpl for DesignCanvas {
    fn render(this: &Self, ctx: &mut DrawingContext) {
        let bounds = this.bounds();
        let mut v = this.viewport.get();
        v.width = bounds.width;
        v.height = bounds.height;
        if v.width <= 0.0 || v.height <= 0.0 {
            return;
        }
        let editor = this.editor.borrow();
        let prototype_scene = this.prototype_scene.borrow();
        let doc = prototype_scene.as_ref().map(|scene| &scene.document).unwrap_or(&editor.document);
        let roots: Vec<NodeId> = if let Some(scene) = prototype_scene.as_ref() {
            vec![scene.root]
        } else if let Some(root) = this.presentation.get() {
            vec![root]
        } else {
            doc.page(editor.active_page)
                .map(|p| p.children.clone())
                .unwrap_or_default()
        };
        let fitted = prototype_scene.is_none() && !this.initialized.replace(true);
        if fitted {
            if let Some(bounds) = roots
                .iter()
                .filter_map(|&id| doc.world_bounds(id))
                .reduce(|a, b| a.union(b))
            {
                v.fit(bounds, 65.0);
                if this.presentation.get().is_none() {
                    v.pan.y -= 22.0;
                }
            }
        }
        this.viewport.set(v);
        if fitted {
            // Fitting needs the arranged viewport. Update native chrome after
            // rendering so it observes that fit without changing controls here.
            if let Some(callback) = this.callback.borrow().clone() {
                Dispatcher::ui_thread().post_local(
                    move || callback(CanvasEvent::Camera),
                    DispatcherPriority::AFTER_RENDER,
                );
            }
        }
        this.frames.set(this.frames.get() + 1);
        let page_color = doc
            .page(editor.active_page)
            .map(|p| p.background)
            .unwrap_or(Color::rgb(229, 229, 229));
        let page_hex = format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            page_color.a, page_color.r, page_color.g, page_color.b
        );
        let background = brush(if this.presentation.get().is_some() {
            "#181818"
        } else if this.dark.get() {
            "#252525"
        } else {
            &page_hex
        });
        ctx.draw_rectangle(
            Some(&background),
            None,
            UiRect::new(0.0, 0.0, v.width, v.height),
            0.0,
            0.0,
            &BoxShadows::default(),
        );
        if this.show_grid.get() && this.presentation.get().is_none() {
            let step = (32.0 * v.zoom).max(8.0);
            let grid_pen = pen(
                if this.dark.get() {
                    0xFF333333
                } else {
                    0xFFD5D5D5
                },
                0.5,
            );
            let mut x = v.pan.x.rem_euclid(step);
            while x < v.width {
                ctx.draw_line(&grid_pen, UiPoint::new(x, 0.0), UiPoint::new(x, v.height));
                x += step;
            }
            let mut y = v.pan.y.rem_euclid(step);
            while y < v.height {
                ctx.draw_line(&grid_pen, UiPoint::new(0.0, y), UiPoint::new(v.width, y));
                y += step;
            }
        }
        let camera = ctx.push_transform(
            Matrix::create_scale(v.zoom, v.zoom) * Matrix::create_translation(v.pan.x, v.pan.y),
        );
        {
            let display_scale = TopLevel::get_top_level(Some(this))
                .map_or(1.0, |top_level| top_level.render_scaling());
            let mut painter = this.painter.borrow_mut();
            painter.set_raster_scale(v.zoom * display_scale);
            if prototype_scene.is_none() && this.presentation.get().is_none() {
                painter.render_editor(ctx, &editor, &roots, v.visible_rect());
            } else {
                painter.render(ctx, doc, &roots, v.visible_rect());
            }
        }
        ctx.pop(camera);
        if this.presentation.get().is_none() && prototype_scene.is_none() {
            this.layout_grids.borrow_mut().render(ctx, &editor, v);
        }
        if let Some(current) = this.presentation.get() {
            for &(id, position) in this.overlays.borrow().iter().filter(|_| prototype_scene.is_none()) {
                if let (Some(current_bounds), Some(overlay_bounds)) =
                    (doc.world_bounds(current), doc.world_bounds(id))
                {
                    let dim = brush("#66000000");
                    ctx.draw_rectangle(
                        Some(&dim),
                        None,
                        UiRect::new(0.0, 0.0, v.width, v.height),
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                    let dx = current_bounds.x + position.x - overlay_bounds.x;
                    let dy = current_bounds.y + position.y - overlay_bounds.y;
                    let transform = ctx.push_transform(
                        Matrix::create_translation(dx, dy)
                            * Matrix::create_scale(v.zoom, v.zoom)
                            * Matrix::create_translation(v.pan.x, v.pan.y),
                    );
                    let visible = v.visible_rect();
                    this.painter.borrow_mut().render(
                        ctx,
                        doc,
                        &[id],
                        Rect::new(
                            visible.x - dx,
                            visible.y - dy,
                            visible.width,
                            visible.height,
                        ),
                    );
                    ctx.pop(transform);
                }
            }
            label(
                ctx,
                "Esc to return to editor",
                Point::new(20.0, v.height - 30.0),
                11.0,
                "#AAAAAA",
            );
            return;
        }
        let blue = pen(0xFF0D99FF, 1.0);
        let (vertical, horizontal) = this.snap_guides.get();
        if vertical.is_some() || horizontal.is_some() {
            let guide = pen(0xFFDD498B, 1.0);
            if let Some(x) = vertical {
                let x = v.world_to_screen(Point::new(x, 0.0)).x;
                ctx.draw_line(&guide, UiPoint::new(x, 0.0), UiPoint::new(x, v.height));
            }
            if let Some(y) = horizontal {
                let y = v.world_to_screen(Point::new(0.0, y)).y;
                ctx.draw_line(&guide, UiPoint::new(0.0, y), UiPoint::new(v.width, y));
            }
        }
        for &id in &roots {
            if let Some(n) = doc.nodes.get(&id) {
                if n.kind.is_container() && n.visible {
                    if let Some(bounds) = doc.world_bounds(id) {
                        let p = v.world_to_screen(Point::new(bounds.x, bounds.y));
                        if p.x > -400.0 && p.x < v.width && p.y > -30.0 && p.y < v.height {
                            label(
                                ctx,
                                &n.name,
                                Point::new(p.x, p.y - 20.0),
                                11.0,
                                if matches!(n.kind, NodeKind::Component | NodeKind::Instance) {
                                    "#9747FF"
                                } else if this.dark.get() {
                                    "#999999"
                                } else {
                                    "#777777"
                                },
                            );
                        }
                    }
                }
            }
        }
        if let Some(id) = this.hover.get() {
            if !editor.selection.contains(&id) {
                if let Some(b) = doc.world_bounds(id) {
                    let a = v.world_to_screen(Point::new(b.x, b.y));
                    ctx.draw_rectangle_outline(
                        &blue,
                        UiRect::new(a.x, a.y, b.width * v.zoom, b.height * v.zoom),
                        0.0,
                    );
                }
            }
        }
        let white = brush("#FFFFFF");
        for &id in &editor.selection {
            let Some(n) = doc.nodes.get(&id) else {
                continue;
            };
            let Some(t) = doc.world_transform(id) else {
                continue;
            };
            if this.render_linear_gradient(ctx, &editor, id) { continue; }
            if this.render_image_crop(ctx, n, t, v) { continue; }
            if this.render_stroke_width(ctx, &editor, id, t, v) { continue; }
            if this.vector_node.get() == Some(id) {
                if let Some(graph) = &n.vector_network {
                    this.render_bucket_hover(ctx, &editor, id, t, v);
                    network::render_network_editor(ctx, graph, t, v, this.network_point.get(), &this.network_vertices.borrow());
                } else { render_vector_editor(ctx, &n.path, t, v, this.vector_point.get()); }
                continue;
            }
            if this.pen_node.get() == Some(id) {
                let anchor = n
                    .path
                    .edit_points()
                    .into_iter()
                    .filter(|p| p.id.kind == PathPointKind::Anchor)
                    .last();
                render_vector_editor(ctx, &n.path, t, v, anchor.as_ref().map(|p| p.id));
                if let (Some(anchor), Some(tangent)) = (anchor, this.pen_tangent.get()) {
                    let a = v.world_to_screen(t.apply(anchor.position));
                    let b = v.world_to_screen(t.apply(tangent));
                    ctx.draw_line(&blue, ui(a), ui(b));
                    ctx.draw_ellipse_at(Some(&white), Some(&blue), ui(b), 3.5, 3.5);
                }
                continue;
            }
            let corners = n
                .local_bounds()
                .corners()
                .map(|p| v.world_to_screen(t.apply(p)));
            for i in 0..4 {
                ctx.draw_line(&blue, ui(corners[i]), ui(corners[(i + 1) % 4]));
            }
            if editor.selection.len() == 1 {
                for p in corners {
                    ctx.draw_rectangle(
                        Some(&white),
                        Some(&blue),
                        UiRect::new(p.x - 3.0, p.y - 3.0, 6.0, 6.0),
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                }
                let top = (corners[0] + corners[1]) * 0.5;
                let center = (corners[0] + corners[2]) * 0.5;
                let direction = top - center;
                let length = direction.distance(Point::new(0.0, 0.0)).max(0.001);
                let rotate = top + direction * (24.0 / length);
                ctx.draw_line(&blue, ui(top), ui(rotate));
                ctx.draw_ellipse_at(Some(&white), Some(&blue), ui(rotate), 3.0, 3.0);
                if let Some(b) = doc.world_bounds(id) {
                    let p = v.world_to_screen(Point::new(b.x + b.width / 2.0, b.bottom()));
                    let value = format!("{} × {}", n.width.round(), n.height.round());
                    let width = value.len() as f64 * 6.2 + 16.0;
                    let fill = brush("#0D99FF");
                    ctx.draw_rectangle(
                        Some(&fill),
                        None,
                        UiRect::new(p.x - width / 2.0, p.y + 9.0, width, 22.0),
                        4.0,
                        4.0,
                        &BoxShadows::default(),
                    );
                    label(
                        ctx,
                        &value,
                        Point::new(p.x - width / 2.0 + 8.0, p.y + 13.0),
                        10.0,
                        "#FFFFFF",
                    );
                }
                this.render_corner_radius(ctx, &editor, id);
            }
        }
        if let Gesture::Marquee { start, current, .. } = &*this.gesture.borrow() {
            let a = v.world_to_screen(*start);
            let b = v.world_to_screen(*current);
            let fill = brush("#200D99FF");
            ctx.draw_rectangle(
                Some(&fill),
                Some(&blue),
                rect(Rect::from_points(a, b)),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
        }
        this.render_layout_drag(ctx, v);
        for comment in &doc.comments {
            if !comment.resolved && doc.comment_page(comment) == Some(editor.active_page) {
                let p = v.world_to_screen(comment.position);
                let fill = brush("#FFCD29");
                ctx.draw_ellipse_at(Some(&fill), None, ui(p), 10.0, 10.0);
                label(ctx, "C", Point::new(p.x - 3.5, p.y - 6.0), 10.0, "#2A2200");
            }
        }
        if this.show_rulers.get() {
            let fill = brush("#F5F5F5");
            ctx.draw_rectangle(
                Some(&fill),
                None,
                UiRect::new(0.0, 0.0, v.width, 20.0),
                0.0,
                0.0,
                &BoxShadows::default(),
            );
            let step = 100.0 * if v.zoom < 0.3 { 5.0 } else { 1.0 };
            let mut x = (v.visible_rect().x / step).floor() * step;
            let tick = pen(0xFFBBBBBB, 1.0);
            while x < v.visible_rect().right() {
                let px = v.world_to_screen(Point::new(x, 0.0)).x;
                ctx.draw_line(&tick, UiPoint::new(px, 14.0), UiPoint::new(px, 20.0));
                label(
                    ctx,
                    &format!("{x:.0}"),
                    Point::new(px + 3.0, 3.0),
                    9.0,
                    "#777777",
                );
                x += step;
            }
        }
    }
}
