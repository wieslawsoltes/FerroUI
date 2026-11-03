//! Derived from ScreenKeyboard.Uno/Views/KeysView.cs (Apache-2.0).
use super::{KeyView, KeyViewError};
use crate::{
    primitives::Brushes,
    themes::{KeyboardTheme, KeyboardThemes},
};
use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{
    engine::{EngineError, KeyboardEngine},
    input::{IKeyboardScheduler, KeyboardTouchProcessor, TouchError},
    layouts::{ComputedKey, ComputedKeyboard, KeyboardGeometryOptions},
};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, Point, Ref, Size, StyledElementImpl, VisualImpl,
    ferro_class, ferro_impl_classes,
    input::{
        IPointer, InputElement, InputElementImpl, PointerEventArgs, PointerPressedEventArgs,
        PointerReleasedEventArgs,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{FontFamily, PenLineCap, PenLineJoin, Points},
};
use ferroui_controls::{
    Canvas, ControlImpl, Grid, PanelImpl, automation::ElementNotEnabledError, shapes::Polyline,
};
use ferroui_system::events::HandlerList;
use indexmap::IndexMap;
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
    sync::Arc,
};

#[derive(Debug)]
pub(crate) enum KeysViewError {
    Engine(EngineError),
    Key(KeyViewError),
    Touch(TouchError),
    ViewsCollectionModified,
}
impl From<EngineError> for KeysViewError {
    fn from(e: EngineError) -> Self {
        Self::Engine(e)
    }
}
impl From<KeyViewError> for KeysViewError {
    fn from(e: KeyViewError) -> Self {
        Self::Key(e)
    }
}
impl From<TouchError> for KeysViewError {
    fn from(e: TouchError) -> Self {
        Self::Touch(e)
    }
}
impl std::fmt::Display for KeysViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Engine(e) => write!(f, "{e}"),
            Self::Key(e) => write!(f, "{e}"),
            Self::Touch(e) => write!(f, "{e}"),
            Self::ViewsCollectionModified => {
                write!(f, "key view collection modified during enumeration")
            }
        }
    }
}
impl std::error::Error for KeysViewError {}
type VisualHandlers = HandlerList<dyn Fn(&KeysView, &Rc<ComputedKey>)>;

#[derive(Clone)]
struct KeyIdentity(Rc<ComputedKey>);
impl PartialEq for KeyIdentity {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for KeyIdentity {}
impl std::hash::Hash for KeyIdentity {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&Rc::as_ptr(&self.0), state);
    }
}

#[repr(C)]
pub(crate) struct KeysView {
    base: Grid,
    keys_layer: Ref<Canvas>,
    trail_layer: Ref<Canvas>,
    trail: Ref<Polyline>,
    // Native insertion-ordered map over retained Rc key identities. Replacing a duplicate preserves its slot;
    // the source still appends both visual children when a key occurs twice.
    views: RefCell<IndexMap<KeyIdentity, Ref<KeyView>>>,
    views_version: Cell<u32>,
    captured_pointers: RefCell<HashSet<i32>>,
    engine: Rc<KeyboardEngine>,
    touch: Rc<KeyboardTouchProcessor>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    rendered_keyboard: RefCell<Option<Rc<ComputedKeyboard>>>,
    rendered_version: Cell<i32>,
    theme: RefCell<Arc<KeyboardTheme>>,
    trail_fade_timer: RefCell<Option<ScheduledAction>>,
    geometry: RefCell<Rc<KeyboardGeometryOptions>>,
    font_scale: Cell<f64>,
    label_font_family: RefCell<Option<FontFamily>>,
    key_pressed_visual: VisualHandlers,
    key_released_visual: VisualHandlers,
    touch_subscriptions: Cell<Option<[u64; 4]>>,
}
ferro_class!(KeysView:Grid);
ferro_impl_classes!(KeysView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for KeysView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_background(Some(Brushes::transparent().into()));
        this.set_is_tab_stop(false);
        this.set_allow_focus_on_interaction(false);
        // Pointer presses opt out of native ancestor gesture recognition below.
        this.keys_layer.set_is_hit_test_visible(false);
        this.trail_layer.set_is_hit_test_visible(false);
        this.trail.set_is_hit_test_visible(false);
        this.trail.set_stroke_join(PenLineJoin::Round);
        this.trail.set_stroke_line_cap(PenLineCap::Round);
        this.trail_layer.children().add(this.trail.clone());
        this.children().add(this.keys_layer.clone());
        this.children().add(this.trail_layer.clone());
        let weak = this.to_ref().downgrade();
        this.size_changed(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.on_size_changed(e.new_size())
                    .expect("KeysView size change");
            }
        });
        let weak = this.to_ref().downgrade();
        this.pointer_pressed(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.pointer_down(e).expect("KeysView pointer down");
            }
        });
        let weak = this.to_ref().downgrade();
        this.pointer_moved(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.pointer_move(e).expect("KeysView pointer move");
            }
        });
        let weak = this.to_ref().downgrade();
        this.pointer_released(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.pointer_up(e).expect("KeysView pointer up");
            }
        });
        let weak = this.to_ref().downgrade();
        this.pointer_capture_lost(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.pointer_cancel(e.pointer())
                    .expect("KeysView capture loss");
            }
        });
        // The touch processor is borrowed. Its notifications must not own this native view.
        let weak = this.to_ref().downgrade();
        let pressed = this.touch.key_pressed().add(Rc::new(move |key| {
            if let Some(view) = weak.upgrade() {
                view.set_pressed(key, true).expect("KeysView press visual");
            }
        }));
        let weak = this.to_ref().downgrade();
        let released = this.touch.key_released().add(Rc::new(move |key| {
            if let Some(view) = weak.upgrade() {
                view.set_pressed(key, false)
                    .expect("KeysView release visual");
            }
        }));
        let weak = this.to_ref().downgrade();
        let trail = this.touch.glide_trail_changed().add(Rc::new(move |_| {
            if let Some(view) = weak.upgrade() {
                view.update_trail();
            }
        }));
        let weak = this.to_ref().downgrade();
        let ended = this.touch.glide_ended().add(Rc::new(move |_| {
            if let Some(view) = weak.upgrade() {
                view.schedule_trail_fade();
            }
        }));
        this.touch_subscriptions
            .set(Some([pressed, released, trail, ended]));
    }
}
impl KeysView {
    pub fn construct(
        engine: Rc<KeyboardEngine>,
        touch: Rc<KeyboardTouchProcessor>,
        scheduler: Rc<dyn IKeyboardScheduler>,
    ) -> Self {
        Self {
            base: Grid::construct(),
            keys_layer: Canvas::new(),
            trail_layer: Canvas::new(),
            trail: Polyline::new(),
            views: RefCell::new(IndexMap::new()),
            views_version: Cell::new(0),
            captured_pointers: RefCell::new(HashSet::new()),
            engine,
            touch,
            scheduler,
            rendered_keyboard: RefCell::new(None),
            rendered_version: Cell::new(-1),
            theme: RefCell::new(KeyboardThemes::floris_day()),
            trail_fade_timer: RefCell::new(None),
            geometry: RefCell::new(Rc::new(KeyboardGeometryOptions::default())),
            font_scale: Cell::new(1.0),
            label_font_family: RefCell::new(None),
            key_pressed_visual: HandlerList::with_ordered_ids(),
            key_released_visual: HandlerList::with_ordered_ids(),
            touch_subscriptions: Cell::new(None),
        }
    }
    pub fn new(
        engine: Rc<KeyboardEngine>,
        touch: Rc<KeyboardTouchProcessor>,
        scheduler: Rc<dyn IKeyboardScheduler>,
    ) -> Ref<Self> {
        instantiate(Self::construct(engine, touch, scheduler))
    }
    pub fn geometry(&self) -> Rc<KeyboardGeometryOptions> {
        self.geometry.borrow().clone()
    }
    pub fn set_geometry(&self, value: Rc<KeyboardGeometryOptions>) {
        *self.geometry.borrow_mut() = value;
    }
    pub fn font_scale(&self) -> f64 {
        self.font_scale.get()
    }
    pub fn set_font_scale(&self, value: f64) {
        self.font_scale.set(value);
    }
    pub fn label_font_family(&self) -> Option<FontFamily> {
        self.label_font_family.borrow().clone()
    }
    pub fn set_label_font_family(&self, value: Option<FontFamily>) {
        *self.label_font_family.borrow_mut() = value;
    }
    pub fn key_pressed_visual(
        &self,
        handler: impl Fn(&KeysView, &Rc<ComputedKey>) + 'static,
    ) -> u64 {
        self.key_pressed_visual.add(Rc::new(handler))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_key_pressed_visual(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.key_pressed_visual, token)
    }
    pub fn key_released_visual(
        &self,
        handler: impl Fn(&KeysView, &Rc<ComputedKey>) + 'static,
    ) -> u64 {
        self.key_released_visual.add(Rc::new(handler))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_key_released_visual(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.key_released_visual, token)
    }
    /// Cancel fade work owned by this view, without changing the borrowed touch processor.
    pub(crate) fn cancel_trail_fade(&self) {
        let timer = self.trail_fade_timer.replace(None);
        if let Some(timer) = timer {
            timer.cancel();
        }
    }
    pub fn release_subscriptions(&self) {
        if let Some([p, r, t, e]) = self.touch_subscriptions.take() {
            self.touch.key_pressed().remove(p);
            self.touch.key_released().remove(r);
            self.touch.glide_trail_changed().remove(t);
            self.touch.glide_ended().remove(e);
        }
    }
    pub fn set_theme(&self, theme: Arc<KeyboardTheme>) -> Result<(), KeysViewError> {
        *self.theme.borrow_mut() = theme.clone();
        self.trail
            .set_stroke(Some(Brushes::get(theme.glide_trail_color()).into()));
        self.trail
            .set_stroke_thickness(theme.glide_trail_thickness());
        self.trail.set_opacity(0.75);
        self.refresh(true)
    }
    fn on_size_changed(&self, size: Size) -> Result<(), KeysViewError> {
        if size.width <= 0.0 || size.height <= 0.0 {
            return Ok(());
        }
        self.engine
            .update_layout(size.width, size.height, Some(self.geometry()))?;
        self.refresh(true)
    }
    pub fn relayout(&self) -> Result<(), KeysViewError> {
        let size = self.bounds().size();
        if size.width > 0.0 && size.height > 0.0 {
            self.engine
                .update_layout(size.width, size.height, Some(self.geometry()))?;
        }
        self.refresh(true)
    }
    pub fn refresh(&self, mut force: bool) -> Result<(), KeysViewError> {
        let keyboard = self.engine.keyboard()?;
        let size = self.bounds().size();
        if size.width > 0.0
            && size.height > 0.0
            && (keyboard.width() != size.width || keyboard.height() != size.height)
        {
            self.engine
                .update_layout(size.width, size.height, Some(self.geometry()))?;
        }
        if !self
            .rendered_keyboard
            .borrow()
            .as_ref()
            .is_some_and(|old| Rc::ptr_eq(old, &keyboard))
        {
            self.keys_layer.children().clear();
            self.views.borrow_mut().clear();
            let mut keys = keyboard.keys();
            while let Some(key) = keys.try_next().map_err(EngineError::from)? {
                let view = KeyView::new(key.clone());
                view.set_activation_owner(self.to_ref().downgrade());
                {
                    let mut views = self.views.borrow_mut();
                    if views.insert(KeyIdentity(key), view.clone()).is_none() {
                        self.views_version
                            .set(self.views_version.get().wrapping_add(1));
                    }
                }
                self.keys_layer.children().add(view);
            }
            *self.rendered_keyboard.borrow_mut() = Some(keyboard.clone());
            force = true;
        }
        if !force && self.rendered_version.get() == keyboard.version() {
            return Ok(());
        }
        self.rendered_version.set(keyboard.version());
        let version = self.views_version.get();
        let keys: Vec<_> = self.views.borrow().keys().cloned().collect();
        for key in keys {
            if version != self.views_version.get() {
                return Err(KeysViewError::ViewsCollectionModified);
            }
            let view = self.views.borrow().get(&key).cloned();
            if let Some(view) = view {
                self.update_view(&view)?;
            }
        }
        if version != self.views_version.get() {
            return Err(KeysViewError::ViewsCollectionModified);
        }
        Ok(())
    }
    fn update_view(&self, view: &KeyView) -> Result<(), KeysViewError> {
        let theme = self.theme.borrow().clone();
        view.update(
            &theme,
            self.engine.mode(),
            self.engine.shift_state(),
            self.font_scale(),
            self.label_font_family(),
        )?;
        Ok(())
    }
    pub(super) fn invoke_key(&self, view: &KeyView) -> Result<(), ElementNotEnabledError> {
        let key = view.key();
        if !self.is_active_key(view) {
            return Err(ElementNotEnabledError::with_message("Key is not active."));
        }
        let keyboard = self.engine.keyboard().map_err(|error| {
            ElementNotEnabledError::with_message(format!("Cannot activate keyboard key: {error}"))
        })?;
        let current = self
            .rendered_keyboard
            .borrow()
            .as_ref()
            .is_some_and(|rendered| Rc::ptr_eq(rendered, &keyboard));
        if !current {
            return Err(ElementNotEnabledError::with_message(
                "Displayed keyboard has changed.",
            ));
        }
        let mut keys = keyboard.keys();
        let mut found = false;
        while let Some(current) = keys.try_next().map_err(|error| {
            ElementNotEnabledError::with_message(format!("Cannot activate keyboard key: {error:?}"))
        })? {
            if Rc::ptr_eq(&current, &key) {
                found = true;
                break;
            }
        }
        // Row providers can reenter. Release their enumeration before querying current state,
        // then verify membership, the displayed frame and scene ownership before editing.
        drop(keys);
        let latest = self.engine.keyboard().map_err(|error| {
            ElementNotEnabledError::with_message(format!("Cannot activate keyboard key: {error}"))
        })?;
        if !found || !Rc::ptr_eq(&keyboard, &latest) || !self.is_active_key(view) {
            return Err(ElementNotEnabledError::with_message("Key is not active."));
        }
        // Use the source's atomic full-key path and current computed data. Do not manufacture
        // pointer IDs, capture, repeat/long-press timers, visual press events or a cached target.
        self.engine.input_key(key.data().as_ref()).map_err(|error| {
            ElementNotEnabledError::with_message(format!("Cannot activate keyboard key: {error}"))
        })
    }
    fn is_active_key(&self, view: &KeyView) -> bool {
        let key = view.key();
        self.is_attached_to_visual_tree()
            && self.is_effectively_enabled()
            && view.is_effectively_visible()
            && key.is_visible()
            && key.is_enabled()
            && view
                .visual_parent()
                .and_then(|parent| parent.cast::<Canvas>())
                .as_ref()
                == Some(&self.keys_layer)
    }
    fn set_pressed(&self, key: &Rc<ComputedKey>, pressed: bool) -> Result<(), KeysViewError> {
        let view = self.views.borrow().get(&KeyIdentity(key.clone())).cloned();
        if let Some(view) = view {
            view.set_is_pressed(pressed);
            self.update_view(&view)?;
        }
        let handlers = if pressed {
            &self.key_pressed_visual
        } else {
            &self.key_released_visual
        };
        for (_, handler) in handlers.snapshot().iter() {
            handler(self, key);
        }
        Ok(())
    }
    fn pointer_down(&self, e: &PointerPressedEventArgs) -> Result<(), KeysViewError> {
        e.set_handled(true);
        let pointer = e.pointer();
        let this = self.to_ref().upcast::<InputElement>();
        pointer.capture(Some(&this));
        if pointer.captured().as_ref() == Some(&this) {
            self.captured_pointers.borrow_mut().insert(pointer.id());
        }
        // Source ManipulationMode.None reserves these contacts for the keyboard state
        // machine. Native recognizers also see handled events, including on ancestors.
        e.prevent_gesture_recognition();
        let p = e.get_position(Some(self));
        let timer = self.trail_fade_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.cancel();
        }
        self.touch.pointer_down(i64::from(pointer.id()), p.x, p.y)?;
        Ok(())
    }
    fn pointer_move(&self, e: &PointerEventArgs) -> Result<(), KeysViewError> {
        let props = e.properties();
        // Native touch/pen contact is represented by button flags; explicit IsInContact is
        // absent at this pin. Captured pointers follow the unconditional source path.
        let contact = props.is_left_button_pressed
            || props.is_right_button_pressed
            || props.is_middle_button_pressed
            || props.is_x_button_1_pressed
            || props.is_x_button_2_pressed;
        if !self.captured_pointers.borrow().contains(&e.pointer().id()) && !contact {
            return Ok(());
        }
        e.set_handled(true);
        // FerroUI supplies chronological previous points followed by current, unlike Uno's
        // newest-first collection. Convert positions in the framework, including transforms.
        for p in e.get_intermediate_points(Some(self)) {
            self.touch
                .pointer_move(i64::from(e.pointer().id()), p.position.x, p.position.y)?;
        }
        Ok(())
    }
    fn pointer_up(&self, e: &PointerReleasedEventArgs) -> Result<(), KeysViewError> {
        e.set_handled(true);
        let p = e.get_position(Some(self));
        let pointer = e.pointer();
        self.captured_pointers.borrow_mut().remove(&pointer.id());
        if pointer.captured().as_ref() == Some(&self.to_ref().upcast::<InputElement>()) {
            pointer.capture(None);
        }
        self.touch.pointer_up(i64::from(pointer.id()), p.x, p.y)?;
        Ok(())
    }
    fn pointer_cancel(&self, pointer: &Rc<dyn IPointer>) -> Result<(), KeysViewError> {
        let removed = self.captured_pointers.borrow_mut().remove(&pointer.id());
        if removed {
            self.touch.pointer_cancel(i64::from(pointer.id()))?;
        }
        Ok(())
    }
    fn update_trail(&self) {
        let trail = self.touch.glide_trail();
        let trail = trail.borrow();
        let points = Points::from_items(
            trail
                .iter()
                .skip(trail.len().saturating_sub(80))
                .map(|p| Point::new(p.x, p.y)),
        );
        let visible = trail.len() > 1;
        drop(trail);
        self.trail.set_points(Some(points));
        self.trail.set_opacity(0.75);
        self.trail.set_is_visible(visible);
    }
    fn schedule_trail_fade(&self) {
        let timer = self.trail_fade_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.cancel();
        }
        let interval = Duration::from_millis(
            ((self.engine.settings().glide_trail_duration().max(0) / 6).max(1)) as u64,
        );
        self.schedule_fade_step(interval, Rc::new(Cell::new(0)));
    }
    fn schedule_fade_step(&self, interval: Duration, step: Rc<Cell<i32>>) {
        let weak = self.to_ref().downgrade();
        let action = Rc::new(move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let n = step.get() + 1;
            step.set(n);
            if n >= 6 {
                this.touch.clear_glide_trail();
                this.trail.set_is_visible(false);
            } else {
                this.trail.set_opacity(0.75 * (1.0 - f64::from(n) / 6.0));
                this.schedule_fade_step(interval, step.clone());
            }
        });
        let timer = self.scheduler.schedule(interval, action);
        let previous = self.trail_fade_timer.replace(Some(timer));
        drop(previous);
    }
}
impl Drop for KeysView {
    fn drop(&mut self) {
        self.cancel_trail_fade();
        self.release_subscriptions();
    }
}
#[cfg(test)]
#[path = "keys_view_tests.rs"]
mod tests;
