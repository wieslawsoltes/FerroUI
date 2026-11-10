//! Derived from ScreenKeyboard.Uno/Primitives/TouchButton.cs (Apache-2.0).
//! Scheduled work has weak receivers; native Drop cancels the owned timer. Live press/repeat
//! ordering is unchanged, and tree removal uses the framework's capture-loss cancellation.
use crate::icons::{IconError, IconFactory};
use ferroboard_core::input::ScheduledAction;
use ferroboard_core::{input::IKeyboardScheduler, keys::KeyIcon};
use ferroui_base::{
    BoxedValue, CornerRadius, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
    Thickness, VisualImpl, ferro_class, ferro_impl_classes,
    input::{
        IPointer, InputElement, InputElementImpl, PointerPressedEventArgs, PointerReleasedEventArgs,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment},
    media::{Color, FontFamily, IBrush, SolidColorBrush, TextTrimming, TextWrapping},
};
use ferroui_controls::{
    Border, Control, ControlImpl, Grid, Panel, PanelImpl, TextBlock,
    automation::AutomationProperties,
};
use ferroui_system::events::HandlerList;
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

ferroui_base::ferro_thread_local! {
    static BRUSHES: RefCell<HashMap<Color, Ref<SolidColorBrush>>> = RefCell::new(HashMap::new());
    // The source Transparent field is a distinct instance from Get(transparent).
    static TRANSPARENT: Ref<SolidColorBrush> = SolidColorBrush::with_color(Color::default());
}
pub(crate) struct Brushes;
impl Brushes {
    pub fn get(color: Color) -> Ref<SolidColorBrush> {
        BRUSHES.with(|cache| {
            cache
                .borrow_mut()
                .entry(color)
                .or_insert_with(|| SolidColorBrush::with_color(color))
                .clone()
        })
    }
    pub fn transparent() -> Ref<SolidColorBrush> {
        TRANSPARENT.with(Clone::clone)
    }
}

/// Native, nonfocusable Grid primitive with the source repeat/long-press state machine.
/// Full focus-neutral descendant behavior and markup registration remain incomplete
/// (docs/native-api.json).
#[repr(C)]
pub struct TouchButton {
    base: Grid,
    content: Ref<Grid>,
    decoration: Ref<Border>,
    scheduler: RefCell<Option<Rc<dyn IKeyboardScheduler>>>,
    timer: RefCell<Option<ScheduledAction>>,
    pressed: Cell<bool>,
    long_pressed: Cell<bool>,
    pointer_id: Cell<Option<i32>>,
    background: Cell<Color>,
    pressed_background: Cell<Color>,
    repeat_delay: Cell<Option<Duration>>,
    repeat_interval: Cell<Duration>,
    long_press_delay: Cell<Duration>,
    value: RefCell<Option<BoxedValue>>,
    click: HandlerList<dyn Fn(&TouchButton)>,
    long_press: HandlerList<dyn Fn(&TouchButton)>,
}
ferro_class!(TouchButton: Grid);
ferroui_base::ferro_class_info!(TouchButton {
    new: TouchButton::new
});
ferro_impl_classes!(TouchButton: StyledElementImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl, PanelImpl);
impl ControlImpl for TouchButton {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::touch_button_automation_peer::TouchButtonAutomationPeer::new(this).upcast()
    }
}
impl FerroObjectImpl for TouchButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // Native Grid lacks Uno's corner/border/padding properties. A native Border paints
        // and pads the existing content Grid; the inherited background property stays authoritative.
        this.decoration.set_child(this.content.clone());
        this.children().add(this.decoration.clone());
        let weak = this.to_ref().downgrade();
        this.property_changed(move |e| {
            if e.property() == Panel::background_property().as_property()
                && let Some(this) = weak.upgrade()
            {
                this.decoration.set_background(this.background());
            }
        });
        this.set_is_tab_stop(false);
        this.set_allow_focus_on_interaction(false);
        this.set_background(Some(Brushes::transparent().into()));
        let weak = this.to_ref().downgrade();
        this.pointer_pressed(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.pressed(e);
            }
        });
        let weak = this.to_ref().downgrade();
        this.pointer_released(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.released(e);
            }
        });
        let weak = this.to_ref().downgrade();
        this.pointer_capture_lost(move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.reset(e.pointer());
            }
        });
        // FerroUI reports cancellation through pointer capture loss; there is no separate
        // PointerCanceled routed event. Native cancellation delivery remains platform-unverified.
        let weak = this.to_ref().downgrade();
        this.pointer_exited(move |_, _| {
            if let Some(this) = weak.upgrade()
                && this.pointer_id.get().is_none()
            {
                this.pressed.set(false);
                this.update_background();
            }
        });
    }
}
impl VisualImpl for TouchButton {
    fn render(_this: &Self, _context: &mut ferroui_base::media::DrawingContext) {
        // Painting is performed by the decoration Border child. Do not paint Panel's square
        // background underneath it: that would fill the rounded transparent corners.
    }
}
impl TouchButton {
    pub fn construct() -> Self {
        Self {
            base: Grid::construct(),
            content: Grid::new(),
            decoration: Border::new(),
            scheduler: RefCell::new(None),
            timer: RefCell::new(None),
            pressed: Cell::new(false),
            long_pressed: Cell::new(false),
            pointer_id: Cell::new(None),
            background: Cell::new(Color::default()),
            pressed_background: Cell::new(Color::default()),
            repeat_delay: Cell::new(None),
            repeat_interval: Cell::new(Duration::from_millis(50)),
            long_press_delay: Cell::new(Duration::from_millis(450)),
            value: RefCell::new(None),
            click: HandlerList::new(),
            long_press: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn corner_radius(&self) -> CornerRadius {
        self.decoration.corner_radius()
    }
    pub fn set_corner_radius(&self, value: CornerRadius) {
        self.decoration.set_corner_radius(value);
    }
    pub fn padding(&self) -> Thickness {
        self.decoration.padding()
    }
    pub fn set_padding(&self, value: Thickness) {
        self.decoration.set_padding(value);
    }
    pub fn border_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.decoration.border_brush()
    }
    pub fn set_border_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.decoration.set_border_brush(value);
    }
    pub fn border_thickness(&self) -> Thickness {
        self.decoration.border_thickness()
    }
    pub fn set_border_thickness(&self, value: Thickness) {
        self.decoration.set_border_thickness(value);
    }
    pub fn repeat_delay(&self) -> Option<Duration> {
        self.repeat_delay.get()
    }
    pub fn set_repeat_delay(&self, value: Option<Duration>) {
        self.repeat_delay.set(value);
    }
    pub fn repeat_interval(&self) -> Duration {
        self.repeat_interval.get()
    }
    pub fn set_repeat_interval(&self, value: Duration) {
        self.repeat_interval.set(value);
    }
    pub fn long_press_delay(&self) -> Duration {
        self.long_press_delay.get()
    }
    pub fn set_long_press_delay(&self, value: Duration) {
        self.long_press_delay.set(value);
    }
    pub fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().clone()
    }
    pub fn set_value(&self, value: Option<BoxedValue>) {
        *self.value.borrow_mut() = value;
    }
    pub fn content_grid(&self) -> Ref<Grid> {
        self.content.clone()
    }
    pub fn set_content(&self, element: Ref<Control>) {
        self.content.children().clear();
        self.content.children().add(element);
    }
    pub fn set_icon(
        &self,
        icon: KeyIcon,
        size: f64,
        foreground: Option<Rc<dyn IBrush>>,
        automation_name: Option<&str>,
    ) -> Result<(), IconError> {
        self.set_icon_value((icon).0, size, foreground, automation_name)
    }
    /// Numeric bridge for unnamed source KeyIcon values, pending the general Core enum migration.
    pub fn set_icon_value(
        &self,
        icon: i32,
        size: f64,
        foreground: Option<Rc<dyn IBrush>>,
        automation_name: Option<&str>,
    ) -> Result<(), IconError> {
        let path = IconFactory::create_value(icon, size, foreground)?;
        self.set_content(path.upcast());
        let name = IconFactory::name_value(icon);
        AutomationProperties::set_name(self, Some(automation_name.unwrap_or(&name)));
        Ok(())
    }
    pub fn set_text(
        &self,
        text: &str,
        font_size: f64,
        foreground: Option<Rc<dyn IBrush>>,
        font_family: Option<FontFamily>,
    ) -> Ref<TextBlock> {
        self.set_text_nullable(Some(text), font_size, foreground, font_family)
    }
    pub fn set_text_nullable(
        &self,
        text: Option<&str>,
        font_size: f64,
        foreground: Option<Rc<dyn IBrush>>,
        font_family: Option<FontFamily>,
    ) -> Ref<TextBlock> {
        let tb = TextBlock::new();
        tb.set_text(text);
        tb.set_font_size(font_size);
        tb.set_foreground(foreground);
        tb.set_horizontal_alignment(HorizontalAlignment::Center);
        tb.set_vertical_alignment(VerticalAlignment::Center);
        tb.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        tb.set_text_wrapping(TextWrapping::NoWrap);
        tb.set_is_hit_test_visible(false);
        if let Some(font_family) = font_family {
            tb.set_font_family(font_family);
        }
        self.set_content(tb.clone().upcast());
        AutomationProperties::set_name(self, text);
        tb
    }
    pub fn set_colors(&self, background: Color, pressed_background: Color) {
        self.background.set(background);
        self.pressed_background.set(pressed_background);
        self.update_background();
    }
    pub fn set_scheduler(&self, scheduler: Rc<dyn IKeyboardScheduler>) {
        let previous = self.scheduler.replace(Some(scheduler));
        drop(previous);
    }
    /// Registers a publisher-owned callback until explicit removal or this button's drop.
    pub fn click(&self, handler: impl Fn(&TouchButton) + 'static) -> u64 {
        self.click.add(Rc::new(handler))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_click(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.click, token)
    }
    /// Registers a publisher-owned callback until explicit removal or this button's drop.
    pub fn long_press(&self, handler: impl Fn(&TouchButton) + 'static) -> u64 {
        self.long_press.add(Rc::new(handler))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_long_press(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.long_press, token)
    }
    // Accessibility activation is one ordinary click, independent of pointer hold/repeat state.
    pub(super) fn invoke_click(&self) {
        for (_, handler) in self.click.snapshot().iter() {
            handler(self);
        }
    }
    fn update_background(&self) {
        self.set_background(Some(
            Brushes::get(if self.pressed.get() {
                self.pressed_background.get()
            } else {
                self.background.get()
            })
            .into(),
        ));
    }
    fn replace_timer(&self, timer: Option<ScheduledAction>) {
        // Caller-owned cancellation handles can reenter from Drop. Publish the new
        // handle and release storage before retiring the previous one.
        let previous = self.timer.replace(timer);
        drop(previous);
    }
    fn pressed(&self, e: &PointerPressedEventArgs) {
        e.set_handled(true);
        self.pointer_id.set(Some(e.pointer().id()));
        e.pointer()
            .capture(Some(&self.to_ref().upcast::<InputElement>()));
        self.pressed.set(true);
        self.long_pressed.set(false);
        self.update_background();
        let timer = self.timer.borrow().clone();
        if let Some(timer) = timer {
            timer.cancel();
        }
        let scheduler = self.scheduler.borrow().clone();
        let Some(scheduler) = scheduler else {
            return;
        };
        if let Some(delay) = self.repeat_delay.get() {
            let weak = self.to_ref().downgrade();
            let timer = scheduler.schedule(
                delay,
                Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.repeat();
                    }
                }),
            );
            self.replace_timer(Some(timer));
        } else if !self.long_press.is_empty() {
            let weak = self.to_ref().downgrade();
            let timer = scheduler.schedule(
                self.long_press_delay.get(),
                Rc::new(move || {
                    let Some(this) = weak.upgrade() else {
                        return;
                    };
                    if this.pressed.get() {
                        this.long_pressed.set(true);
                        for (_, handler) in this.long_press.snapshot().iter() {
                            handler(&this);
                        }
                    }
                }),
            );
            self.replace_timer(Some(timer));
        }
    }
    fn repeat(&self) {
        if !self.pressed.get() || self.scheduler.borrow().is_none() {
            return;
        }
        self.long_pressed.set(true);
        for (_, handler) in self.click.snapshot().iter() {
            handler(self);
        }
        // Read the scheduler and interval again AFTER callbacks, as source does.
        let scheduler = self
            .scheduler
            .borrow()
            .clone()
            .expect("scheduler remains installed");
        let weak = self.to_ref().downgrade();
        let timer = scheduler.schedule(
            self.repeat_interval.get(),
            Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.repeat();
                }
            }),
        );
        self.replace_timer(Some(timer));
    }
    fn released(&self, e: &PointerReleasedEventArgs) {
        e.set_handled(true);
        let was_pressed = self.pressed.get();
        self.reset(e.pointer());
        if was_pressed && !self.long_pressed.get() {
            let p = e.get_position(Some(self));
            let bounds = self.bounds();
            if p.x >= -8.0 && p.y >= -8.0 && p.x <= bounds.width + 8.0 && p.y <= bounds.height + 8.0
            {
                for (_, handler) in self.click.snapshot().iter() {
                    handler(self);
                }
            }
        }
    }
    fn reset(&self, pointer: &Rc<dyn IPointer>) {
        // WinUI ReleasePointerCapture only releases THIS element's capture. The FerroUI
        // pointer-level capture(None) is unconditional, so preserve the owner check explicitly.
        if pointer.captured().as_ref() == Some(&self.to_ref().upcast::<InputElement>()) {
            pointer.capture(None);
        }
        self.reset_core();
    }
    fn reset_core(&self) {
        self.pointer_id.set(None);
        self.pressed.set(false);
        let timer = self.timer.borrow().clone();
        if let Some(timer) = timer {
            timer.cancel();
        }
        self.replace_timer(None);
        self.update_background();
    }
}

impl Drop for TouchButton {
    fn drop(&mut self) {
        // Cleanup must not paint, emit input or attempt to recreate a Ref to a dying control.
        if let Some(timer) = self.timer.get_mut().take() {
            timer.cancel();
        }
    }
}

#[cfg(test)]
#[path = "touch_button_lifetime_tests.rs"]
mod native_lifetime_tests;
