//! Canvas camera and native input routing. Application layout remains in authored units.
use super::*;
use crate::{CanvasViewport, CanvasWheelMode};
use ferroui_base::input::{Cursor, IPointer, MouseButton, PointerType, StandardCursorType};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{Point, Rect, Size, Vector, Visual};
use ferroui_controls::Canvas;

struct PanGesture {
    pointer: Rc<dyn IPointer>,
    button: MouseButton,
    start: Point,
    viewport: CanvasViewport,
    uri: String,
    revision: u64,
    generation: u64,
    scope: u64,
}

/// The reference viewport prevents an entering/leaving scope banner or a resized dock
/// from applying the same recentering delta twice when a saved view is restored.
#[derive(Clone, Copy)]
pub(super) struct RetainedCanvasViewport {
    viewport: CanvasViewport,
    size: Size,
}

impl PanGesture {
    fn matches(&self, session: &DesignerSession) -> bool {
        session.document_uri() == self.uri
            && session.document().revision() == self.revision
            && session.generation() == self.generation
            && session.definition_scope_epoch() == self.scope
    }
}

pub(super) struct NavigationUi {
    viewport: Cell<CanvasViewport>,
    size: Cell<Size>,
    initialized: Cell<bool>,
    wheel: Cell<CanvasWheelMode>,
    space: Cell<bool>,
    hand: Cell<bool>,
    pan: RefCell<Option<PanGesture>>,
    uri: RefCell<Option<String>>,
    scope: Cell<Option<NodeId>>,
    document_view: Cell<Option<RetainedCanvasViewport>>,
    layout: RefCell<Option<Rc<dyn ferroui_base::reactive::IDisposable>>>,
}

impl Default for NavigationUi {
    fn default() -> Self {
        Self {
            viewport: Cell::new(CanvasViewport::default()),
            size: Cell::new(Size::new(0.0, 0.0)),
            initialized: Cell::new(false),
            wheel: Cell::new(CanvasWheelMode::default()),
            space: Cell::new(false),
            hand: Cell::new(false),
            pan: RefCell::new(None),
            uri: RefCell::new(None),
            scope: Cell::new(None),
            document_view: Cell::new(None),
            layout: RefCell::new(None),
        }
    }
}

impl Drop for NavigationUi {
    fn drop(&mut self) {
        if let Some(subscription) = self.layout.get_mut().take() {
            subscription.dispose();
        }
        if let Some(pan) = self.pan.get_mut().take() {
            pan.pointer.capture(None);
        }
    }
}

impl DesignerWorkspace {
    pub fn canvas_viewport(&self) -> CanvasViewport {
        self.navigation.viewport.get()
    }

    /// Restore a host-owned view. This never applies pending source or mutates undo history.
    pub fn set_canvas_viewport(&self, viewport: CanvasViewport) {
        self.cancel_canvas_guide_drag();
        self.end_canvas_pan(false);
        self.navigation.initialized.set(true);
        self.surface.cancel_for_navigation();
        self.apply_canvas_viewport(viewport);
    }

    pub fn canvas_wheel_mode(&self) -> CanvasWheelMode {
        self.navigation.wheel.get()
    }

    pub fn set_canvas_wheel_mode(&self, mode: CanvasWheelMode) {
        self.navigation.wheel.set(mode);
        for (name, candidate) in [
            ("WheelPan", CanvasWheelMode::Pan),
            ("WheelZoom", CanvasWheelMode::Zoom),
        ] {
            self.root
                .get_control::<Button>(name)
                .classes()
                .set("active", mode == candidate);
        }
    }

    pub fn canvas_pan_tool(&self) -> bool {
        self.navigation.hand.get()
    }

    pub fn set_canvas_pan_tool(&self, active: bool) {
        self.cancel_canvas_guide_drag();
        self.end_canvas_pan(false);
        self.navigation.hand.set(active);
        self.navigation.space.set(false);
        self.surface.cancel_for_navigation();
        if active {
            self.surface.set_interactive(false);
        }
        self.refresh_canvas_cursor();
        self.refresh_shell();
    }

    /// Map an authored point through the same transform used for rendering and input.
    pub fn artboard_to_canvas(&self, point: Point) -> Point {
        self.canvas_viewport().artboard_to_viewport(point)
    }

    pub fn canvas_to_artboard(&self, point: Point) -> Point {
        self.canvas_viewport().viewport_to_artboard(point)
    }

    /// Cursor-anchored zoom. `anchor` is relative to the ArtboardScroll viewport control.
    pub fn zoom_canvas_at(&self, zoom: f64, anchor: Point) -> Result<(), String> {
        let viewport = self.canvas_viewport().zoom_at(zoom, anchor)?;
        self.set_canvas_viewport(viewport);
        Ok(())
    }

    pub fn pan_canvas(&self, delta: Vector) -> Result<(), String> {
        self.set_canvas_viewport(self.canvas_viewport().pan(delta)?);
        Ok(())
    }

    /// Accepts a percentage with an optional `%`; invalid drafts do not change the view.
    pub fn set_zoom_percent(&self, text: &str) -> Result<(), String> {
        let text = text.trim();
        let value = text
            .strip_suffix('%')
            .unwrap_or(text)
            .trim()
            .parse::<f64>()
            .map_err(|_| "Enter a zoom percentage from 1 to 1600".to_owned())?;
        if !value.is_finite()
            || !(CanvasViewport::MIN_ZOOM * 100.0..=CanvasViewport::MAX_ZOOM * 100.0)
                .contains(&value)
        {
            return Err("Enter a zoom percentage from 1 to 1600".into());
        }
        self.zoom_canvas_at(value / 100.0, self.canvas_center())
    }

    /// Fit the actual transformed union of the selected authored native visuals.
    pub fn fit_selection(&self) -> Result<(), String> {
        self.require_canvas_size()?;
        let bounds = self
            .session
            .borrow()
            .selection_bounds(&self.artboard.clone().upcast::<Visual>())
            .into_iter()
            .map(|(_, bounds)| bounds)
            .filter(|bounds| bounds.width > 0.0 && bounds.height > 0.0)
            .reduce(|left, right| left.union(right))
            .ok_or("Select a visible authored control before fitting the selection")?;
        self.fit_canvas_bounds(bounds)
    }

    pub(super) fn fit_canvas_bounds(&self, bounds: Rect) -> Result<(), String> {
        let size = self.require_canvas_size()?;
        // Keep the title and floating tool shelf outside the fitted content.
        let viewport = CanvasViewport::fit(bounds, size, Thickness::new(40.0, 40.0, 40.0, 76.0))?;
        self.set_canvas_viewport(viewport);
        Ok(())
    }

    fn require_canvas_size(&self) -> Result<Size, String> {
        let canvas = self.root.get_control::<Control>("ArtboardScroll");
        let size = canvas.bounds().size();
        if !canvas.is_visible() || size.width <= 80.0 || size.height <= 116.0 {
            return Err("Show and arrange the design canvas before fitting".into());
        }
        Ok(size)
    }

    fn canvas_center(&self) -> Point {
        let size = self
            .root
            .get_control::<Control>("ArtboardScroll")
            .bounds()
            .size();
        Point::new(size.width / 2.0, size.height / 2.0)
    }

    pub(super) fn set_unmeasured_canvas_zoom(&self, zoom: f64) -> bool {
        let size = self
            .root
            .get_control::<Control>("ArtboardScroll")
            .bounds()
            .size();
        if self.navigation.initialized.get() || size.width > 0.0 || size.height > 0.0 {
            return false;
        }
        if let Ok(viewport) = CanvasViewport::new(zoom, self.canvas_viewport().offset()) {
            self.apply_canvas_viewport(viewport);
        }
        true
    }

    fn apply_canvas_viewport(&self, viewport: CanvasViewport) {
        self.cancel_canvas_guide_drag();
        self.navigation.viewport.set(viewport);
        self.surface.set_zoom(viewport.zoom());
        self.artboard_transform.set_render_transform(Some(
            MatrixTransform::with_matrix(viewport.transform()).into(),
        ));
        let title = self.root.get_control::<TextBlock>("ArtboardTitle");
        Canvas::set_left(&title, viewport.offset().x);
        Canvas::set_top(&title, viewport.offset().y - 24.0);
        let percent = format!("{:.1}", viewport.zoom() * 100.0);
        let percent = percent.trim_end_matches(".0");
        self.root
            .get_control::<Button>("ZoomReset")
            .set_content(Some(Rc::new(format!("{percent}%"))));
        let field = self.root.get_control::<TextBox>("ZoomPercent");
        if !field.is_focused() {
            field.set_text(Some(percent));
        }
        self.surface.invalidate_visual();
        self.refresh_canvas_guide_overlay();
    }

    fn retain_canvas_viewport(&self) -> Option<RetainedCanvasViewport> {
        self.navigation
            .initialized
            .get()
            .then(|| RetainedCanvasViewport {
                viewport: self.canvas_viewport(),
                size: self.navigation.size.get(),
            })
    }

    pub(super) fn document_canvas_viewport(&self) -> Option<RetainedCanvasViewport> {
        self.navigation
            .document_view
            .get()
            .or_else(|| self.retain_canvas_viewport())
    }

    pub(super) fn restore_canvas_viewport(&self, viewport: Option<RetainedCanvasViewport>) {
        self.refresh_canvas_navigation();
        if let Some(viewport) = viewport {
            self.restore_retained_canvas_viewport(viewport);
        }
    }

    fn restore_retained_canvas_viewport(&self, retained: RetainedCanvasViewport) {
        let size = self
            .root
            .get_control::<Control>("ArtboardScroll")
            .bounds()
            .size();
        let viewport = if retained.size.width > 0.0
            && retained.size.height > 0.0
            && size.width > 0.0
            && size.height > 0.0
        {
            retained
                .viewport
                .pan(Vector::new(
                    (size.width - retained.size.width) / 2.0,
                    (size.height - retained.size.height) / 2.0,
                ))
                .unwrap_or(retained.viewport)
        } else {
            retained.viewport
        };
        self.set_canvas_viewport(viewport);
        self.navigation.size.set(size);
    }

    /// Successful Save As changes document identity while retaining its current camera.
    /// This is explicit so opening another document with identical source still resets it.
    pub(super) fn rebind_canvas_document_uri(&self) {
        self.rebind_canvas_guide_uri();
        self.cancel_canvas_navigation();
        *self.navigation.uri.borrow_mut() = Some(self.session.borrow().document_uri().to_owned());
    }

    pub(super) fn refresh_canvas_navigation(&self) {
        let Ok(session) = self.session.try_borrow() else {
            return;
        };
        let uri = session.document_uri().to_owned();
        let scope = session.definition_scope().map(|scope| scope.node);
        let uri_changed = self.navigation.uri.borrow().as_deref() != Some(uri.as_str());
        let scope_changed = self.navigation.scope.get() != scope;
        drop(session);
        if uri_changed || scope_changed {
            self.end_canvas_pan(false);
            self.navigation.space.set(false);
            let previous_scope = self.navigation.scope.replace(scope);
            *self.navigation.uri.borrow_mut() = Some(uri);
            if uri_changed {
                self.navigation.document_view.set(None);
            } else if previous_scope.is_none() && scope.is_some() {
                self.navigation
                    .document_view
                    .set(self.retain_canvas_viewport());
            }
            if !uri_changed && scope.is_none() {
                if let Some(viewport) = self.navigation.document_view.take() {
                    self.restore_retained_canvas_viewport(viewport);
                    self.refresh_canvas_layout();
                    return;
                }
            }
            self.navigation.initialized.set(false);
            self.navigation.viewport.set(CanvasViewport::default());
            self.refresh_canvas_cursor();
        }
        self.refresh_canvas_layout();
    }

    fn refresh_canvas_layout(&self) {
        let size = self
            .root
            .get_control::<Control>("ArtboardScroll")
            .bounds()
            .size();
        if !crate::viewport::finite_size(size) || size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let old = self.navigation.size.replace(size);
        if !self.navigation.initialized.replace(true) {
            let (width, height) = self.artboard_size();
            let zoom = self.canvas_viewport().zoom();
            let offset = Vector::new(
                ((size.width - width * zoom) / 2.0).max(40.0),
                ((size.height - height * zoom - 36.0) / 2.0).max(40.0),
            );
            if let Ok(viewport) = CanvasViewport::new(zoom, offset) {
                self.apply_canvas_viewport(viewport);
            }
        } else if old != size && old.width > 0.0 && old.height > 0.0 {
            self.end_canvas_pan(false);
            self.surface.cancel_for_navigation();
            if let Ok(viewport) = self.canvas_viewport().pan(Vector::new(
                (size.width - old.width) / 2.0,
                (size.height - old.height) / 2.0,
            )) {
                self.apply_canvas_viewport(viewport);
            }
        }
    }

    pub(super) fn canvas_space_pan_active(&self) -> bool {
        self.navigation.space.get()
    }

    pub(super) fn canvas_navigation_active(&self) -> bool {
        self.navigation.pan.borrow().is_some()
            || self.canvas_space_pan_active()
            || self.canvas_pan_tool()
    }

    fn refresh_canvas_cursor(&self) {
        let canvas = self.root.get_control::<Control>("ArtboardScroll");
        canvas.set_cursor(if self.navigation.pan.borrow().is_some() {
            Some(Cursor::new(StandardCursorType::SizeAll))
        } else if self.navigation.space.get() || self.navigation.hand.get() {
            Some(Cursor::new(StandardCursorType::Hand))
        } else {
            None
        });
    }

    fn end_canvas_pan(&self, restore: bool) {
        let pan = self.navigation.pan.borrow_mut().take();
        if let Some(pan) = pan {
            if restore
                && self
                    .session
                    .try_borrow()
                    .is_ok_and(|session| pan.matches(&session))
            {
                self.apply_canvas_viewport(pan.viewport);
            }
            let canvas = self.root.get_control::<Control>("ArtboardScroll");
            if pan.pointer.captured().as_ref() == Some(&canvas.upcast::<InputElement>()) {
                pan.pointer.capture(None);
            }
        }
        self.refresh_canvas_cursor();
    }

    pub(super) fn cancel_canvas_navigation(&self) {
        self.cancel_canvas_guide_drag();
        self.navigation.space.set(false);
        self.end_canvas_pan(false);
        self.surface.cancel_for_navigation();
    }

    pub(super) fn connect_canvas_navigation(self: &Rc<Self>) {
        let canvas = self.root.get_control::<Control>("ArtboardScroll");
        let weak = Rc::downgrade(self);
        *self.navigation.layout.borrow_mut() = Some(canvas.layout_updated(move || {
            if let Some(this) = weak.upgrade() {
                this.refresh_canvas_layout();
            }
        }));
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                let modifiers = event.key_modifiers();
                let control = modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META);
                if modifiers.contains(KeyModifiers::ALT)
                    || (!this.surface.is_hit_test_visible() && !control)
                {
                    return;
                }
                let delta = event.delta();
                if !delta.x.is_finite() || !delta.y.is_finite() || delta == Vector::ZERO {
                    return;
                }
                let horizontal = modifiers.contains(KeyModifiers::SHIFT);
                let zoom =
                    !horizontal && (control != (this.canvas_wheel_mode() == CanvasWheelMode::Zoom));
                let result = if zoom {
                    let steps = if delta.y != 0.0 { delta.y } else { delta.x }.clamp(-32.0, 32.0);
                    let canvas = this.root.get_control::<Control>("ArtboardScroll");
                    this.zoom_canvas_at(
                        this.zoom() * 1.2_f64.powf(steps),
                        event.get_position(Some(&canvas)),
                    )
                } else {
                    let delta = if horizontal {
                        Vector::new(if delta.x != 0.0 { delta.x } else { delta.y }, 0.0)
                    } else {
                        delta
                    };
                    this.pan_canvas(Vector::new(
                        delta.x.clamp(-100.0, 100.0) * 48.0,
                        delta.y.clamp(-100.0, 100.0) * 48.0,
                    ))
                };
                if result.is_ok() {
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                if event.pointer().type_() != PointerType::Mouse {
                    return;
                }
                let properties = event.properties();
                let button = if properties.is_middle_button_pressed {
                    MouseButton::Middle
                } else if properties.is_left_button_pressed
                    && (this.navigation.space.get() || this.navigation.hand.get())
                {
                    MouseButton::Left
                } else {
                    return;
                };
                this.cancel_canvas_guide_drag();
                this.end_canvas_pan(false);
                this.surface.cancel_for_navigation();
                let canvas = this.root.get_control::<Control>("ArtboardScroll");
                let position = event.get_position(Some(&canvas));
                if !crate::viewport::finite_point(position) {
                    return;
                }
                let session = this.session.borrow();
                let pan = PanGesture {
                    pointer: event.pointer().clone(),
                    button,
                    start: position,
                    viewport: this.canvas_viewport(),
                    uri: session.document_uri().to_owned(),
                    revision: session.document().revision(),
                    generation: session.generation(),
                    scope: session.definition_scope_epoch(),
                };
                drop(session);
                *this.navigation.pan.borrow_mut() = Some(pan);
                canvas.focus();
                event
                    .pointer()
                    .capture(Some(&canvas.upcast::<InputElement>()));
                this.refresh_canvas_cursor();
                event.set_handled(true);
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::pointer_moved_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                let viewport = {
                    let state = this.navigation.pan.borrow();
                    let Some(pan) = state
                        .as_ref()
                        .filter(|pan| pan.pointer.id() == event.pointer().id())
                    else {
                        return;
                    };
                    let session = this.session.borrow();
                    let properties = event.get_current_point(None).properties;
                    let current = pan.matches(&session)
                        && match pan.button {
                            MouseButton::Middle => properties.is_middle_button_pressed,
                            _ => properties.is_left_button_pressed,
                        };
                    let position = event
                        .get_position(Some(&this.root.get_control::<Control>("ArtboardScroll")));
                    current.then(|| {
                        pan.viewport.pan(Vector::new(
                            position.x - pan.start.x,
                            position.y - pan.start.y,
                        ))
                    })
                };
                match viewport {
                    Some(Ok(viewport)) => this.apply_canvas_viewport(viewport),
                    _ => this.end_canvas_pan(false),
                }
                event.set_handled(true);
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::pointer_released_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                let finish = this.navigation.pan.borrow().as_ref().is_some_and(|pan| {
                    pan.pointer.id() == event.pointer().id()
                        && pan.button == event.initial_press_mouse_button()
                });
                if finish {
                    this.end_canvas_pan(false);
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler(
            InputElement::pointer_capture_lost_event(),
            move |sender, event| {
                let Some(this) = weak.upgrade() else { return };
                if event.is_source(sender) {
                    // Intentional release removed the gesture before releasing capture.
                    // Keep a physically held Space key active for the next hand drag.
                    if this.navigation.pan.borrow_mut().take().is_some() {
                        this.navigation.space.set(false);
                    }
                    this.refresh_canvas_cursor();
                }
            },
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                if event.key == Key::Escape && this.navigation.pan.borrow().is_some() {
                    this.end_canvas_pan(true);
                    this.navigation.space.set(false);
                    this.refresh_canvas_cursor();
                    event.set_handled(true);
                    return;
                }
                if this.text_input_focused() || !this.surface.is_hit_test_visible() {
                    return;
                }
                let modifiers = event.key_modifiers;
                if event.key == Key::Space && modifiers.is_empty() {
                    this.navigation.space.set(true);
                    this.refresh_canvas_cursor();
                    event.set_handled(true);
                    return;
                }
                let control = modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META);
                let result = match event.key {
                    Key::D1 if modifiers == KeyModifiers::SHIFT => Some(this.fit_artboard()),
                    Key::D2 if modifiers == KeyModifiers::SHIFT => Some(this.fit_selection()),
                    Key::D0 if control && !modifiers.contains(KeyModifiers::ALT) => {
                        Some(this.zoom_canvas_at(1.0, this.canvas_center()))
                    }
                    Key::OemPlus | Key::Add if !modifiers.contains(KeyModifiers::ALT) => {
                        Some(this.zoom_canvas_at(this.zoom() * 1.2, this.canvas_center()))
                    }
                    Key::OemMinus | Key::Subtract if !modifiers.contains(KeyModifiers::ALT) => {
                        Some(this.zoom_canvas_at(this.zoom() / 1.2, this.canvas_center()))
                    }
                    _ => None,
                };
                if let Some(result) = result {
                    this.report(result);
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::key_up_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                if event.key == Key::Space && this.navigation.space.replace(false) {
                    this.end_canvas_pan(false);
                    this.refresh_canvas_cursor();
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler(InputElement::lost_focus_event(), move |_, _| {
            let Some(this) = weak.upgrade() else { return };
            if !this
                .root
                .get_control::<Control>("ArtboardScroll")
                .is_keyboard_focus_within()
            {
                this.navigation.space.set(false);
                this.end_canvas_pan(false);
                this.refresh_canvas_cursor();
            }
        });
        for (name, command) in [
            ("ZoomActualSize", 0),
            ("ZoomFitMenu", 1),
            ("ZoomSelection", 2),
            ("ZoomApply", 3),
            ("WheelPan", 4),
            ("WheelZoom", 5),
        ] {
            let weak = Rc::downgrade(self);
            self.root.get_control::<Button>(name).click(move |_, _| {
                let Some(this) = weak.upgrade() else { return };
                let result = match command {
                    0 => this.zoom_canvas_at(1.0, this.canvas_center()),
                    1 => this.fit_artboard(),
                    2 => this.fit_selection(),
                    3 => this.set_zoom_percent(
                        &this
                            .root
                            .get_control::<TextBox>("ZoomPercent")
                            .text()
                            .unwrap_or_default(),
                    ),
                    4 => {
                        this.set_canvas_wheel_mode(CanvasWheelMode::Pan);
                        Ok(())
                    }
                    _ => {
                        this.set_canvas_wheel_mode(CanvasWheelMode::Zoom);
                        Ok(())
                    }
                };
                this.root
                    .get_control::<TextBlock>("ZoomError")
                    .set_text(result.as_ref().err().map(String::as_str));
                this.report(result);
            });
        }
        let weak = Rc::downgrade(self);
        self.root.get_control::<TextBox>("ZoomPercent").add_handler(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key != Key::Enter {
                    return;
                }
                let Some(this) = weak.upgrade() else { return };
                let result = this.set_zoom_percent(
                    &this
                        .root
                        .get_control::<TextBox>("ZoomPercent")
                        .text()
                        .unwrap_or_default(),
                );
                this.root
                    .get_control::<TextBlock>("ZoomError")
                    .set_text(result.as_ref().err().map(String::as_str));
                this.report(result);
                event.set_handled(true);
            },
        );
        self.set_canvas_wheel_mode(CanvasWheelMode::default());
    }
}
