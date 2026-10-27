use crate::DesignerSession;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    MouseButton, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen,
    SolidColorBrush, Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    FerroObjectImpl, Point, Rect, Ref, StyledElementImpl, Visual, VisualImpl, ferro_class,
    ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_designer_core::{Ast, NodeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// Semantic pointer results. A host translates these into layout-adapter transactions.
#[derive(Clone, Debug)]
pub enum DesignGesture {
    Move {
        nodes: Vec<NodeId>,
        delta: Point,
    },
    Resize {
        node: NodeId,
        before: Rect,
        after: Rect,
    },
    /// A rectangle in the target's original local coordinate space, used by oriented handles.
    ResizeLocal {
        node: NodeId,
        after: Rect,
    },
    Rotate {
        node: NodeId,
        degrees: f64,
    },
    Insert {
        source: String,
        position: Point,
    },
}

#[derive(Clone, Copy, Debug)]
enum Handle {
    Move,
    Resize(u8),
    Rotate,
    Marquee,
}
struct Gesture {
    start: Point,
    current: Point,
    handle: Handle,
    rectangles: Vec<(NodeId, Rect)>,
    selection_bounds: Option<ferroui_designer_tools::Rect>,
    snap_index: Option<ferroui_designer_tools::SnapIndex>,
    geometry: Option<crate::affine_manipulation::Geometry>,
}

type CommitGesture = Rc<dyn Fn(DesignGesture)>;

/// Retaining the syntax allocation distinguishes document instances even when a
/// host replaces a session with the same URI, revision, and local node IDs.
struct GestureContext {
    uri: String,
    revision: u64,
    syntax: Arc<Ast>,
}
impl GestureContext {
    fn capture(session: &DesignerSession) -> Self {
        Self {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            syntax: session.document().ast_snapshot(),
        }
    }
    fn matches(&self, session: &DesignerSession) -> bool {
        session.can_manipulate()
            && session.document_uri() == self.uri
            && session.document().revision() == self.revision
            && std::ptr::eq(session.document().ast(), self.syntax.as_ref())
    }
}

/// Transparent native adorner for selecting, moving, resizing and rotating live controls.
/// Motion changes only the adorner; release commits one semantic transaction. Escape and
/// capture loss cancel the gesture, so neither source nor history receive partial drags.
#[repr(C)]
pub struct DesignSurface {
    base: Control,
    session: RefCell<Option<Weak<RefCell<DesignerSession>>>>,
    commit: RefCell<Option<CommitGesture>>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
    gesture: RefCell<Option<Gesture>>,
    insert: RefCell<Option<String>>,
    grid: Cell<f64>,
    interactive: Cell<bool>,
    gesture_context: RefCell<Option<GestureContext>>,
    releasing_capture: Cell<bool>,
    zoom: Cell<f64>,
    snap_guides: RefCell<Vec<ferroui_designer_tools::SnapGuide>>,
    cursor_position: Cell<Option<Point>>,
    pointer_inside: Cell<bool>,
}
ferro_class!(DesignSurface: Control);
ferro_class_info!(DesignSurface {
    new: DesignSurface::new
});
ferro_impl_classes!(DesignSurface: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl DesignSurface {
    pub fn new() -> Ref<Self> {
        let result = instantiate(Self::default());
        result.set_focusable(true);
        result
    }
    pub fn attach(&self, session: &Rc<RefCell<DesignerSession>>) {
        *self.session.borrow_mut() = Some(Rc::downgrade(session));
    }
    pub fn on_commit(&self, callback: impl Fn(DesignGesture) + 'static) {
        *self.commit.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_selection_changed(&self, callback: impl Fn() + 'static) {
        *self.changed.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn set_zoom(&self, zoom: f64) {
        self.zoom
            .set(if zoom.is_finite() { zoom.max(0.1) } else { 1.0 });
    }
    pub fn set_grid(&self, size: f64) {
        self.grid
            .set(if size.is_finite() { size.max(0.0) } else { 0.0 });
    }
    pub fn set_interactive(&self, value: bool) {
        if let Some(session) = self.session() {
            session.borrow_mut().set_interactive(value);
        }
        self.interactive.set(value);
        if value {
            self.cursor_position.set(None);
            self.pointer_inside.set(false);
            self.clear_pending();
        }
        self.set_is_hit_test_visible(!value);
        self.invalidate_visual();
    }
    /// The local pointer in unsnapped artboard coordinates for collaboration presence.
    pub fn cursor_position(&self) -> Option<Point> {
        self.cursor_position.get()
    }
    fn track_cursor(&self, position: Point) {
        let inside = Rect::from_size(self.bounds().size()).contains(position);
        self.pointer_inside.set(inside);
        self.cursor_position.set(
            (!self.interactive.get()
                && position.x.is_finite()
                && position.y.is_finite()
                && (inside || self.gesture.borrow().is_some() || self.insert.borrow().is_some()))
            .then_some(position),
        );
    }
    pub fn arm_insert(&self, source: String) {
        self.clear_pending();
        *self.gesture_context.borrow_mut() = self
            .session()
            .map(|session| GestureContext::capture(&session.borrow()));
        *self.insert.borrow_mut() = Some(source);
    }
    pub fn begin_asset_drag(&self, source: String, event: &PointerPressedEventArgs) {
        self.arm_insert(source);
        event
            .pointer()
            .capture(Some(&self.to_ref().upcast::<InputElement>()));
        event.set_handled(true);
    }
    fn session(&self) -> Option<Rc<RefCell<DesignerSession>>> {
        self.session.borrow().as_ref().and_then(Weak::upgrade)
    }
    fn pending_is_current(&self) -> bool {
        self.session().is_some_and(|session| {
            self.gesture_context
                .borrow()
                .as_ref()
                .is_some_and(|context| context.matches(&session.borrow()))
        })
    }
    fn clear_pending(&self) {
        self.gesture.borrow_mut().take();
        self.insert.borrow_mut().take();
        self.gesture_context.borrow_mut().take();
        self.snap_guides.borrow_mut().clear();
    }
    fn snapped(&self, point: Point, modifiers: KeyModifiers) -> Point {
        let grid = self.grid.get();
        if grid > 0.0 && !modifiers.contains(KeyModifiers::ALT) {
            Point::new(
                (point.x / grid).round() * grid,
                (point.y / grid).round() * grid,
            )
        } else {
            point
        }
    }
    fn snap_gesture(&self, gesture: &Gesture, position: Point, modifiers: KeyModifiers) -> Point {
        self.snap_guides.borrow_mut().clear();
        if modifiers.contains(KeyModifiers::ALT) {
            return position;
        }
        if matches!(gesture.handle, Handle::Resize(_)) {
            return self.snapped(position, modifiers);
        }
        if !matches!(gesture.handle, Handle::Move) {
            return position;
        }
        let Some(bounds) = gesture.selection_bounds else {
            return position;
        };
        let delta = ferroui_designer_tools::Point::new(
            position.x - gesture.start.x,
            position.y - gesture.start.y,
        );
        let Ok(moving) = bounds.translated(delta) else {
            return position;
        };
        let Some(index) = &gesture.snap_index else {
            return position;
        };
        match index.snap_with(
            moving,
            (self.grid.get() > 0.0).then(|| self.grid.get()),
            6.0 / self.zoom.get(),
        ) {
            Ok(snapped) => {
                *self.snap_guides.borrow_mut() = snapped.guides;
                Point::new(position.x + snapped.delta.x, position.y + snapped.delta.y)
            }
            Err(_) => position,
        }
    }
    fn notify(&self) {
        if let Some(callback) = self.changed.borrow().as_ref().cloned() {
            callback();
        }
        self.invalidate_visual();
    }
    fn rectangles(&self) -> Vec<(NodeId, Rect)> {
        self.session()
            .map(|session| session.borrow().selection_bounds(&self.to_ref().upcast()))
            .unwrap_or_default()
    }
    fn pick(&self, position: Point) -> Option<NodeId> {
        let session = self.session()?;
        let session = session.borrow();
        let root = session.preview.root()?;
        let point = self.translate_point(position, &root)?;
        let mut hit = root.get_visual_at(point);
        while let Some(visual) = hit {
            if let Some(id) = session.preview.source_id(&visual) {
                return Some(id);
            }
            if std::ptr::eq::<Visual>(&*visual, &*root.clone().upcast::<Visual>()) {
                break;
            }
            hit = visual.visual_parent();
        }
        None
    }
    fn finish(&self, position: Point, modifiers: KeyModifiers) {
        if !self.pending_is_current() {
            self.clear_pending();
            self.notify();
            return;
        }
        let insertion = self.insert.borrow_mut().take();
        if let Some(source) = insertion {
            if Rect::from_size(self.bounds().size()).contains(position) {
                self.gesture_context.borrow_mut().take();
                if let Some(commit) = self.commit.borrow().as_ref().cloned() {
                    commit(DesignGesture::Insert {
                        source,
                        position: self.snapped(position, modifiers),
                    });
                }
            } else {
                *self.insert.borrow_mut() = Some(source);
            }
            self.notify();
            return;
        }
        self.gesture_context.borrow_mut().take();
        let Some(mut gesture) = self.gesture.borrow_mut().take() else {
            return;
        };
        let moved =
            (position.x - gesture.start.x).abs() + (position.y - gesture.start.y).abs() > 3.0;
        gesture.current = self.snap_gesture(&gesture, position, modifiers);
        let delta = gesture.current - gesture.start;
        let commit = match gesture.handle {
            Handle::Move if moved && delta.x.abs() + delta.y.abs() > 0.5 => {
                Some(DesignGesture::Move {
                    nodes: gesture.rectangles.iter().map(|(id, _)| *id).collect(),
                    delta: Point::new(delta.x, delta.y),
                })
            }
            Handle::Resize(handle) if moved => gesture.rectangles.first().and_then(|(node, _)| {
                gesture.geometry.map(|geometry| DesignGesture::ResizeLocal {
                    node: *node,
                    after: geometry.resize(gesture.start, gesture.current, handle),
                })
            }),
            Handle::Rotate if moved => gesture.rectangles.first().and_then(|(node, _)| {
                gesture.geometry.map(|geometry| {
                    let degrees = geometry.rotation(gesture.start, gesture.current);
                    DesignGesture::Rotate {
                        node: *node,
                        degrees: if modifiers.contains(KeyModifiers::SHIFT) {
                            (degrees / 15.0).round() * 15.0
                        } else {
                            degrees
                        },
                    }
                })
            }),
            Handle::Marquee => {
                if let Some(session) = self.session() {
                    let rect = marquee(gesture.start, gesture.current);
                    let mut session = session.borrow_mut();
                    let ids = session
                        .document()
                        .ast()
                        .elements()
                        .filter_map(|node| {
                            session
                                .preview
                                .bounds(node.id, &self.to_ref().upcast())
                                .filter(|bounds| rect.contains_rect(*bounds))
                                .map(|_| node.id)
                        })
                        .collect::<Vec<_>>();
                    session.select(ids);
                }
                None
            }
            _ => None,
        };
        if let (Some(command), Some(callback)) = (commit, self.commit.borrow().as_ref().cloned()) {
            callback(command);
        }
        self.snap_guides.borrow_mut().clear();
        self.notify();
    }
}
impl Default for DesignSurface {
    fn default() -> Self {
        Self {
            base: Control::construct(),
            session: RefCell::new(None),
            commit: RefCell::new(None),
            changed: RefCell::new(None),
            gesture: RefCell::new(None),
            insert: RefCell::new(None),
            grid: Cell::new(8.0),
            interactive: Cell::new(false),
            gesture_context: RefCell::new(None),
            releasing_capture: Cell::new(false),
            zoom: Cell::new(1.0),
            cursor_position: Cell::new(None),
            pointer_inside: Cell::new(false),
            snap_guides: RefCell::new(Vec::new()),
        }
    }
}

impl InputElementImpl for DesignSurface {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        this.track_cursor(event.get_position(Some(this)));
        if !event.properties().is_left_button_pressed {
            return;
        }
        let position = event.get_position(Some(this));
        let Some(session) = this.session() else {
            this.clear_pending();
            return;
        };
        if !session.borrow().can_manipulate() {
            this.clear_pending();
            return;
        }
        if this.insert.borrow().is_some() {
            // A palette or keyboard command may have armed insertion before this
            // press. Never rebind that intent to a newly opened document.
            if this.pending_is_current() {
                event
                    .pointer()
                    .capture(Some(&this.to_ref().upcast::<InputElement>()));
            } else {
                this.clear_pending();
            }
            event.set_handled(true);
            this.notify();
            Self::parent_on_pointer_pressed(this, event);
            return;
        }
        *this.gesture_context.borrow_mut() = Some(GestureContext::capture(&session.borrow()));
        session.borrow_mut().refresh_runtime_mapping();
        let rectangles = this.rectangles();
        let mut handle = None;
        let geometry = rectangles.first().and_then(|(node, _)| {
            let session = session.borrow();
            session.preview.visual(*node).and_then(|visual| {
                crate::affine_manipulation::Geometry::capture(&visual, &this.to_ref().upcast())
            })
        });
        if rectangles.len() == 1 {
            if let Some(geometry) = geometry {
                for (index, point) in geometry.handles().iter().enumerate() {
                    if (point.x - position.x).abs() <= 7.0 / this.zoom.get()
                        && (point.y - position.y).abs() <= 7.0 / this.zoom.get()
                    {
                        handle = Some(Handle::Resize(index as u8));
                        break;
                    }
                }
                let rotate = geometry.rotation_handle(this.zoom.get());
                if (rotate.x - position.x).abs() <= 7.0 / this.zoom.get()
                    && (rotate.y - position.y).abs() <= 7.0 / this.zoom.get()
                {
                    handle = Some(Handle::Rotate);
                }
            }
        }
        if handle.is_none() {
            if let Some(id) = this.pick(position).filter(|id| {
                *id != session.borrow().document().ast().root()
                    && !event.key_modifiers().contains(KeyModifiers::CONTROL)
            }) {
                let mut session = session.borrow_mut();
                let mut selected = session.selection().to_vec();
                if event.key_modifiers().contains(KeyModifiers::SHIFT) {
                    if let Some(index) = selected.iter().position(|selected| *selected == id) {
                        selected.remove(index);
                    } else {
                        selected.push(id);
                    }
                } else if !selected.contains(&id) {
                    selected = vec![id];
                }
                session.select(selected);
                handle = Some(Handle::Move);
            } else {
                session.borrow_mut().select([]);
                handle = Some(Handle::Marquee);
            }
        }
        let start = position;
        let selected = session.borrow().selection().to_vec();
        let snap_bounds: Vec<_> = {
            let session = session.borrow();
            let moving: std::collections::BTreeSet<_> = selected
                .iter()
                .flat_map(|node| session.document().ast().subtree(*node))
                .map(|node| node.id)
                .collect();
            session
                .document()
                .ast()
                .elements()
                .filter(|node| !moving.contains(&node.id))
                .filter_map(|node| session.preview.bounds(node.id, &this.to_ref().upcast()))
                .map(|r| ferroui_designer_tools::Rect::new(r.x, r.y, r.width, r.height))
                .collect()
        };
        let rectangles = this.rectangles();
        let selection_bounds = rectangles
            .iter()
            .map(|(_, r)| ferroui_designer_tools::Rect::new(r.x, r.y, r.width, r.height))
            .reduce(|a, b| a.union(b));
        let snap_index = ferroui_designer_tools::SnapSettings::default()
            .prepare(&snap_bounds)
            .ok();
        let geometry = rectangles
            .first()
            .and_then(|(node, _)| session.borrow().preview.visual(*node))
            .and_then(|visual| {
                crate::affine_manipulation::Geometry::capture(&visual, &this.to_ref().upcast())
            });
        *this.gesture.borrow_mut() = Some(Gesture {
            start,
            current: start,
            handle: handle.unwrap_or(Handle::Move),
            rectangles,
            selection_bounds,
            snap_index,
            geometry,
        });
        event
            .pointer()
            .capture(Some(&this.to_ref().upcast::<InputElement>()));
        event.set_handled(true);
        this.notify();
        Self::parent_on_pointer_pressed(this, event);
    }
    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
        this.track_cursor(event.get_position(Some(this)));
        let pending = this.gesture.borrow().is_some() || this.insert.borrow().is_some();
        if pending && !this.pending_is_current() {
            this.clear_pending();
            if event.pointer().captured().as_ref() == Some(&this.to_ref().upcast::<InputElement>())
            {
                event.pointer().capture(None);
            }
            this.notify();
            Self::parent_on_pointer_moved(this, event);
            return;
        }
        if let Some(gesture) = this.gesture.borrow_mut().as_mut() {
            gesture.current = this.snap_gesture(
                gesture,
                event.get_position(Some(this)),
                event.key_modifiers(),
            );
            this.invalidate_visual();
        }
        Self::parent_on_pointer_moved(this, event);
    }
    fn on_pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        if event.initial_press_mouse_button() != MouseButton::Left {
            Self::parent_on_pointer_released(this, event);
            return;
        }
        let position = event.get_position(Some(this));
        this.finish(position, event.key_modifiers());
        this.releasing_capture.set(true);
        event.pointer().capture(None);
        this.releasing_capture.set(false);
        this.track_cursor(position);
        event.set_handled(true);
        Self::parent_on_pointer_released(this, event);
    }
    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        this.gesture.borrow_mut().take();
        if !this.releasing_capture.get() {
            this.insert.borrow_mut().take();
        }
        if this.insert.borrow().is_none() {
            this.gesture_context.borrow_mut().take();
        }
        this.snap_guides.borrow_mut().clear();
        if !this.pointer_inside.get() {
            this.cursor_position.set(None);
        }
        this.invalidate_visual();
        Self::parent_on_pointer_capture_lost(this, event);
    }
    fn on_pointer_entered(this: &Self, event: &PointerEventArgs) {
        this.track_cursor(event.get_position(Some(this)));
        Self::parent_on_pointer_entered(this, event);
    }
    fn on_pointer_exited(this: &Self, event: &PointerEventArgs) {
        this.pointer_inside.set(false);
        if this.gesture.borrow().is_none() && this.insert.borrow().is_none() {
            this.cursor_position.set(None);
        }
        Self::parent_on_pointer_exited(this, event);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if event.key == Key::Escape {
            this.clear_pending();
            if !this.pointer_inside.get() {
                this.cursor_position.set(None);
            }
            this.invalidate_visual();
            event.set_handled(true);
        }
        Self::parent_on_key_down(this, event);
    }
}
impl VisualImpl for DesignSurface {
    fn render(this: &Self, context: &mut DrawingContext) {
        if this.interactive.get() {
            return;
        }
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        context.draw_rectangle(
            Some(&transparent),
            None,
            Rect::from_size(this.bounds().size()),
            0.0,
            0.0,
            &BoxShadows::default(),
        );
        if let Some(session) = this.session() {
            let session = session.borrow();
            for peer in session.current_presence(30_000) {
                let index = peer.actor.bytes().fold(0usize, |value, byte| {
                    value.wrapping_mul(31).wrapping_add(byte as usize)
                }) % 6;
                let color = [
                    0xff54c8ae, 0xffb89afa, 0xffffbe6e, 0xfff38dab, 0xff76c4f3, 0xffd2c768,
                ][index];
                let pen: Rc<dyn IPen> = Pen::from_uint32(color, 1.5).into();
                let brush: Rc<dyn IBrush> = SolidColorBrush::from_uint32(color).into();
                for shared in &peer.selection {
                    if let Some(rect) = session
                        .presence_source_node(shared)
                        .and_then(|node| session.preview.bounds(node, &this.to_ref().upcast()))
                    {
                        context.draw_rectangle(
                            None,
                            Some(&pen),
                            rect,
                            0.0,
                            0.0,
                            &BoxShadows::default(),
                        );
                    }
                }
                if let Some(cursor) = peer.cursor {
                    let p = Point::new(cursor.x, cursor.y);
                    context.draw_line(&pen, p, Point::new(p.x + 2.0, p.y + 16.0));
                    context.draw_line(&pen, p, Point::new(p.x + 12.0, p.y + 10.0));
                    context.draw_line(
                        &pen,
                        Point::new(p.x + 2.0, p.y + 16.0),
                        Point::new(p.x + 12.0, p.y + 10.0),
                    );
                    let name = peer.display_name.chars().take(24).collect::<String>();
                    let text = FormattedText::new(
                        &name,
                        CultureInfo::current_culture(),
                        FlowDirection::LeftToRight,
                        Typeface::default(),
                        11.0,
                        Some(Brushes::black()),
                    );
                    context.draw_rectangle(
                        Some(&brush),
                        None,
                        Rect::new(p.x + 12.0, p.y + 12.0, 160.0, 20.0),
                        3.0,
                        3.0,
                        &BoxShadows::default(),
                    );
                    context.draw_text(&text, Point::new(p.x + 17.0, p.y + 15.0));
                }
            }
        }
        let accent: Rc<dyn IPen> = Pen::from_uint32(0xff3099cc, 1.5).into();
        let fill: Rc<dyn IBrush> = Brushes::white();
        let gesture = this.gesture.borrow();
        let mut rectangles = this.rectangles();
        if let Some(gesture) = gesture.as_ref() {
            let delta = gesture.current - gesture.start;
            match gesture.handle {
                Handle::Move => {
                    for (_, rect) in &mut rectangles {
                        rect.x += delta.x;
                        rect.y += delta.y;
                    }
                }
                Handle::Resize(handle) => {
                    if let Some((_, rect)) = rectangles.first_mut() {
                        *rect = resize(*rect, delta.x, delta.y, handle);
                    }
                }
                Handle::Marquee => {
                    let rect = marquee(gesture.start, gesture.current);
                    context.draw_rectangle(
                        None,
                        Some(&accent),
                        rect,
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                }
                _ => {}
            }
        }
        let guide_pen: Rc<dyn IPen> = Pen::from_uint32(0xffd68af4, 1.0).into();
        for guide in this.snap_guides.borrow().iter() {
            match guide.axis {
                ferroui_designer_tools::Axis::Horizontal => context.draw_line(
                    &guide_pen,
                    Point::new(guide.position, 0.0),
                    Point::new(guide.position, this.bounds().height),
                ),
                ferroui_designer_tools::Axis::Vertical => context.draw_line(
                    &guide_pen,
                    Point::new(0.0, guide.position),
                    Point::new(this.bounds().width, guide.position),
                ),
            }
        }
        if rectangles.len() == 1 {
            let geometry = gesture
                .as_ref()
                .and_then(|gesture| gesture.geometry)
                .or_else(|| {
                    this.session().and_then(|session| {
                        let session = session.borrow();
                        session.preview.visual(rectangles[0].0).and_then(|visual| {
                            crate::affine_manipulation::Geometry::capture(
                                &visual,
                                &this.to_ref().upcast(),
                            )
                        })
                    })
                });
            if let Some(geometry) = geometry {
                let mut local = Rect::from_size(geometry.size);
                let mut transform = geometry.to_surface;
                if let Some(gesture) = gesture.as_ref() {
                    match gesture.handle {
                        Handle::Resize(handle) => {
                            local = geometry.resize(gesture.start, gesture.current, handle)
                        }
                        Handle::Move => {
                            let d = gesture.current - gesture.start;
                            transform *= ferroui_base::Matrix::create_translation(d.x, d.y);
                        }
                        Handle::Rotate => {
                            transform *= ferroui_base::Matrix::create_rotation_at(
                                geometry
                                    .rotation(gesture.start, gesture.current)
                                    .to_radians(),
                                geometry.pivot,
                            )
                        }
                        _ => {}
                    }
                }
                let points = crate::affine_manipulation::local_handles(local)
                    .map(|point| point.transform(transform));
                for (a, b) in [(0, 2), (2, 4), (4, 6), (6, 0)] {
                    context.draw_line(&accent, points[a], points[b]);
                }
                for point in points {
                    let radius = 3.5 / this.zoom.get();
                    context.draw_rectangle(
                        Some(&fill),
                        Some(&accent),
                        Rect::new(
                            point.x - radius,
                            point.y - radius,
                            radius * 2.0,
                            radius * 2.0,
                        ),
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                }
                let rotate = geometry.rotation_handle(this.zoom.get());
                context.draw_line(&accent, points[1], rotate);
                context.draw_ellipse_at(
                    Some(&fill),
                    Some(&accent),
                    rotate,
                    4.5 / this.zoom.get(),
                    4.5 / this.zoom.get(),
                );
                context.draw_ellipse_at(
                    None,
                    Some(&accent),
                    geometry.pivot,
                    3.0 / this.zoom.get(),
                    3.0 / this.zoom.get(),
                );
            }
        } else {
            for (_, rect) in &rectangles {
                context.draw_rectangle(
                    None,
                    Some(&accent),
                    *rect,
                    0.0,
                    0.0,
                    &BoxShadows::default(),
                );
            }
        }
    }
}
fn resize(before: Rect, dx: f64, dy: f64, handle: u8) -> Rect {
    let mut x = before.x;
    let mut y = before.y;
    let mut right = x + before.width;
    let mut bottom = y + before.height;
    if matches!(handle, 0 | 6 | 7) {
        x = (x + dx).min(right - 1.0);
    }
    if matches!(handle, 2..=4) {
        right = (right + dx).max(x + 1.0);
    }
    if matches!(handle, 0..=2) {
        y = (y + dy).min(bottom - 1.0);
    }
    if matches!(handle, 4..=6) {
        bottom = (bottom + dy).max(y + 1.0);
    }
    Rect::new(x, y, right - x, bottom - y)
}
fn marquee(a: Point, b: Point) -> Rect {
    Rect::new(
        a.x.min(b.x),
        a.y.min(b.y),
        (a.x - b.x).abs(),
        (a.y - b.y).abs(),
    )
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
