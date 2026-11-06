//! Derived from ScreenKeyboard.Uno/ScreenKeyboardHost.cs (Apache-2.0).
use crate::{
    KeyboardControlResult, OnScreenKeyboard,
    primitives::{Brushes, FocusNeutral, source_max, source_min},
};
use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl, StyledProperty, Thickness,
    Visual, VisualImpl,
    animation::{
        Animation, Cue, FillMode, IClock, KeyFrame,
        easings::{CubicEaseOut, Easing},
    },
    data::BindingPriority,
    ferro_class, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, PointerEventArgs},
    instantiate,
    interactivity::{InteractiveImpl, RoutingStrategies},
    layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment},
    media::TranslateTransform,
    reactive::{Disposable, Observable},
    styling::Setter,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Border, Canvas, Control, ControlImpl, Grid, GridLength, PanelImpl, RowDefinition, TopLevel,
};
use ferroui_system::{
    IDisposable, Subscription, component_model::INotifyPropertyChanged, events::Event,
};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    rc::Rc,
};

// Source Math.Clamp uses min=0 and Math.Max(0,...) as max here. .NET permits a NaN
// maximum; Rust f64::clamp rejects it. Retain the source comparison order.
fn position_clamp(value: f64, maximum: f64) -> f64 {
    if value < 0.0 {
        0.0
    } else if value > maximum {
        maximum
    } else {
        value
    }
}
// Per-setter selection keeps the source's original transform target. The native default
// TransformAnimator redirects through a Visual's current RenderTransform instead.
struct HostDoubleAnimator;
impl ferroui_base::animation::ICustomAnimator for HostDoubleAnimator {
    fn create_wrapper(self: Rc<Self>) -> Rc<dyn ferroui_base::animation::IAnimator> {
        Rc::new(ferroui_base::animation::animators::DoubleAnimator::new())
    }
    fn wrapper_type(&self) -> TypeId {
        TypeId::of::<ferroui_base::animation::animators::DoubleAnimator>()
    }
}

/// Source enum values, including unnamed CLR values accepted by the property system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct KeyboardVisibilityMode(pub i32);
#[allow(non_upper_case_globals)]
impl KeyboardVisibilityMode {
    pub const Auto: Self = Self(0);
    pub const AlwaysVisible: Self = Self(1);
    pub const Manual: Self = Self(2);
}
/// Optional custom scrolling request. These source alignment/animation hints are supplied to
/// custom handlers; the default uses FerroUI's native minimal-reveal behavior.
#[derive(Clone)]
pub struct HostBringIntoViewRequest {
    pub element: Ref<Visual>,
    pub animation_desired: bool,
    pub vertical_alignment_ratio: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostIntegrationError {
    BringIntoViewOptionsUnavailable,
}
impl std::fmt::Display for HostIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for HostIntegrationError {}
/// Port capability seam for platform-specific support. This is synchronous on the UI thread;
/// errors are caught at the source's intentional platform-fallback boundary and remain observable.
pub type HostBringIntoViewHandler =
    Rc<dyn Fn(&HostBringIntoViewRequest) -> Result<(), HostIntegrationError>>;

// WinUI Storyboard's default HoldEnd does not overwrite TranslateTransform.Y's local/base value.
// Native FillMode::Forward does. Retain a separate animation-priority value until Stop instead.
struct HostStoryboard {
    stopped: Cell<bool>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    hold: RefCell<Option<Rc<dyn IDisposable>>>,
}
impl HostStoryboard {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            stopped: Cell::new(false),
            subscription: RefCell::new(None),
            hold: RefCell::new(None),
        })
    }
    fn stop(&self) {
        self.stopped.set(true);
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let hold = self.hold.borrow_mut().take();
        if let Some(hold) = hold {
            hold.dispose();
        }
    }
}
#[repr(C)]
pub struct ScreenKeyboardHost {
    base: Grid,
    keyboard: Ref<OnScreenKeyboard>,
    content_host: Ref<Grid>,
    float_layer: Ref<Canvas>,
    float_frame: Ref<Border>,
    float_grid: Ref<Grid>,
    drag_handle: Ref<Border>,
    drag_bar: Ref<Border>,
    is_floating: Cell<bool>,
    float_position: Cell<Option<Point>>,
    drag_start: Cell<Point>,
    drag_origin: Cell<Point>,
    drag_pointer: Cell<Option<i32>>,
    translate: Ref<TranslateTransform>,
    storyboard: RefCell<Option<Rc<HostStoryboard>>>,
    focused_input: RefCell<Option<Ref<Visual>>>,
    pending_close: Cell<bool>,
    native_owner_released: Cell<bool>,
    keyboard_open_changed: Event<Self>,
    subscriptions: RefCell<Vec<Subscription>>,
    animation_clock: Option<Rc<dyn IClock>>,
    bring_into_view: RefCell<Option<HostBringIntoViewHandler>>,
    integration_failed: Event<HostIntegrationError>,
}
ferro_class!(ScreenKeyboardHost:Grid);
ferro_impl_classes!(ScreenKeyboardHost:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl ScreenKeyboardHost {
    pub fn content_property()->StyledProperty<Option<Ref<Control>>> {FerroProperty::register::<Self,_>("Content",None)}
    pub fn is_keyboard_open_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsKeyboardOpen",false)}
    pub fn visibility_mode_property()->StyledProperty<KeyboardVisibilityMode> {FerroProperty::register::<Self,_>("VisibilityMode",KeyboardVisibilityMode::Auto)}
    pub fn is_animated_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsAnimated",true)}
}}
impl FerroObjectImpl for ScreenKeyboardHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.row_definitions()
            .add(RowDefinition::with_height(GridLength::STAR));
        this.row_definitions()
            .add(RowDefinition::with_height(GridLength::AUTO));
        this.keyboard.set_auto_attach(false);
        this.keyboard
            .set_render_transform(Some((&this.translate).into()));
        this.keyboard.set_is_visible(false);
        Grid::set_row(&this.content_host, 0);
        Grid::set_row(&this.keyboard, 1);
        this.children().add(this.content_host.clone());
        this.children().add(this.keyboard.clone());
        let weak = this.to_ref().downgrade();
        let token = this.keyboard.hide_requested().add(Rc::new(move |_| {
            if let Some(host) = weak.upgrade() {
                host.set_is_keyboard_open(
                    host.visibility_mode() == KeyboardVisibilityMode::AlwaysVisible,
                );
            }
        }));
        let keyboard = this.keyboard.clone();
        this.subscriptions.borrow_mut().push(
            Disposable::create(move || {
                keyboard.hide_requested().remove(token);
            })
            .into(),
        );
        Grid::set_row_span(&this.float_layer, 2);
        this.float_grid
            .row_definitions()
            .add(RowDefinition::with_height(GridLength::AUTO));
        this.float_grid
            .row_definitions()
            .add(RowDefinition::with_height(GridLength::AUTO));
        this.drag_handle.set_height(22.0);
        this.drag_bar.set_width(44.0);
        this.drag_bar.set_height(5.0);
        this.drag_bar.set_corner_radius(CornerRadius::uniform(2.5));
        this.drag_bar
            .set_horizontal_alignment(HorizontalAlignment::Center);
        this.drag_bar
            .set_vertical_alignment(VerticalAlignment::Center);
        this.drag_bar.set_is_hit_test_visible(false);
        this.drag_handle.set_child(this.drag_bar.clone());
        let weak = this.to_ref().downgrade();
        this.drag_handle.pointer_pressed(move |_, e| {
            if let Some(host) = weak.upgrade() {
                host.on_drag_pressed(e);
            }
        });
        let weak = this.to_ref().downgrade();
        this.drag_handle.pointer_moved(move |_, e| {
            if let Some(host) = weak.upgrade() {
                host.on_drag_moved(e);
            }
        });
        let weak = this.to_ref().downgrade();
        this.drag_handle.pointer_released(move |_, e| {
            if let Some(host) = weak.upgrade() {
                host.on_drag_released(e);
            }
        });
        let weak = this.to_ref().downgrade();
        this.drag_handle.pointer_capture_lost(move |_, _| {
            if let Some(host) = weak.upgrade() {
                host.drag_pointer.set(None);
            }
        });
        this.float_grid.children().add(this.drag_handle.clone());
        this.float_frame.set_child(this.float_grid.clone());
        this.float_frame
            .set_corner_radius(CornerRadius::uniform(14.0));
        this.float_frame
            .set_border_thickness(Thickness::uniform(1.0));
        this.float_layer.children().add(this.float_frame.clone());
        FocusNeutral::apply(&this.float_frame);
        this.children().add(this.float_layer.clone());
        this.float_layer.set_is_visible(false);
        let weak = this.to_ref().downgrade();
        let settings = this.keyboard.settings();
        let token = settings.property_changed().add(Rc::new(move |name| {
            if matches!(name, "Floating" | "FloatingWidthRatio")
                && let Some(host) = weak.upgrade()
            {
                host.apply_floating().expect("host floating setting");
            }
        }));
        this.subscriptions.borrow_mut().push(
            Disposable::create(move || {
                settings.property_changed().remove(token);
            })
            .into(),
        );
        let weak = this.to_ref().downgrade();
        this.size_changed(move |_, _| {
            if let Some(host) = weak.upgrade()
                && host.is_floating()
            {
                host.position_floating_frame();
            }
        });
        let weak = this.to_ref().downgrade();
        this.content_host.size_changed(move |_, _| {
            if let Some(host) = weak.upgrade()
                && host.is_keyboard_open()
            {
                let focused = host.focused_input.borrow().clone();
                if let Some(focused) = focused {
                    host.bring_into_view_later(focused);
                }
            }
        });
        let weak = this.to_ref().downgrade();
        this.loaded(move |_, _| {
            if let Some(host) = weak.upgrade() {
                host.apply_floating().expect("host loaded floating");
            }
        });
        let weak = this.to_ref().downgrade();
        this.got_focus(move |_, e| {
            if let Some(host) = weak.upgrade() {
                let element = e.source().and_then(|s| s.downcast::<Visual>().ok());
                host.on_got_focus(element).expect("host got focus");
            }
        });
        let weak = this.to_ref().downgrade();
        this.lost_focus(move |_, e| {
            if let Some(host) = weak.upgrade() {
                host.on_lost_focus(e.source().and_then(|s| s.downcast::<Visual>().ok()));
            }
        });
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(host) = weak.upgrade() {
                    host.on_pointer_pressed(e.source().and_then(|s| s.downcast::<Visual>().ok()));
                }
            },
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            true,
        );
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::content_property().as_property() {
            // Source e.OldValue is required: querying Content now only yields the new child.
            let old = change.get_old_value::<Option<Ref<Control>>>();
            let new = change.get_new_value::<Option<Ref<Control>>>();
            this.on_content_changed(old.flatten(), new);
        } else if change.property() == Self::is_keyboard_open_property().as_property() {
            this.on_is_open_changed(change.get_new_value::<bool>())
                .expect("host open changed");
        } else if change.property() == Self::visibility_mode_property().as_property() {
            this.on_visibility_mode_changed();
        }
    }
}
impl ScreenKeyboardHost {
    pub fn new() -> KeyboardControlResult<Ref<Self>> {
        Self::with_keyboard(OnScreenKeyboard::new(None)?, None)
    }
    /// Deterministic/native integration extension; source constructor creates its own keyboard.
    pub fn with_keyboard(
        keyboard: Ref<OnScreenKeyboard>,
        animation_clock: Option<Rc<dyn IClock>>,
    ) -> KeyboardControlResult<Ref<Self>> {
        Ok(instantiate(Self {
            base: Grid::construct(),
            keyboard,
            content_host: Grid::new(),
            float_layer: Canvas::new(),
            float_frame: Border::new(),
            float_grid: Grid::new(),
            drag_handle: Border::new(),
            drag_bar: Border::new(),
            is_floating: Cell::new(false),
            float_position: Cell::new(None),
            drag_start: Cell::new(Point::default()),
            drag_origin: Cell::new(Point::default()),
            drag_pointer: Cell::new(None),
            translate: TranslateTransform::new(),
            storyboard: RefCell::new(None),
            focused_input: RefCell::new(None),
            pending_close: Cell::new(false),
            native_owner_released: Cell::new(false),
            keyboard_open_changed: Event::new(),
            subscriptions: RefCell::new(Vec::new()),
            animation_clock,
            bring_into_view: RefCell::new(None),
            integration_failed: Event::new(),
        }))
    }
    // The pinned markup event invoker is add-only and discards the callable's return.
    // Retain cleanup with this native owner so explicit library release removes the handler.
    pub(crate) fn retain_markup_subscription(&self, subscription: Rc<dyn IDisposable>) {
        self.subscriptions.borrow_mut().push(subscription.into());
    }
    pub fn keyboard(&self) -> Ref<OnScreenKeyboard> {
        self.keyboard.clone()
    }
    pub fn content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::content_property())
    }
    pub fn set_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::content_property(), value);
    }
    pub fn is_keyboard_open(&self) -> bool {
        self.get_value(Self::is_keyboard_open_property())
    }
    pub fn set_is_keyboard_open(&self, value: bool) {
        self.set_value(Self::is_keyboard_open_property(), value);
    }
    pub fn visibility_mode(&self) -> KeyboardVisibilityMode {
        self.get_value(Self::visibility_mode_property())
    }
    pub fn set_visibility_mode(&self, value: KeyboardVisibilityMode) {
        self.set_value(Self::visibility_mode_property(), value);
    }
    pub fn is_animated(&self) -> bool {
        self.get_value(Self::is_animated_property())
    }
    pub fn set_is_animated(&self, value: bool) {
        self.set_value(Self::is_animated_property(), value);
    }
    pub fn keyboard_open_changed(&self) -> &Event<Self> {
        &self.keyboard_open_changed
    }
    pub fn is_floating(&self) -> bool {
        self.is_floating.get()
    }
    pub fn set_bring_into_view_handler(&self, handler: HostBringIntoViewHandler) {
        *self.bring_into_view.borrow_mut() = Some(handler);
    }
    pub fn integration_failed(&self) -> &Event<HostIntegrationError> {
        &self.integration_failed
    }
    fn stop_storyboard(&self) {
        let storyboard = self.storyboard.borrow().clone();
        if let Some(storyboard) = storyboard {
            storyboard.stop();
        }
    }
    fn apply_floating(&self) -> KeyboardControlResult<()> {
        let floating = self.keyboard.settings().floating();
        if floating == self.is_floating()
            && (!floating
                || self
                    .float_grid
                    .children()
                    .contains(&self.keyboard.clone().upcast::<Control>()))
        {
            if floating {
                self.position_floating_frame();
            }
            return Ok(());
        }
        self.is_floating.set(floating);
        self.stop_storyboard();
        self.translate.set_y(0.0);
        if floating {
            self.children().remove(self.keyboard.clone());
            Grid::set_row(&self.keyboard, 1);
            self.float_grid.children().add(self.keyboard.clone());
            let theme = self.keyboard.effective_theme();
            self.float_frame
                .set_background(Some(Brushes::get(theme.background()).into()));
            self.float_frame
                .set_border_brush(Some(Brushes::get(theme.popup_border_color()).into()));
            self.drag_bar
                .set_background(Some(Brushes::get(theme.key_hint_foreground()).into()));
            self.drag_handle
                .set_background(Some(Brushes::get(theme.smartbar_background()).into()));
            self.keyboard.set_is_visible(true);
            self.float_layer.set_is_visible(self.is_keyboard_open());
            self.position_floating_frame();
        } else {
            self.float_grid.children().remove(self.keyboard.clone());
            Grid::set_row(&self.keyboard, 1);
            self.children().insert(1, self.keyboard.clone());
            self.float_layer.set_is_visible(false);
            self.keyboard
                .set_width(f64::from_bits(0xfff8_0000_0000_0000));
            self.keyboard.set_is_visible(self.is_keyboard_open());
        }
        Ok(())
    }
    fn position_floating_frame(&self) {
        let actual = self.bounds();
        if actual.width <= 0.0 {
            return;
        }
        let width = source_max(
            320.0,
            source_min(
                actual.width,
                actual.width * self.keyboard.settings().floating_width_ratio(),
            ),
        );
        self.keyboard.set_width(width);
        self.float_frame.set_width(width + 2.0);
        self.float_frame.update_layout();
        let height = if self.float_frame.bounds().height > 0.0 {
            self.float_frame.bounds().height
        } else {
            320.0
        };
        let position = self.float_position.get().unwrap_or_else(|| {
            Point::new((actual.width - width) / 2.0, actual.height - height - 16.0)
        });
        let x = position_clamp(position.x, source_max(0.0, actual.width - width - 2.0));
        let y = position_clamp(position.y, source_max(0.0, actual.height - height));
        Canvas::set_left(&self.float_frame, x);
        Canvas::set_top(&self.float_frame, y);
    }
    fn on_drag_pressed(&self, e: &PointerEventArgs) {
        self.drag_pointer.set(Some(e.pointer().id()));
        self.drag_start.set(e.get_position(Some(self)));
        self.drag_origin.set(Point::new(
            Canvas::get_left(&self.float_frame),
            Canvas::get_top(&self.float_frame),
        ));
        e.pointer()
            .capture(Some(&self.drag_handle.to_ref().upcast::<InputElement>()));
        e.set_handled(true);
    }
    fn on_drag_moved(&self, e: &PointerEventArgs) {
        if self.drag_pointer.get() != Some(e.pointer().id()) {
            return;
        }
        let p = e.get_position(Some(self));
        let origin = self.drag_origin.get();
        let start = self.drag_start.get();
        self.float_position.set(Some(Point::new(
            origin.x + p.x - start.x,
            origin.y + p.y - start.y,
        )));
        self.position_floating_frame();
        e.set_handled(true);
    }
    fn on_drag_released(&self, e: &PointerEventArgs) {
        self.drag_pointer.set(None);
        if e.pointer().captured().as_ref()
            == Some(&self.drag_handle.to_ref().upcast::<InputElement>())
        {
            e.pointer().capture(None);
        }
    }
    fn on_content_changed(&self, old: Option<Ref<Control>>, new: Option<Ref<Control>>) {
        if let Some(old) = old {
            self.content_host.children().remove(old);
        }
        if let Some(new) = new {
            self.content_host.children().add(new);
        }
    }
    fn on_visibility_mode_changed(&self) {
        if self.visibility_mode() == KeyboardVisibilityMode::AlwaysVisible {
            self.set_is_keyboard_open(true);
        }
    }
    fn on_got_focus(&self, element: Option<Ref<Visual>>) -> KeyboardControlResult<()> {
        if self.native_owner_released.get() {
            return Ok(());
        }
        if let Some(element) = element
            && OnScreenKeyboard::is_text_input(Some(&element))
            && !self.is_inside_keyboard(&element)
        {
            self.pending_close.set(false);
            *self.focused_input.borrow_mut() = Some(element.clone());
            self.keyboard.attach_to(Some(element.clone()))?;
            if self.visibility_mode() != KeyboardVisibilityMode::Manual {
                self.set_is_keyboard_open(true);
            }
            self.bring_into_view_later(element);
        }
        Ok(())
    }
    fn on_lost_focus(&self, element: Option<Ref<Visual>>) {
        if self.native_owner_released.get() {
            return;
        }
        if element.is_none() || element != *self.focused_input.borrow() {
            return;
        }
        self.pending_close.set(true);
        let weak = self.to_ref().downgrade();
        Dispatcher::current_dispatcher().post_local(
            move || {
                if let Some(host) = weak.upgrade() {
                    host.close_after_focus_move()
                        .expect("host deferred focus close");
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
    fn close_after_focus_move(&self) -> KeyboardControlResult<()> {
        if self.native_owner_released.get() {
            return Ok(());
        }
        if !self.pending_close.get() {
            return Ok(());
        }
        let root = TopLevel::get_top_level(Some(self));
        let focused = root
            .as_ref()
            .and_then(|root| root.focus_manager().get_focused_element())
            .map(|e| e.upcast::<Visual>())
            .filter(|e| TopLevel::get_top_level(Some(e)) == root);
        if focused.as_ref().is_some_and(|e| {
            self.is_inside_keyboard(e)
                || (Self::is_descendant_of(e, &self.content_host)
                    && OnScreenKeyboard::is_text_input(Some(e)))
        }) {
            return Ok(());
        }
        *self.focused_input.borrow_mut() = None;
        self.keyboard.detach()?;
        if self.visibility_mode() == KeyboardVisibilityMode::Auto {
            self.set_is_keyboard_open(false);
        }
        Ok(())
    }
    fn on_pointer_pressed(&self, source: Option<Ref<Visual>>) {
        let focused = self.focused_input.borrow().clone();
        if !self.is_keyboard_open()
            && self.visibility_mode() == KeyboardVisibilityMode::Auto
            && let (Some(source), Some(focused)) = (source, focused)
            && Self::is_descendant_of(&source, &focused)
        {
            self.set_is_keyboard_open(true);
        }
    }
    fn is_inside_keyboard(&self, element: &Visual) -> bool {
        Self::is_descendant_of(element, &self.keyboard)
    }
    fn is_descendant_of(element: &Visual, ancestor: &Visual) -> bool {
        let mut current = Some(element.to_ref());
        while let Some(element) = current {
            if std::ptr::eq(&*element, ancestor) {
                return true;
            }
            current = element.visual_parent();
        }
        false
    }
    fn bring_into_view_later(&self, element: Ref<Visual>) {
        let weak = self.to_ref().downgrade();
        Dispatcher::current_dispatcher().post_local(
            move || {
                let Some(host) = weak.upgrade() else {
                    return;
                };
                if host.native_owner_released.get() {
                    return;
                }
                let handler = host.bring_into_view.borrow().clone();
                let request = HostBringIntoViewRequest {
                    element,
                    animation_desired: true,
                    vertical_alignment_ratio: 0.5,
                };
                if let Some(handler) = handler {
                    if let Err(error) = handler(&request) {
                        host.integration_failed.raise(&error);
                    }
                } else {
                    host.reveal_focused_input(&request.element);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn reveal_focused_input(&self, element: &Ref<Visual>) {
        if !self.is_keyboard_open()
            || self.focused_input.borrow().as_ref() != Some(element)
            || !Self::is_descendant_of(element, &self.content_host)
        {
            return;
        }
        let Some(root) = TopLevel::get_top_level(Some(self)) else {
            return;
        };
        if TopLevel::get_top_level(Some(element)) != Some(root.clone())
            || !root
                .focus_manager()
                .get_focused_element()
                .is_some_and(|focused| Self::is_descendant_of(&focused.upcast(), element))
        {
            return;
        }
        if let Some(control) = element.cast::<Control>() {
            // FerroUI routes this through ancestor scroll presenters and defers invalid layout.
            control.bring_into_view();
        }
    }
    fn on_is_open_changed(&self, open: bool) -> KeyboardControlResult<()> {
        self.stop_storyboard();
        if self.is_floating() {
            self.float_layer.set_is_visible(open);
            if open {
                self.position_floating_frame();
            } else {
                self.keyboard.touch_processor().cancel_all()?;
            }
            self.keyboard_open_changed.raise(self);
            return Ok(());
        }
        if open {
            self.keyboard.set_is_visible(true);
            if self.is_animated() {
                self.keyboard.update_layout();
                let height = if self.keyboard.bounds().height > 0.0 {
                    self.keyboard.bounds().height
                } else {
                    300.0
                };
                self.animate(height, 0.0, None);
            }
            let focused = self.focused_input.borrow().clone();
            if let Some(focused) = focused {
                self.bring_into_view_later(focused);
            }
        } else {
            self.keyboard.touch_processor().cancel_all()?;
            if self.is_animated() && self.keyboard.bounds().height > 0.0 {
                let weak = self.to_ref().downgrade();
                self.animate(
                    0.0,
                    self.keyboard.bounds().height,
                    Some(Rc::new(move || {
                        if let Some(host) = weak.upgrade() {
                            host.keyboard.set_is_visible(host.is_keyboard_open());
                        }
                    })),
                );
            } else {
                self.keyboard.set_is_visible(false);
            }
        }
        self.keyboard_open_changed.raise(self);
        Ok(())
    }
    fn animate(&self, from: f64, to: f64, completed: Option<Rc<dyn Fn()>>) {
        let animation = Animation::new();
        animation.set_duration(std::time::Duration::from_millis(180).into());
        animation.set_easing(Easing::new(CubicEaseOut));
        animation.set_fill_mode(FillMode::None);
        for (cue, value) in [(0.0, from), (1.0, to)] {
            let setter: Rc<dyn ferroui_base::animation::IAnimationSetter> =
                Setter::new(TranslateTransform::y_property(), value);
            Animation::set_animator(&setter, Rc::new(HostDoubleAnimator));
            animation
                .children()
                .add(KeyFrame::with_cue(Cue::new(cue), [setter]));
        }
        let storyboard = HostStoryboard::new();
        *self.storyboard.borrow_mut() = Some(storyboard.clone());
        let state = storyboard.clone();
        let translate = self.translate.clone();
        let completion = Rc::new(move || {
            if state.stopped.get() {
                return;
            }
            let hold = translate.set_value_with_priority(
                TranslateTransform::y_property(),
                to,
                BindingPriority::Animation,
            );
            // Applying the terminal value can synchronously reenter Stop via a native property
            // observer. The new value's token does not exist until SetValue returns.
            if state.stopped.get() {
                if let Some(hold) = hold {
                    hold.dispose();
                }
                return;
            }
            *state.hold.borrow_mut() = hold;
            if let Some(completed) = &completed {
                completed();
            }
        });
        let subscription = animation.apply(
            &self.translate,
            self.animation_clock.clone(),
            Observable::return_(true),
            Some(completion),
            true,
        );
        *storyboard.subscription.borrow_mut() = Some(subscription);
    }
    /// Release only resources owned by this host. A retained keyboard remains independently usable.
    fn release_host_subscriptions(&self) {
        // Queued native work holds weak host references and cannot restart a released owner.
        self.native_owner_released.set(true);
        self.pending_close.set(false);
        *self.focused_input.borrow_mut() = None;
        self.stop_storyboard();
        *self.storyboard.borrow_mut() = None;
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
    /// Explicit shutdown of both host and keyboard. Dropping only the host removes its own hooks.
    pub fn release_native_subscriptions(&self) -> KeyboardControlResult<()> {
        self.release_host_subscriptions();
        self.keyboard.release_native_subscriptions()
    }
}
impl Drop for ScreenKeyboardHost {
    fn drop(&mut self) {
        self.release_host_subscriptions();
    }
}
#[cfg(test)]
#[path = "screen_keyboard_host_tests.rs"]
mod tests;
