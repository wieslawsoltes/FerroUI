use crate::DesignerSession;
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyModifiers,
    PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
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
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect,
    Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_designer_core::NodeId;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

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
    snap_bounds: Vec<ferroui_designer_tools::Rect>,
}

/// Transparent native adorner for selecting, moving, resizing and rotating live controls.
/// Motion changes only the adorner; release commits one semantic transaction. Escape and
/// capture loss cancel the gesture, so neither source nor history receive partial drags.
#[repr(C)]
pub struct DesignSurface {
    base: Control,
    session: RefCell<Option<Weak<RefCell<DesignerSession>>>>,
    commit: RefCell<Option<Rc<dyn Fn(DesignGesture)>>>,
    changed: RefCell<Option<Rc<dyn Fn()>>>,
    gesture: RefCell<Option<Gesture>>,
    insert: RefCell<Option<String>>,
    grid: Cell<f64>,
    interactive: Cell<bool>,
    gesture_revision: Cell<u64>,
    releasing_capture: Cell<bool>,
    zoom: Cell<f64>,
    snap_guides: RefCell<Vec<ferroui_designer_tools::SnapGuide>>,
}
ferro_class!(DesignSurface: Control);
ferro_class_info!(DesignSurface {
    new: DesignSurface::new
});
ferro_impl_classes!(DesignSurface: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, ControlImpl);

impl DesignSurface {
    pub fn new() -> Ref<Self> {
        let result = instantiate(Self {
            base: Control::construct(),
            session: RefCell::new(None),
            commit: RefCell::new(None),
            changed: RefCell::new(None),
            gesture: RefCell::new(None),
            insert: RefCell::new(None),
            grid: Cell::new(8.0),
            interactive: Cell::new(false),
            gesture_revision: Cell::new(0),
            releasing_capture: Cell::new(false),
            zoom: Cell::new(1.0),
            snap_guides: RefCell::new(Vec::new()),
        });
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
        self.set_is_hit_test_visible(!value);
        self.invalidate_visual();
    }
    pub fn arm_insert(&self, source: String) {
        if let Some(session) = self.session() {
            self.gesture_revision
                .set(session.borrow().document().revision());
        }
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
        let Some(bounds) = gesture
            .rectangles
            .iter()
            .map(|(_, r)| ferroui_designer_tools::Rect::new(r.x, r.y, r.width, r.height))
            .reduce(|a, b| a.union(b))
        else {
            return position;
        };
        let delta = ferroui_designer_tools::Point::new(
            position.x - gesture.start.x,
            position.y - gesture.start.y,
        );
        let Ok(moving) = bounds.translated(delta) else {
            return position;
        };
        let settings = ferroui_designer_tools::SnapSettings {
            grid: (self.grid.get() > 0.0).then(|| self.grid.get()),
            tolerance: 6.0 / self.zoom.get(),
            ..Default::default()
        };
        match settings.snap(moving, &gesture.snap_bounds) {
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
        if self.session().is_none_or(|session| {
            let session = session.borrow();
            !session.can_manipulate()
                || session.document().revision() != self.gesture_revision.get()
        }) {
            self.gesture.borrow_mut().take();
            self.insert.borrow_mut().take();
            self.notify();
            return;
        }
        let insertion = self.insert.borrow_mut().take();
        if let Some(source) = insertion {
            if Rect::from_size(self.bounds().size()).contains(position) {
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
            Handle::Resize(handle) if moved => {
                gesture
                    .rectangles
                    .first()
                    .map(|(node, before)| DesignGesture::Resize {
                        node: *node,
                        before: *before,
                        after: resize(*before, delta.x, delta.y, handle),
                    })
            }
            Handle::Rotate if moved => gesture.rectangles.first().map(|(node, rect)| {
                let center = Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0);
                let from = (gesture.start.y - center.y).atan2(gesture.start.x - center.x);
                let to = (gesture.current.y - center.y).atan2(gesture.current.x - center.x);
                DesignGesture::Rotate {
                    node: *node,
                    degrees: if modifiers.contains(KeyModifiers::SHIFT) {
                        ((to - from).to_degrees() / 15.0).round() * 15.0
                    } else {
                        (to - from).to_degrees()
                    },
                }
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
            gesture_revision: Cell::new(0),
            releasing_capture: Cell::new(false),
            zoom: Cell::new(1.0),
            snap_guides: RefCell::new(Vec::new()),
        }
    }
}

impl InputElementImpl for DesignSurface {
    fn on_pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !event.properties().is_left_button_pressed {
            return;
        }
        let position = event.get_position(Some(this));
        let Some(session) = this.session() else {
            return;
        };
        if !session.borrow().can_manipulate() {
            return;
        }
        this.gesture_revision
            .set(session.borrow().document().revision());
        session.borrow_mut().refresh_runtime_mapping();
        let rectangles = this.rectangles();
        let mut handle = None;
        if rectangles.len() == 1 {
            let rect = rectangles[0].1;
            for (index, point) in handles(rect).iter().enumerate() {
                if (point.x - position.x).abs() <= 7.0 && (point.y - position.y).abs() <= 7.0 {
                    handle = Some(Handle::Resize(index as u8));
                    break;
                }
            }
            let rotate = Point::new(rect.x + rect.width / 2.0, rect.y - 24.0);
            if (rotate.x - position.x).abs() <= 7.0 && (rotate.y - position.y).abs() <= 7.0 {
                handle = Some(Handle::Rotate);
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
        let snap_bounds = {
            let session = session.borrow();
            session
                .document()
                .ast()
                .elements()
                .filter(|node| {
                    let mut ancestor = Some(node.id);
                    while let Some(id) = ancestor {
                        if selected.contains(&id) {
                            return false;
                        }
                        ancestor = session
                            .document()
                            .ast()
                            .node(id)
                            .and_then(|node| node.parent);
                    }
                    true
                })
                .filter_map(|node| session.preview.bounds(node.id, &this.to_ref().upcast()))
                .map(|r| ferroui_designer_tools::Rect::new(r.x, r.y, r.width, r.height))
                .collect()
        };
        *this.gesture.borrow_mut() = Some(Gesture {
            start,
            current: start,
            handle: handle.unwrap_or(Handle::Move),
            rectangles: this.rectangles(),
            snap_bounds,
        });
        event
            .pointer()
            .capture(Some(&this.to_ref().upcast::<InputElement>()));
        event.set_handled(true);
        this.notify();
        Self::parent_on_pointer_pressed(this, event);
    }
    fn on_pointer_moved(this: &Self, event: &PointerEventArgs) {
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
        this.finish(event.get_position(Some(this)), event.key_modifiers());
        this.releasing_capture.set(true);
        event.pointer().capture(None);
        this.releasing_capture.set(false);
        event.set_handled(true);
        Self::parent_on_pointer_released(this, event);
    }
    fn on_pointer_capture_lost(this: &Self, event: &PointerCaptureLostEventArgs) {
        this.gesture.borrow_mut().take();
        if !this.releasing_capture.get() {
            this.insert.borrow_mut().take();
        }
        this.snap_guides.borrow_mut().clear();
        this.invalidate_visual();
        Self::parent_on_pointer_capture_lost(this, event);
    }
    fn on_key_down(this: &Self, event: &KeyEventArgs) {
        if event.key == Key::Escape {
            this.gesture.borrow_mut().take();
            this.insert.borrow_mut().take();
            this.snap_guides.borrow_mut().clear();
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
        let accent: Rc<dyn IPen> = Pen::from_uint32(0xff6d91ff, 1.5).into();
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
        for (_, rect) in &rectangles {
            context.draw_rectangle(None, Some(&accent), *rect, 0.0, 0.0, &BoxShadows::default());
            if rectangles.len() == 1 {
                for point in handles(*rect) {
                    context.draw_rectangle(
                        Some(&fill),
                        Some(&accent),
                        Rect::new(point.x - 3.5, point.y - 3.5, 7.0, 7.0),
                        0.0,
                        0.0,
                        &BoxShadows::default(),
                    );
                }
                let top = Point::new(rect.x + rect.width / 2.0, rect.y);
                let rotate = Point::new(top.x, top.y - 24.0);
                context.draw_line(&accent, top, rotate);
                context.draw_ellipse_at(Some(&fill), Some(&accent), rotate, 4.5, 4.5);
            }
        }
    }
}
fn handles(r: Rect) -> [Point; 8] {
    [
        Point::new(r.x, r.y),
        Point::new(r.x + r.width / 2.0, r.y),
        Point::new(r.x + r.width, r.y),
        Point::new(r.x + r.width, r.y + r.height / 2.0),
        Point::new(r.x + r.width, r.y + r.height),
        Point::new(r.x + r.width / 2.0, r.y + r.height),
        Point::new(r.x, r.y + r.height),
        Point::new(r.x, r.y + r.height / 2.0),
    ]
}
fn resize(before: Rect, dx: f64, dy: f64, handle: u8) -> Rect {
    let mut x = before.x;
    let mut y = before.y;
    let mut right = x + before.width;
    let mut bottom = y + before.height;
    if matches!(handle, 0 | 6 | 7) {
        x = (x + dx).min(right - 1.0);
    }
    if matches!(handle, 2 | 3 | 4) {
        right = (right + dx).max(x + 1.0);
    }
    if matches!(handle, 0 | 1 | 2) {
        y = (y + dy).min(bottom - 1.0);
    }
    if matches!(handle, 4 | 5 | 6) {
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
