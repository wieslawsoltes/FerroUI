//! Per-document/per-definition canvas aids and pointer routing. A drag changes only a
//! ghost until release; stale source, scope, camera, settings or capture retires it.
use super::*;
use crate::canvas_guide_overlay::{CanvasGuideOverlay, RULER_SIZE};
use crate::{
    CanvasGuide, CanvasGuideAxis, CanvasGuideSettings, CanvasGuidesEditor, CanvasViewport,
};
use ferroui_base::input::{IPointer, MouseButton, PointerType};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{Point, Size};
use ferroui_controls::Flyout;
use ferroui_designer_core::Ast;
use std::collections::BTreeMap;
use std::sync::Arc;

/// Accepted source and view identity for a staged guide editor. A source change retires
/// the draft even when it returns to the same text; camera changes alone are harmless.
#[derive(Clone)]
pub struct CanvasGuideEditSnapshot {
    uri: String,
    syntax: Arc<Ast>,
    revision: u64,
    generation: u64,
    scope: u64,
    epoch: u64,
}

/// Retained tab presentation. This can only restore into the matching document allocation.
#[derive(Clone)]
pub struct CanvasGuideViewSnapshot {
    identity: Rc<()>,
    uri: String,
    syntax: Arc<Ast>,
    scopes: BTreeMap<Option<NodeId>, CanvasGuideSettings>,
}

struct GuideDrag {
    pointer: Rc<dyn IPointer>,
    snapshot: CanvasGuideEditSnapshot,
    viewport: CanvasViewport,
    size: Size,
    original: Option<u64>,
    guide: CanvasGuide,
    remove: bool,
}

#[derive(Default)]
pub(super) struct GuidesUi {
    identity: RefCell<Rc<()>>,
    settings: RefCell<CanvasGuideSettings>,
    scopes: RefCell<BTreeMap<Option<NodeId>, CanvasGuideSettings>>,
    uri: RefCell<Option<String>>,
    scope: Cell<Option<NodeId>>,
    epoch: Cell<u64>,
    selected: Cell<Option<u64>>,
    drag: RefCell<Option<GuideDrag>>,
    overlay: RefCell<Option<Ref<CanvasGuideOverlay>>>,
    editor: RefCell<Option<Rc<CanvasGuidesEditor>>>,
}

impl Drop for GuidesUi {
    fn drop(&mut self) {
        if let Some(drag) = self.drag.get_mut().take() {
            drag.pointer.capture(None);
        }
        if let Some(editor) = self.editor.get_mut().take() {
            editor.retire();
        }
    }
}

impl DesignerWorkspace {
    pub fn canvas_guides(&self) -> CanvasGuideSettings {
        self.guides.settings.borrow().clone()
    }

    /// Applies a complete local view atomically. It is available to viewers and never
    /// stages/applies source text, rebuilds the preview, or changes collaboration history.
    pub fn set_canvas_guides(&self, value: CanvasGuideSettings) -> Result<(), String> {
        value.validate()?;
        self.cancel_canvas_guide_drag();
        if *self.guides.settings.borrow() == value {
            return Ok(());
        }
        self.guides.selected.set(
            self.guides.selected.get().filter(|id| {
                value.guides_visible && value.guides.iter().any(|guide| guide.id == *id)
            }),
        );
        self.surface.set_canvas_guides(value.clone())?;
        *self.guides.settings.borrow_mut() = value;
        self.guides
            .epoch
            .set(self.guides.epoch.get().wrapping_add(1));
        self.refresh_canvas_guide_overlay();
        Ok(())
    }

    pub fn add_canvas_guide(&self, axis: CanvasGuideAxis, position: f64) -> Result<u64, String> {
        let mut value = self.canvas_guides();
        let id = value.add(axis, position)?;
        self.set_canvas_guides(value)?;
        self.select_canvas_guide(Some(id));
        Ok(id)
    }

    pub fn move_canvas_guide(&self, id: u64, position: f64) -> Result<(), String> {
        CanvasGuideSettings::validate_position(position)?;
        let mut value = self.canvas_guides();
        value
            .guides
            .iter_mut()
            .find(|guide| guide.id == id)
            .ok_or("Unknown canvas guide")?
            .position = position;
        self.set_canvas_guides(value)
    }

    pub fn remove_canvas_guide(&self, id: u64) -> Result<(), String> {
        let mut value = self.canvas_guides();
        let index = value
            .guides
            .iter()
            .position(|guide| guide.id == id)
            .ok_or("Unknown canvas guide")?;
        value.guides.remove(index);
        self.set_canvas_guides(value)
    }

    pub fn selected_canvas_guide(&self) -> Option<u64> {
        self.guides.selected.get()
    }

    pub fn canvas_guide_snapshot(&self) -> CanvasGuideEditSnapshot {
        let session = self.session.borrow();
        CanvasGuideEditSnapshot {
            uri: session.document_uri().into(),
            syntax: session.document().ast_snapshot(),
            revision: session.document().revision(),
            generation: session.generation(),
            scope: session.definition_scope_epoch(),
            epoch: self.guides.epoch.get(),
        }
    }

    pub fn apply_canvas_guides(
        &self,
        snapshot: &CanvasGuideEditSnapshot,
        value: CanvasGuideSettings,
    ) -> Result<(), String> {
        self.validate_canvas_guide_snapshot(snapshot)?;
        self.set_canvas_guides(value)
    }

    fn validate_canvas_guide_snapshot(
        &self,
        snapshot: &CanvasGuideEditSnapshot,
    ) -> Result<(), String> {
        let session = self.session.borrow();
        if snapshot.uri != session.document_uri()
            || snapshot.revision != session.document().revision()
            || !Arc::ptr_eq(&snapshot.syntax, &session.document().ast_snapshot())
            || snapshot.generation != session.generation()
            || snapshot.scope != session.definition_scope_epoch()
            || snapshot.epoch != self.guides.epoch.get()
        {
            return Err(
                "The document, preview scope or guides changed; reopen Guides and snapping".into(),
            );
        }
        Ok(())
    }

    pub fn canvas_guide_view_snapshot(&self) -> CanvasGuideViewSnapshot {
        let session = self.session.borrow();
        let mut scopes = self.guides.scopes.borrow().clone();
        scopes.insert(self.guides.scope.get(), self.canvas_guides());
        CanvasGuideViewSnapshot {
            identity: self.guides.identity.borrow().clone(),
            uri: session.document_uri().into(),
            syntax: session.document().ast_snapshot(),
            scopes,
        }
    }

    pub fn restore_canvas_guide_view(&self, snapshot: &CanvasGuideViewSnapshot) -> bool {
        let session = self.session.borrow();
        if snapshot.uri != session.document_uri()
            || !Arc::ptr_eq(&snapshot.syntax, &session.document().ast_snapshot())
            || !Rc::ptr_eq(&snapshot.identity, &self.guides.identity.borrow())
        {
            return false;
        }
        drop(session);
        if snapshot
            .scopes
            .values()
            .any(|settings| settings.validate().is_err())
        {
            return false;
        }
        self.cancel_canvas_guide_drag();
        *self.guides.scopes.borrow_mut() = snapshot.scopes.clone();
        let settings = snapshot
            .scopes
            .get(&self.guides.scope.get())
            .cloned()
            .unwrap_or_default();
        let _ = self.set_canvas_guides(settings);
        true
    }

    /// Only a retained tab may reinstate its original document-view identity. Public view
    /// restoration cannot attach an old snapshot to a new same-URI/same-source document.
    pub(super) fn restore_tab_canvas_guide_view(&self, snapshot: &CanvasGuideViewSnapshot) -> bool {
        let session = self.session.borrow();
        if snapshot.uri != session.document_uri()
            || !Arc::ptr_eq(&snapshot.syntax, &session.document().ast_snapshot())
        {
            return false;
        }
        drop(session);
        *self.guides.identity.borrow_mut() = snapshot.identity.clone();
        self.restore_canvas_guide_view(snapshot)
    }

    pub(super) fn reset_canvas_guides(&self) {
        self.cancel_canvas_guide_drag();
        self.guides.scopes.borrow_mut().clear();
        *self.guides.identity.borrow_mut() = Rc::new(());
        *self.guides.uri.borrow_mut() = None;
        self.guides.scope.set(None);
        self.guides.selected.set(None);
        let _ = self.set_canvas_guides(CanvasGuideSettings::default());
    }

    pub(super) fn rebind_canvas_guide_uri(&self) {
        self.cancel_canvas_guide_drag();
        *self.guides.uri.borrow_mut() = Some(self.session.borrow().document_uri().to_owned());
        self.guides
            .epoch
            .set(self.guides.epoch.get().wrapping_add(1));
    }

    pub(super) fn refresh_canvas_guides(&self) {
        let Ok(session) = self.session.try_borrow() else {
            return;
        };
        let uri = session.document_uri().to_owned();
        let scope = session.definition_scope().map(|scope| scope.node);
        let uri_changed = self.guides.uri.borrow().as_deref() != Some(&uri);
        let scope_changed = self.guides.scope.get() != scope;
        // Removed definitions cannot leave a guide set that later attaches to another scope.
        self.guides.scopes.borrow_mut().retain(|scope, _| {
            scope.is_none_or(|node| session.document().ast().node(node).is_some())
        });
        drop(session);
        if uri_changed || scope_changed {
            self.cancel_canvas_guide_drag();
            self.guides.selected.set(None);
            if uri_changed {
                *self.guides.identity.borrow_mut() = Rc::new(());
                self.guides.scopes.borrow_mut().clear();
            } else {
                self.guides
                    .scopes
                    .borrow_mut()
                    .insert(self.guides.scope.get(), self.canvas_guides());
            }
            *self.guides.uri.borrow_mut() = Some(uri);
            self.guides.scope.set(scope);
            let value = self
                .guides
                .scopes
                .borrow()
                .get(&scope)
                .cloned()
                .unwrap_or_default();
            self.guides
                .epoch
                .set(self.guides.epoch.get().wrapping_add(1));
            let _ = self.set_canvas_guides(value);
        }
        let stale = self
            .guides
            .drag
            .borrow()
            .as_ref()
            .is_some_and(|drag| self.validate_canvas_guide_snapshot(&drag.snapshot).is_err());
        if stale {
            self.cancel_canvas_guide_drag();
        }
        self.refresh_canvas_guide_overlay();
    }

    pub(super) fn refresh_canvas_guide_overlay(&self) {
        if let Some(overlay) = self.guides.overlay.borrow().as_ref() {
            let (width, height) = self.artboard_size();
            let draft = self
                .guides
                .drag
                .borrow()
                .as_ref()
                .filter(|drag| !drag.remove)
                .map(|drag| drag.guide);
            overlay.configure(
                &self.guides.settings.borrow(),
                self.canvas_viewport(),
                Size::new(width, height),
                self.guides.selected.get(),
                draft,
            );
        }
    }

    fn select_canvas_guide(&self, id: Option<u64>) {
        self.guides.selected.set(id);
        self.refresh_canvas_guide_overlay();
    }

    pub(super) fn cancel_canvas_guide_drag(&self) {
        let drag = self.guides.drag.borrow_mut().take();
        if let Some(drag) = drag {
            self.guides.selected.set(drag.original);
            let canvas = self.root.get_control::<Control>("ArtboardScroll");
            if drag.pointer.captured().as_ref() == Some(&canvas.upcast::<InputElement>()) {
                drag.pointer.capture(None);
            }
            self.refresh_canvas_guide_overlay();
        }
    }

    fn guide_drag_current(&self, drag: &GuideDrag) -> bool {
        self.validate_canvas_guide_snapshot(&drag.snapshot).is_ok()
            && self.canvas_viewport() == drag.viewport
            && self
                .root
                .get_control::<Control>("ArtboardScroll")
                .bounds()
                .size()
                == drag.size
    }

    fn update_guide_drag_position(
        &self,
        drag: &mut GuideDrag,
        point: Point,
        modifiers: KeyModifiers,
    ) -> bool {
        if !crate::viewport::finite_point(point) {
            return false;
        }
        let local = drag.viewport.viewport_to_artboard(point);
        let settings = self.guides.settings.borrow();
        let mut position = match drag.guide.axis {
            CanvasGuideAxis::Vertical => local.x,
            CanvasGuideAxis::Horizontal => local.y,
        };
        if settings.snap_to_grid && !modifiers.contains(KeyModifiers::ALT) {
            position = (position / settings.grid_spacing).round() * settings.grid_spacing;
        }
        if CanvasGuideSettings::validate_position(position).is_err() {
            return false;
        }
        drag.guide.position = position;
        drag.remove = settings.rulers_visible
            && match drag.guide.axis {
                CanvasGuideAxis::Vertical => point.x < RULER_SIZE,
                CanvasGuideAxis::Horizontal => point.y < RULER_SIZE,
            };
        true
    }

    pub(super) fn connect_canvas_guides(self: &Rc<Self>) {
        let canvas = self.root.get_control::<Grid>("ArtboardScroll");
        let overlay = CanvasGuideOverlay::new();
        canvas.children().add(overlay.clone().upcast::<Control>());
        *self.guides.overlay.borrow_mut() = Some(overlay);
        self.connect_canvas_guide_editor();
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                if event.pointer().type_() != PointerType::Mouse
                    || !event.properties().is_left_button_pressed
                    || !this.surface.is_hit_test_visible()
                    || this.canvas_pan_tool()
                    || this.canvas_space_pan_active()
                    || event
                        .key_modifiers()
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                {
                    return;
                }
                let canvas = this.root.get_control::<Control>("ArtboardScroll");
                let point = event.get_position(Some(&canvas));
                if !crate::viewport::finite_point(point) {
                    return;
                }
                let settings = this.canvas_guides();
                let artboard = this.canvas_to_artboard(point);
                let created = if settings.rulers_visible
                    && point.y >= 0.0
                    && point.y < RULER_SIZE
                    && point.x >= RULER_SIZE
                {
                    Some((CanvasGuideAxis::Horizontal, artboard.y))
                } else if settings.rulers_visible
                    && point.x >= 0.0
                    && point.x < RULER_SIZE
                    && point.y >= RULER_SIZE
                {
                    Some((CanvasGuideAxis::Vertical, artboard.x))
                } else {
                    None
                };
                let selected = settings
                    .guides_visible
                    .then(|| {
                        settings
                            .guides
                            .iter()
                            .copied()
                            .filter_map(|guide| {
                                let projected = this
                                    .artboard_to_canvas(Point::new(guide.position, guide.position));
                                let distance = match guide.axis {
                                    CanvasGuideAxis::Vertical => (point.x - projected.x).abs(),
                                    CanvasGuideAxis::Horizontal => (point.y - projected.y).abs(),
                                };
                                (distance <= 5.0).then_some((distance, guide))
                            })
                            .min_by(|left, right| left.0.total_cmp(&right.0))
                            .map(|(_, guide)| guide)
                    })
                    .flatten();
                let (guide, original) = if let Some((axis, position)) = created {
                    let mut value = settings;
                    let Ok(id) = value.add(axis, position) else {
                        return;
                    };
                    (CanvasGuide { id, axis, position }, None)
                } else if let Some(mut guide) = selected {
                    if event.key_modifiers().contains(KeyModifiers::ALT) {
                        let mut value = settings;
                        let Ok(id) = value.add(guide.axis, guide.position) else {
                            return;
                        };
                        guide.id = id;
                        (guide, None)
                    } else {
                        (guide, Some(guide.id))
                    }
                } else {
                    this.select_canvas_guide(None);
                    return;
                };
                this.cancel_canvas_guide_drag();
                this.surface.cancel_for_navigation();
                let drag = GuideDrag {
                    pointer: event.pointer().clone(),
                    snapshot: this.canvas_guide_snapshot(),
                    viewport: this.canvas_viewport(),
                    size: canvas.bounds().size(),
                    original,
                    guide,
                    remove: false,
                };
                *this.guides.drag.borrow_mut() = Some(drag);
                this.select_canvas_guide(original.or(Some(guide.id)));
                canvas.focus();
                event
                    .pointer()
                    .capture(Some(&canvas.upcast::<InputElement>()));
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
                let result = {
                    let mut state = this.guides.drag.borrow_mut();
                    let Some(drag) = state
                        .as_mut()
                        .filter(|drag| drag.pointer.id() == event.pointer().id())
                    else {
                        return;
                    };
                    if !this.guide_drag_current(drag)
                        || !event
                            .get_current_point(None)
                            .properties
                            .is_left_button_pressed
                    {
                        false
                    } else {
                        let point = event.get_position(Some(
                            &this.root.get_control::<Control>("ArtboardScroll"),
                        ));
                        this.update_guide_drag_position(drag, point, event.key_modifiers())
                    }
                };
                if result {
                    this.refresh_canvas_guide_overlay();
                } else {
                    this.cancel_canvas_guide_drag();
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
                if event.initial_press_mouse_button() != MouseButton::Left
                    || !this
                        .guides
                        .drag
                        .borrow()
                        .as_ref()
                        .is_some_and(|drag| drag.pointer.id() == event.pointer().id())
                {
                    return;
                }
                let mut drag = this.guides.drag.borrow_mut().take().unwrap();
                let point =
                    event.get_position(Some(&this.root.get_control::<Control>("ArtboardScroll")));
                if this.guide_drag_current(&drag)
                    && this.update_guide_drag_position(&mut drag, point, event.key_modifiers())
                {
                    let mut value = this.canvas_guides();
                    if let Some(original) = drag.original {
                        value.guides.retain(|guide| guide.id != original);
                    }
                    if !drag.remove {
                        value.guides.push(drag.guide);
                        value.guides_visible = true;
                    }
                    let result = this.set_canvas_guides(value);
                    this.report(result);
                    this.select_canvas_guide((!drag.remove).then_some(drag.guide.id));
                }
                let canvas = this.root.get_control::<Control>("ArtboardScroll");
                if drag.pointer.captured().as_ref() == Some(&canvas.upcast::<InputElement>()) {
                    drag.pointer.capture(None);
                }
                this.refresh_canvas_guide_overlay();
                event.set_handled(true);
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler(
            InputElement::pointer_capture_lost_event(),
            move |sender, event| {
                if event.is_source(sender) {
                    if let Some(this) = weak.upgrade() {
                        let drag = this.guides.drag.borrow_mut().take();
                        if let Some(drag) = drag {
                            this.guides.selected.set(drag.original);
                        }
                        this.refresh_canvas_guide_overlay();
                    }
                }
            },
        );
        let weak = Rc::downgrade(self);
        canvas.add_handler(InputElement::lost_focus_event(), move |_, _| {
            if let Some(this) = weak.upgrade() {
                if !this
                    .root
                    .get_control::<Control>("ArtboardScroll")
                    .is_keyboard_focus_within()
                {
                    this.cancel_canvas_guide_drag();
                    this.select_canvas_guide(None);
                }
            }
        });
        let weak = Rc::downgrade(self);
        canvas.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                let Some(this) = weak.upgrade() else { return };
                if this.text_input_focused() || !this.surface.is_hit_test_visible() {
                    return;
                }
                if event.key == Key::Escape
                    && (this.guides.drag.borrow().is_some() || this.guides.selected.get().is_some())
                {
                    this.cancel_canvas_guide_drag();
                    this.select_canvas_guide(None);
                    event.set_handled(true);
                    return;
                }
                if event.key_modifiers == KeyModifiers::SHIFT
                    && matches!(event.key, Key::R | Key::G)
                {
                    let mut value = this.canvas_guides();
                    if event.key == Key::R {
                        value.rulers_visible = !value.rulers_visible;
                    } else {
                        value.guides_visible = !value.guides_visible;
                    }
                    this.report(this.set_canvas_guides(value));
                    event.set_handled(true);
                    return;
                }
                let Some(id) = this.guides.selected.get() else {
                    return;
                };
                if event
                    .key_modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::META | KeyModifiers::ALT)
                {
                    return;
                }
                let settings = this.canvas_guides();
                let Some(guide) = settings.guides.iter().find(|guide| guide.id == id) else {
                    return;
                };
                let step = if event.key_modifiers.contains(KeyModifiers::SHIFT) {
                    settings.large_nudge
                } else {
                    settings.small_nudge
                };
                let result = match event.key {
                    Key::Delete | Key::Back => Some(this.remove_canvas_guide(id)),
                    Key::Left if guide.axis == CanvasGuideAxis::Vertical => {
                        Some(this.move_canvas_guide(id, guide.position - step))
                    }
                    Key::Right if guide.axis == CanvasGuideAxis::Vertical => {
                        Some(this.move_canvas_guide(id, guide.position + step))
                    }
                    Key::Up if guide.axis == CanvasGuideAxis::Horizontal => {
                        Some(this.move_canvas_guide(id, guide.position - step))
                    }
                    Key::Down if guide.axis == CanvasGuideAxis::Horizontal => {
                        Some(this.move_canvas_guide(id, guide.position + step))
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
        self.refresh_canvas_guides();
    }

    fn connect_canvas_guide_editor(self: &Rc<Self>) {
        let button = self.root.get_control::<Button>("CanvasGuides");
        let flyout = Flyout::new();
        flyout.flyout_presenter_classes().add("designer-chrome");
        button.set_flyout(Some(flyout.clone().upcast()));
        let weak = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout.opening(move |_| {
            let (Some(this), Some(flyout)) = (weak.upgrade(), popup.upgrade()) else {
                return;
            };
            this.cancel_canvas_guide_drag();
            let snapshot = this.canvas_guide_snapshot();
            let editor = CanvasGuidesEditor::new(this.canvas_guides());
            let weak = Rc::downgrade(&this);
            let close = flyout.downgrade();
            editor.on_apply(move |settings| {
                let this = weak.upgrade().ok_or("The designer workspace was closed")?;
                this.apply_canvas_guides(&snapshot, settings)?;
                if let Some(flyout) = close.upgrade() {
                    flyout.hide();
                }
                Ok(())
            });
            let close = flyout.downgrade();
            editor.on_cancel(move || {
                if let Some(flyout) = close.upgrade() {
                    flyout.hide();
                }
            });
            flyout.set_content(Some(Control::boxed(editor.control())));
            *this.guides.editor.borrow_mut() = Some(editor);
        });
        let weak = Rc::downgrade(self);
        flyout.closed(move || {
            if let Some(this) = weak.upgrade() {
                if let Some(editor) = this.guides.editor.borrow_mut().take() {
                    editor.retire();
                }
            }
        });
    }
}
