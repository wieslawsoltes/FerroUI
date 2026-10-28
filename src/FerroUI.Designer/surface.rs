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
    /// A validated placement or layout-handle draft bound to its original document snapshot.
    LayoutEdit {
        preview: Rc<crate::LayoutEditPreview>,
    },
}

#[derive(Clone, Copy, Debug)]
enum Handle {
    Move,
    Selection,
    Resize(u8),
    Rotate,
    Marquee,
    Layout(usize),
}
struct Gesture {
    start: Point,
    current: Point,
    handle: Handle,
    rectangles: Vec<(NodeId, Rect)>,
    selection_bounds: Option<ferroui_designer_tools::Rect>,
    snap_index: Option<ferroui_designer_tools::SnapIndex>,
    guide_snap_index: Option<ferroui_designer_tools::SnapIndex>,
    geometry: Option<crate::affine_manipulation::Geometry>,
    initial_selection: Vec<NodeId>,
    selection_modifiers: KeyModifiers,
    helper_origin: Option<crate::LayoutHelperSnapshot>,
    peers: Vec<Rect>,
    ghosts: Vec<crate::DragVisualGhost>,
    ghost_capture_attempted: bool,
}

type CommitGesture = Rc<dyn Fn(DesignGesture)>;

/// Retaining the syntax allocation distinguishes document instances even when a
/// host replaces a session with the same URI, revision, and local node IDs.
struct GestureContext {
    uri: String,
    revision: u64,
    syntax: Arc<Ast>,
    providers: u64,
    layer_epoch: u64,
    scope_epoch: u64,
    selection: Vec<NodeId>,
    animation: (Option<NodeId>, f64, bool, bool),
}
impl GestureContext {
    fn capture(session: &DesignerSession) -> Self {
        Self {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            syntax: session.document().ast_snapshot(),
            providers: session.layout_tools().revision(),
            layer_epoch: session.layer_view_epoch(),
            scope_epoch: session.definition_scope_epoch(),
            selection: session.selection().to_vec(),
            animation: (
                session.active_animation_program(),
                session.time(),
                session.is_playing(),
                session.recording(),
            ),
        }
    }
    fn matches(&self, session: &DesignerSession) -> bool {
        session.can_manipulate()
            && session.document_uri() == self.uri
            && session.document().revision() == self.revision
            && std::ptr::eq(session.document().ast(), self.syntax.as_ref())
            && session.layout_tools().revision() == self.providers
            && session.layer_view_epoch() == self.layer_epoch
            && session.definition_scope_epoch() == self.scope_epoch
            && session.selection() == self.selection
            && self.animation
                == (
                    session.active_animation_program(),
                    session.time(),
                    session.is_playing(),
                    session.recording(),
                )
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
    canvas_guides: RefCell<crate::CanvasGuideSettings>,
    interactive: Cell<bool>,
    gesture_context: RefCell<Option<GestureContext>>,
    releasing_capture: Cell<bool>,
    zoom: Cell<f64>,
    snap_guides: RefCell<Vec<ferroui_designer_tools::SnapGuide>>,
    cursor_position: Cell<Option<Point>>,
    pointer_inside: Cell<bool>,
    helpers_visible: Cell<bool>,
    focus_order_visible: Cell<bool>,
    helper_epoch: Cell<u64>,
    helper_cache: RefCell<Option<helpers::HelperCache>>,
    helper_preview: RefCell<Option<Rc<crate::LayoutEditPreview>>>,
    helper_error: RefCell<Option<String>>,
    helper_focus: Cell<Option<usize>>,
    helper_hover: Cell<Option<usize>>,
    helper_subscription: RefCell<Option<Rc<dyn ferroui_base::reactive::IDisposable>>>,
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
        let weak = result.downgrade();
        *result.helper_subscription.borrow_mut() = Some(result.layout_updated(move || {
            if let Some(surface) = weak.upgrade() {
                surface
                    .helper_epoch
                    .set(surface.helper_epoch.get().wrapping_add(1));
                surface.invalidate_visual();
            }
        }));
        ferroui_controls::automation::AutomationProperties::set_name(
            &result,
            Some("Design canvas. F8 selects layout handles; arrow keys adjust; Escape cancels."),
        );
        result
    }
    pub fn attach(&self, session: &Rc<RefCell<DesignerSession>>) {
        *self.session.borrow_mut() = Some(Rc::downgrade(session));
        self.invalidate_layout_helpers();
    }
    pub fn on_commit(&self, callback: impl Fn(DesignGesture) + 'static) {
        *self.commit.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn on_selection_changed(&self, callback: impl Fn() + 'static) {
        *self.changed.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn set_zoom(&self, zoom: f64) {
        self.zoom.set(if zoom.is_finite() {
            // Independent hosts may provide a camera beyond the workspace's UI limits.
            // Handle sizes and snap tolerances must use their actual applied scale.
            zoom.max(crate::CanvasViewport::MIN_ZOOM)
        } else {
            1.0
        });
    }
    /// Whether a source-editing gesture or insertion currently owns canvas input.
    pub(crate) fn has_pending_interaction(&self) -> bool {
        self.gesture.borrow().is_some() || self.insert.borrow().is_some()
    }

    /// A camera change cancels pending authored geometry rather than committing stale coordinates.
    pub(crate) fn cancel_for_navigation(&self) {
        let pending = self.has_pending_interaction();
        if pending {
            self.clear_pending();
            self.notify();
        }
        self.invalidate_visual();
    }
    /// Legacy standalone-host override. Any finite nonnegative spacing remains supported;
    /// applying bounded CanvasGuideSettings later replaces this override.
    pub fn set_grid(&self, size: f64) {
        self.clear_pending();
        self.grid
            .set(if size.is_finite() { size.max(0.0) } else { 0.0 });
    }
    /// Shared local snapping contract for standalone hosts and DesignerWorkspace.
    /// Updating it retires pending gestures before their accepted snap index changes.
    pub fn set_canvas_guides(&self, settings: crate::CanvasGuideSettings) -> Result<(), String> {
        settings.validate()?;
        let grid = if settings.snap_to_grid {
            settings.grid_spacing
        } else {
            0.0
        };
        if *self.canvas_guides.borrow() != settings || self.grid.get() != grid {
            self.clear_pending();
            self.grid.set(if settings.snap_to_grid {
                settings.grid_spacing
            } else {
                0.0
            });
            *self.canvas_guides.borrow_mut() = settings;
            self.invalidate_visual();
        }
        Ok(())
    }
    pub fn canvas_guides(&self) -> crate::CanvasGuideSettings {
        self.canvas_guides.borrow().clone()
    }
    pub fn focus_order_visible(&self) -> bool {
        self.focus_order_visible.get()
    }
    /// Shows native keyboard-navigation diagnostics without changing source or history.
    pub fn set_focus_order_visible(&self, visible: bool) {
        self.focus_order_visible.set(visible);
        self.invalidate_visual();
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
        self.cancel_gesture();
        self.insert.borrow_mut().take();
        self.gesture_context.borrow_mut().take();
        self.snap_guides.borrow_mut().clear();
        self.helper_preview.borrow_mut().take();
        self.helper_error.borrow_mut().take();
    }
    fn cancel_gesture(&self) {
        let gesture = self.gesture.borrow_mut().take();
        if let Some(gesture) = gesture.filter(|gesture| matches!(gesture.handle, Handle::Marquee)) {
            if self.pending_is_current() {
                if let Some(session) = self.session() {
                    session.borrow_mut().select(gesture.initial_selection);
                }
            }
        }
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
        let (position, axis) =
            if matches!(gesture.handle, Handle::Move) && modifiers.contains(KeyModifiers::SHIFT) {
                if (position.x - gesture.start.x).abs() >= (position.y - gesture.start.y).abs() {
                    (
                        Point::new(position.x, gesture.start.y),
                        Some(ferroui_designer_tools::Axis::Horizontal),
                    )
                } else {
                    (
                        Point::new(gesture.start.x, position.y),
                        Some(ferroui_designer_tools::Axis::Vertical),
                    )
                }
            } else {
                (position, None)
            };
        if modifiers.contains(KeyModifiers::ALT) {
            return position;
        }
        if let Handle::Resize(handle) = gesture.handle {
            let side = match handle {
                1 | 5 => Some(ferroui_designer_tools::Axis::Vertical),
                3 | 7 => Some(ferroui_designer_tools::Axis::Horizontal),
                _ => None,
            };
            // A rotated/skewed side handle moves on an oblique local axis. Preserve its
            // existing grid behavior; artboard-axis guide corrections cannot constrain it.
            let aligned = gesture.geometry.is_some_and(|geometry| {
                geometry.to_surface.m12.abs() < 1e-10 && geometry.to_surface.m21.abs() < 1e-10
            });
            if side.is_none() || aligned {
                if let Some(index) = &gesture.guide_snap_index {
                    if let Ok(snapped) = index.snap_with(
                        ferroui_designer_tools::Rect::new(position.x, position.y, 0.0, 0.0),
                        None,
                        self.canvas_guides.borrow().snap_tolerance / self.zoom.get(),
                    ) {
                        let guides: Vec<_> = snapped
                            .guides
                            .into_iter()
                            .filter(|guide| side.is_none_or(|axis| guide.axis == axis))
                            .collect();
                        if !guides.is_empty() {
                            *self.snap_guides.borrow_mut() = guides;
                            return Point::new(
                                position.x
                                    + if side == Some(ferroui_designer_tools::Axis::Vertical) {
                                        0.0
                                    } else {
                                        snapped.delta.x
                                    },
                                position.y
                                    + if side == Some(ferroui_designer_tools::Axis::Horizontal) {
                                        0.0
                                    } else {
                                        snapped.delta.y
                                    },
                            );
                        }
                    }
                }
            }
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
            self.canvas_guides.borrow().snap_tolerance / self.zoom.get(),
        ) {
            Ok(snapped) => {
                *self.snap_guides.borrow_mut() = snapped
                    .guides
                    .into_iter()
                    .filter(|guide| axis.is_none_or(|axis| guide.axis == axis))
                    .collect();
                Point::new(
                    position.x
                        + if axis == Some(ferroui_designer_tools::Axis::Vertical) {
                            0.0
                        } else {
                            snapped.delta.x
                        },
                    position.y
                        + if axis == Some(ferroui_designer_tools::Axis::Horizontal) {
                            0.0
                        } else {
                            snapped.delta.y
                        },
                )
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
            .map(|session| {
                let session = session.borrow();
                session
                    .selection_bounds(&self.to_ref().upcast())
                    .into_iter()
                    .filter(|(node, _)| session.can_canvas_select(*node))
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Authored controls beneath an artboard point in native front-to-back paint order.
    /// Generated template visuals map back to their authored owner; duplicates, locked,
    /// hidden and out-of-scope layers are omitted. Hosts may use this for a Select Layer menu.
    pub fn hit_stack(&self, position: Point) -> Vec<NodeId> {
        let Some(session) = self.session() else {
            return Vec::new();
        };
        let session = session.borrow();
        let Some(root) = session.preview.root() else {
            return Vec::new();
        };
        let Some(point) = self.translate_point(position, &root) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for visual in root.get_visuals_at(point) {
            let mut current = Some(visual);
            while let Some(visual) = current {
                if let Some(id) = session.preview.source_id(&visual) {
                    if session.can_canvas_select(id) && seen.insert(id) {
                        result.push(id);
                    }
                    break;
                }
                current = visual.visual_parent();
            }
        }
        result
    }
    fn pick(&self, position: Point) -> Option<NodeId> {
        self.hit_stack(position).into_iter().next()
    }
    fn finish(&self, position: Point, modifiers: KeyModifiers) {
        if !self.pending_is_current() {
            self.clear_pending();
            self.notify();
            return;
        }
        self.update_layout_preview(position, modifiers);
        let insertion = self.insert.borrow_mut().take();
        if let Some(source) = insertion {
            if Rect::from_size(self.bounds().size()).contains(position) {
                self.gesture_context.borrow_mut().take();
                if let Some(commit) = self.commit.borrow().as_ref().cloned() {
                    if self.helpers_visible.get() && !modifiers.contains(KeyModifiers::ALT) {
                        if let Some(preview) = self
                            .helper_preview
                            .borrow()
                            .as_ref()
                            .cloned()
                            .filter(|preview| preview.is_accepted())
                        {
                            commit(DesignGesture::LayoutEdit { preview });
                        }
                    } else {
                        commit(DesignGesture::Insert {
                            source,
                            position: self.snapped(position, modifiers),
                        });
                    }
                }
                self.helper_preview.borrow_mut().take();
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
        let moved = ((position.x - gesture.start.x).abs() + (position.y - gesture.start.y).abs())
            * self.zoom.get()
            > 3.0;
        gesture.current = self.snap_gesture(&gesture, position, modifiers);
        let delta = gesture.current - gesture.start;
        let commit = match gesture.handle {
            Handle::Move if moved && delta.x.abs() + delta.y.abs() > 0.5 => {
                if let Some(preview) = self.helper_preview.borrow().as_ref().cloned() {
                    preview
                        .is_accepted()
                        .then_some(DesignGesture::LayoutEdit { preview })
                } else if self.helper_error.borrow().is_some() {
                    None
                } else {
                    Some(DesignGesture::Move {
                        nodes: gesture.rectangles.iter().map(|(id, _)| *id).collect(),
                        delta: Point::new(delta.x, delta.y),
                    })
                }
            }
            Handle::Layout(_) if moved => self
                .helper_preview
                .borrow()
                .as_ref()
                .cloned()
                .filter(|preview| preview.is_accepted())
                .map(|preview| DesignGesture::LayoutEdit { preview }),
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
                    let crossing = gesture.current.x < gesture.start.x;
                    let root = session.document().ast().root();
                    let ids = session
                        .document()
                        .ast()
                        .elements()
                        .filter(|node| node.id != root)
                        .filter(|node| session.can_canvas_select(node.id))
                        .filter_map(|node| {
                            session
                                .preview
                                .bounds(node.id, &self.to_ref().upcast())
                                .filter(|bounds| {
                                    if crossing {
                                        rect.intersects(*bounds)
                                    } else {
                                        rect.contains_rect(*bounds)
                                    }
                                })
                                .map(|_| node.id)
                        })
                        .collect::<Vec<_>>();
                    session.select(marquee_selection(
                        &gesture.initial_selection,
                        ids,
                        gesture.selection_modifiers,
                    ));
                }
                None
            }
            _ => None,
        };
        if let (Some(command), Some(callback)) = (commit, self.commit.borrow().as_ref().cloned()) {
            callback(command);
        }
        self.snap_guides.borrow_mut().clear();
        self.helper_preview.borrow_mut().take();
        self.helper_error.borrow_mut().take();
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
            canvas_guides: RefCell::new(crate::CanvasGuideSettings::default()),
            interactive: Cell::new(false),
            gesture_context: RefCell::new(None),
            releasing_capture: Cell::new(false),
            zoom: Cell::new(1.0),
            cursor_position: Cell::new(None),
            pointer_inside: Cell::new(false),
            snap_guides: RefCell::new(Vec::new()),
            helpers_visible: Cell::new(true),
            focus_order_visible: Cell::new(false),
            helper_epoch: Cell::new(0),
            helper_cache: RefCell::new(None),
            helper_preview: RefCell::new(None),
            helper_error: RefCell::new(None),
            helper_focus: Cell::new(None),
            helper_hover: Cell::new(None),
            helper_subscription: RefCell::new(None),
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
        this.focus();
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
        session.borrow_mut().refresh_runtime_mapping();
        // Ctrl+Alt-click cycles overlapping authored controls without arming a drag.
        // Ctrl-drag keeps its existing subtract-marquee meaning.
        if event
            .key_modifiers()
            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            this.clear_pending();
            let root = session.borrow().document().ast().root();
            let hits = this
                .hit_stack(position)
                .into_iter()
                .filter(|node| *node != root)
                .collect::<Vec<_>>();
            if !hits.is_empty() {
                let current = session.borrow().selection().first().copied();
                let next = current
                    .and_then(|current| hits.iter().position(|node| *node == current))
                    .map_or(0, |index| (index + 1) % hits.len());
                session.borrow_mut().select([hits[next]]);
            }
            event.set_handled(true);
            this.notify();
            return;
        }
        let initial_selection = session.borrow().selection().to_vec();
        let rectangles = this.rectangles();
        let mut handle = None;
        let mut helper_origin = None;
        this.helper_focus.set(None);
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
        if handle.is_none() && !event.key_modifiers().contains(KeyModifiers::ALT) {
            if let Some(snapshot) = this.layout_helper_snapshot() {
                if let Some(index) =
                    crate::hit_test_layout_handle(&snapshot, position, this.zoom.get())
                {
                    handle = Some(Handle::Layout(index));
                    helper_origin = Some(snapshot);
                    this.helper_focus.set(Some(index));
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
                        handle = Some(Handle::Selection);
                    } else {
                        selected.push(id);
                    }
                } else if !selected.contains(&id) {
                    selected = vec![id];
                }
                session.select(selected);
                handle.get_or_insert(Handle::Move);
            } else {
                if !event
                    .key_modifiers()
                    .intersects(KeyModifiers::SHIFT | KeyModifiers::CONTROL)
                {
                    session.borrow_mut().select([]);
                }
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
                .filter(|node| {
                    let state = session.layer_state(node.id);
                    !state.is_hidden() && !state.outside_isolation
                })
                .filter_map(|node| session.preview.bounds(node.id, &this.to_ref().upcast()))
                .map(|r| ferroui_designer_tools::Rect::new(r.x, r.y, r.width, r.height))
                .collect()
        };
        let rectangles = this.rectangles();
        let selection_bounds = rectangles
            .iter()
            .map(|(_, r)| ferroui_designer_tools::Rect::new(r.x, r.y, r.width, r.height))
            .reduce(|a, b| a.union(b));
        let snap_index = {
            let settings = this.canvas_guides.borrow();
            settings
                .snap_settings(this.zoom.get())
                .prepare(if settings.snap_to_objects {
                    &snap_bounds
                } else {
                    &[]
                })
                .ok()
        };
        let guide_snap_index = this
            .canvas_guides
            .borrow()
            .snap_settings(this.zoom.get())
            .prepare(&[])
            .ok();
        let geometry = rectangles
            .first()
            .and_then(|(node, _)| session.borrow().preview.visual(*node))
            .and_then(|visual| {
                crate::affine_manipulation::Geometry::capture(&visual, &this.to_ref().upcast())
            });
        // Bind after the press has selected its target. Later selection or animation
        // changes cancel the gesture; an unchanged source-buffer refresh remains safe.
        *this.gesture_context.borrow_mut() = Some(GestureContext::capture(&session.borrow()));
        *this.gesture.borrow_mut() = Some(Gesture {
            start,
            current: start,
            handle: handle.unwrap_or(Handle::Move),
            rectangles,
            selection_bounds,
            snap_index,
            guide_snap_index,
            geometry,
            initial_selection,
            selection_modifiers: event.key_modifiers(),
            helper_origin,
            peers: snap_bounds
                .iter()
                .map(|r| Rect::new(r.x, r.y, r.width, r.height))
                .collect(),
            ghosts: Vec::new(),
            ghost_capture_attempted: false,
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
        this.capture_drag_visuals();
        this.update_layout_preview(event.get_position(Some(this)), event.key_modifiers());
        this.update_helper_hover(event.get_position(Some(this)));
        if this.insert.borrow().is_some() {
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
        this.cancel_gesture();
        if !this.releasing_capture.get() {
            this.insert.borrow_mut().take();
        }
        if this.insert.borrow().is_none() {
            this.gesture_context.borrow_mut().take();
        }
        this.snap_guides.borrow_mut().clear();
        this.helper_preview.borrow_mut().take();
        this.helper_error.borrow_mut().take();
        if !this.pointer_inside.get() {
            this.cursor_position.set(None);
        }
        this.notify();
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
        if this.handle_layout_key(event) {
            event.set_handled(true);
            return;
        }
        if event.key == Key::Escape {
            let had_pending = this.gesture.borrow().is_some() || this.insert.borrow().is_some();
            this.clear_pending();
            this.helper_focus.set(None);
            if !had_pending {
                if let Some(session) = this.session() {
                    session.borrow_mut().clear_layer_isolation();
                }
            }
            if !this.pointer_inside.get() {
                this.cursor_position.set(None);
            }
            this.notify();
            event.set_handled(true);
        } else if !this.interactive.get()
            && this.gesture.borrow().is_none()
            && this.insert.borrow().is_none()
            && !event.key_modifiers.contains(KeyModifiers::ALT)
        {
            if !event
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
            {
                let command = match (event.key, event.key_modifiers.contains(KeyModifiers::SHIFT)) {
                    (Key::Enter, false) => Some(crate::LayerCommand::SelectChildren),
                    (Key::Enter, true) => Some(crate::LayerCommand::SelectParent),
                    (Key::Tab, false) => Some(crate::LayerCommand::SelectNextSibling),
                    (Key::Tab, true) => Some(crate::LayerCommand::SelectPreviousSibling),
                    _ => None,
                };
                if let Some(command) = command {
                    if let Some(session) = this.session() {
                        let _ = session.borrow_mut().apply_layer_command(command);
                    }
                    this.notify();
                    event.set_handled(true);
                    return;
                }
            }
            let step = if event.key_modifiers.contains(KeyModifiers::SHIFT) {
                this.canvas_guides.borrow().large_nudge
            } else if event
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
            {
                this.canvas_guides.borrow().small_nudge / 10.0
            } else {
                this.canvas_guides.borrow().small_nudge
            };
            let delta = match event.key {
                Key::Left => Some(Point::new(-step, 0.0)),
                Key::Right => Some(Point::new(step, 0.0)),
                Key::Up => Some(Point::new(0.0, -step)),
                Key::Down => Some(Point::new(0.0, step)),
                _ => None,
            };
            if let Some(delta) = delta {
                let nodes = this.session().and_then(|session| {
                    let session = session.borrow();
                    (session.can_manipulate()
                        && !session.selection().is_empty()
                        && session
                            .selection()
                            .iter()
                            .all(|id| session.can_canvas_select(*id)))
                    .then(|| session.selection().to_vec())
                });
                if let Some(nodes) = nodes {
                    if let Some(commit) = this.commit.borrow().as_ref().cloned() {
                        commit(DesignGesture::Move { nodes, delta });
                        this.notify();
                        event.set_handled(true);
                    }
                }
            }
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
        this.render_drag_visuals(context);
        this.render_layout_feedback(context);
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
                        *rect = gesture.geometry.map_or_else(
                            || resize(*rect, delta.x, delta.y, handle),
                            |geometry| {
                                geometry
                                    .resize(gesture.start, gesture.current, handle)
                                    .transform_to_aabb(geometry.to_surface)
                            },
                        );
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
        // Structural layouts describe insertion, cells and gaps themselves. Distances
        // from a freely translated ghost to unrelated peers would be misleading here.
        let structural_placement = this
            .helper_preview
            .borrow()
            .as_ref()
            .is_some_and(|preview| preview.helpers.scene.placement.is_some());
        if let Some(gesture) = gesture.as_ref().filter(|g| {
            !structural_placement && matches!(g.handle, Handle::Move | Handle::Resize(_))
        }) {
            if let Some(bounds) = rectangles.iter().map(|(_, r)| *r).reduce(|a, b| a.union(b)) {
                crate::render_manipulation_metrics(
                    context,
                    bounds,
                    &gesture.peers,
                    &this.snap_guides.borrow(),
                    this.zoom.get(),
                );
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
                // Rotation affordances belong to the idle selection or a rotation
                // gesture. Keeping their original anchors during a move would draw
                // a long stem back to the accepted object's position.
                if gesture
                    .as_ref()
                    .is_none_or(|g| matches!(g.handle, Handle::Rotate))
                {
                    let mut rotate = geometry.rotation_handle(this.zoom.get());
                    if let Some(gesture) = gesture.as_ref() {
                        rotate = rotate.transform(ferroui_base::Matrix::create_rotation_at(
                            geometry
                                .rotation(gesture.start, gesture.current)
                                .to_radians(),
                            geometry.pivot,
                        ));
                    }
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
        if this.focus_order_visible.get() {
            if let Some(session) = this.session() {
                crate::focus_order::render_focus_order(
                    context,
                    &session.borrow(),
                    &this.to_ref().upcast::<Visual>(),
                    this.zoom.get(),
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

fn marquee_selection(
    initial: &[NodeId],
    hits: Vec<NodeId>,
    modifiers: KeyModifiers,
) -> Vec<NodeId> {
    let control = modifiers.contains(KeyModifiers::CONTROL);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    if !control && !shift {
        return hits;
    }
    let hit_set = hits
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let initial_set = initial
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut result = initial
        .iter()
        .copied()
        .filter(|id| !control || !hit_set.contains(id))
        .collect::<Vec<_>>();
    if !control || !shift {
        result.extend(hits.into_iter().filter(|id| !initial_set.contains(id)));
    }
    result
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;

#[path = "surface_helpers.rs"]
mod helpers;
