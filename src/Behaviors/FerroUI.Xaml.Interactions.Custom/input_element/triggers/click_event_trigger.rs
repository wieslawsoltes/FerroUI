//! Port of `InputElement/Triggers/ClickEventTrigger.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, PointerCaptureLostEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StaticType,
    StyledElementImpl, StyledProperty, Visual, WeakRef,
};
use ferroui_controls::primitives::FlyoutBase;
use ferroui_controls::{Button, ClickMode, Control, TextBox, TopLevel};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The handlers the trigger added to its resolved source control (the reference removes its
/// delegates; a closure has no identity, so the tokens of the handlers are kept).
#[derive(Clone, Copy)]
struct InputHandlers {
    pointer_pressed: RoutedEventHandlerToken,
    pointer_released: RoutedEventHandlerToken,
    pointer_capture_lost: RoutedEventHandlerToken,
    key_down: RoutedEventHandlerToken,
    key_up: RoutedEventHandlerToken,
    lost_focus: RoutedEventHandlerToken,
}

/// Trigger that emulates [`Button`] click semantics on any [`Control`].
#[repr(C)]
pub struct ClickEventTrigger {
    base: StyledElementTriggerOfT,
    is_pressed: Cell<bool>,
    owns_pointer_capture: Cell<bool>,
    // Deviation (DEVIATIONS.md, Behaviors): the reference holds the resolved source control and the
    // root input element strongly; here they are weak references, as the source control is usually
    // the associated object, which owns the trigger.
    resolved_source_control: RefCell<Option<WeakRef<Control>>>,
    input_handlers: Cell<Option<InputHandlers>>,
    root_input_element: RefCell<Option<(WeakRef<InputElement>, RoutedEventHandlerToken)>>,
}

ferro_class!(ClickEventTrigger: StyledElementTriggerOfT);
ferro_class_info!(ClickEventTrigger {
    new: ClickEventTrigger::new,
    markup: {
        property_attributes: [
            SourceControl: [ResolveByName],
            Flyout: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ClickEventTrigger: StyledElementImpl);

ferro_properties! {
    impl ClickEventTrigger {
        /// Identifies the [`source_control`](ClickEventTrigger::source_control) property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the trigger does not own the control, so the
        /// value is an element reference.
        pub fn source_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<ClickEventTrigger, _>("SourceControl", None)
        }

        /// Identifies the [`click_mode`](ClickEventTrigger::click_mode) property.
        pub fn click_mode_property() -> StyledProperty<ClickMode> {
            FerroProperty::register::<ClickEventTrigger, _>("ClickMode", ClickMode::Release)
        }

        /// Identifies the [`routing_strategies`](ClickEventTrigger::routing_strategies) property.
        pub fn routing_strategies_property() -> StyledProperty<RoutingStrategies> {
            FerroProperty::register::<ClickEventTrigger, _>("RoutingStrategies", RoutingStrategies::TUNNEL)
        }

        /// Identifies the [`key_modifiers`](ClickEventTrigger::key_modifiers) property.
        pub fn key_modifiers_property() -> StyledProperty<Option<KeyModifiers>> {
            FerroProperty::register::<ClickEventTrigger, _>("KeyModifiers", None)
        }

        /// Identifies the [`is_default`](ClickEventTrigger::is_default) property.
        pub fn is_default_property() -> StyledProperty<bool> {
            FerroProperty::register::<ClickEventTrigger, _>("IsDefault", false)
        }

        /// Identifies the [`is_cancel`](ClickEventTrigger::is_cancel) property.
        pub fn is_cancel_property() -> StyledProperty<bool> {
            FerroProperty::register::<ClickEventTrigger, _>("IsCancel", false)
        }

        /// Identifies the [`flyout`](ClickEventTrigger::flyout) property.
        pub fn flyout_property() -> StyledProperty<Option<Ref<FlyoutBase>>> {
            FerroProperty::register::<ClickEventTrigger, _>("Flyout", None)
        }

        /// Identifies the [`use_attached_flyout`](ClickEventTrigger::use_attached_flyout) property.
        pub fn use_attached_flyout_property() -> StyledProperty<bool> {
            FerroProperty::register::<ClickEventTrigger, _>("UseAttachedFlyout", true)
        }

        /// Identifies the [`handle_event`](ClickEventTrigger::handle_event) property.
        pub fn handle_event_property() -> StyledProperty<bool> {
            FerroProperty::register::<ClickEventTrigger, _>("HandleEvent", true)
        }

        /// Identifies the [`handled_events_too`](ClickEventTrigger::handled_events_too) property.
        pub fn handled_events_too_property() -> StyledProperty<bool> {
            FerroProperty::register::<ClickEventTrigger, _>("HandledEventsToo", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ClickEventTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let property = change.property();
            if property == Self::source_control_property().as_property() {
                this.set_resolved_source(this.resolve_source_control(), false);
            } else if property == Self::routing_strategies_property().as_property() {
                this.set_resolved_source(this.resolve_source_control(), true);
            } else if property == Self::handle_event_property().as_property() {
                this.set_resolved_source(this.resolve_source_control(), true);
            } else if property == Self::handled_events_too_property().as_property() {
                this.set_resolved_source(this.resolve_source_control(), true);
            } else if property == Self::is_default_property().as_property()
                || property == Self::is_cancel_property().as_property()
            {
                this.update_root_subscription(false);
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ClickEventTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            this.set_resolved_source(this.resolve_source_control(), false);
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.detach_root_subscription();
            this.set_resolved_source(None, false);

            this.is_pressed.set(false);
            this.owns_pointer_capture.set(false);
        }
    }
}

impl ClickEventTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE),
            is_pressed: Cell::new(false),
            owns_pointer_capture: Cell::new(false),
            resolved_source_control: RefCell::new(None),
            input_handlers: Cell::new(None),
            root_input_element: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the source control from which this trigger listens for click semantics. If
    /// not set, it defaults to the associated object. Markup resolves a name given for it to the
    /// named element.
    pub fn source_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::source_control_property()))
    }

    pub fn set_source_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::source_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets how this trigger reacts to pointer and keyboard clicks.
    pub fn click_mode(&self) -> ClickMode {
        self.get_value(Self::click_mode_property())
    }

    pub fn set_click_mode(&self, value: ClickMode) {
        self.set_value(Self::click_mode_property(), value)
    }

    /// Gets or sets the routed event strategies used when subscribing to pointer and keyboard
    /// input events.
    pub fn routing_strategies(&self) -> RoutingStrategies {
        self.get_value(Self::routing_strategies_property())
    }

    pub fn set_routing_strategies(&self, value: RoutingStrategies) {
        self.set_value(Self::routing_strategies_property(), value)
    }

    /// Gets or sets required key modifiers for click execution.
    pub fn key_modifiers(&self) -> Option<KeyModifiers> {
        self.get_value(Self::key_modifiers_property())
    }

    pub fn set_key_modifiers(&self, value: Option<KeyModifiers>) {
        self.set_value(Self::key_modifiers_property(), value)
    }

    /// Gets or sets whether Enter on the visual root should invoke this trigger.
    pub fn is_default(&self) -> bool {
        self.get_value(Self::is_default_property())
    }

    pub fn set_is_default(&self, value: bool) {
        self.set_value(Self::is_default_property(), value)
    }

    /// Gets or sets whether Escape on the visual root should invoke this trigger.
    pub fn is_cancel(&self) -> bool {
        self.get_value(Self::is_cancel_property())
    }

    pub fn set_is_cancel(&self, value: bool) {
        self.set_value(Self::is_cancel_property(), value)
    }

    /// Gets or sets an explicit flyout instance used for toggle-on-click behavior. Markup resolves
    /// a name given for it to the named flyout.
    pub fn flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::flyout_property())
    }

    pub fn set_flyout(&self, value: Option<Ref<FlyoutBase>>) {
        self.set_value(Self::flyout_property(), value)
    }

    /// Gets or sets whether to fallback to the attached flyout of the source control
    /// ([`FlyoutBase::get_attached_flyout`]) when [`flyout`](Self::flyout) is not set.
    pub fn use_attached_flyout(&self) -> bool {
        self.get_value(Self::use_attached_flyout_property())
    }

    pub fn set_use_attached_flyout(&self, value: bool) {
        self.set_value(Self::use_attached_flyout_property(), value)
    }

    /// Gets or sets whether handled semantics should be applied to routed events.
    pub fn handle_event(&self) -> bool {
        self.get_value(Self::handle_event_property())
    }

    pub fn set_handle_event(&self, value: bool) {
        self.set_value(Self::handle_event_property(), value)
    }

    /// Gets or sets whether this trigger should receive already handled routed events.
    pub fn handled_events_too(&self) -> bool {
        self.get_value(Self::handled_events_too_property())
    }

    pub fn set_handled_events_too(&self, value: bool) {
        self.set_value(Self::handled_events_too_property(), value)
    }

    /// The resolved source control, while it is alive.
    fn resolved_source(&self) -> Option<Ref<Control>> {
        self.resolved_source_control.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// The resolved source control, if it is effectively enabled.
    fn enabled_resolved_source(&self) -> Option<Ref<Control>> {
        self.resolved_source().filter(|source_control| source_control.is_effectively_enabled())
    }

    fn on_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let Some(source_control) = self.enabled_resolved_source() else {
            return;
        };
        let source_visual: &Visual = &source_control;
        if !e.get_current_point(Some(source_visual)).properties.is_left_button_pressed {
            return;
        }

        if self.is_flyout_open_for_resolved_source(self.try_get_resolved_flyout().as_ref()) {
            self.set_handled(e);
            self.execute_click(e.key_modifiers());
            return;
        }

        self.is_pressed.set(true);

        if self.should_capture_pointer(&source_control) {
            let element: Ref<InputElement> = source_control.clone().upcast();
            e.pointer().capture(Some(&element));
            self.owns_pointer_capture.set(true);
        }

        self.set_handled(e);

        if self.click_mode() == ClickMode::Press {
            self.execute_click(e.key_modifiers());
        }
    }

    fn on_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if self.is_pressed.get() && e.initial_press_mouse_button() == MouseButton::Left {
            self.is_pressed.set(false);

            if self.owns_pointer_capture.get() {
                e.pointer().capture(None);
                self.owns_pointer_capture.set(false);
            }

            self.set_handled(e);

            if self.click_mode() == ClickMode::Release && self.is_pointer_within_resolved_source(e) {
                self.execute_click(e.key_modifiers());
            }
        }
    }

    fn on_pointer_capture_lost(&self, _e: &PointerCaptureLostEventArgs) {
        if self.owns_pointer_capture.get() {
            self.is_pressed.set(false);
            self.owns_pointer_capture.set(false);
        }
    }

    fn on_lost_focus(&self) {
        self.is_pressed.set(false);
    }

    fn on_key_down(&self, e: &KeyEventArgs) {
        let Some(source_control) = self.enabled_resolved_source() else {
            return;
        };

        match e.key {
            Key::Enter => {
                if source_control.is_focused() && self.execute_click(e.key_modifiers) {
                    self.set_handled(e);
                }
            }
            Key::Space => {
                if source_control.is_focused() {
                    if self.click_mode() == ClickMode::Press {
                        self.execute_click(e.key_modifiers);
                    }

                    self.is_pressed.set(true);
                    self.set_handled(e);
                }
            }
            Key::Escape => {
                if let Some(flyout) = self.try_get_resolved_flyout() {
                    if self.is_flyout_open_for_resolved_source(Some(&flyout)) {
                        flyout.hide();
                    }
                }
            }
            _ => {}
        }
    }

    fn on_key_up(&self, e: &KeyEventArgs) {
        let Some(source_control) = self.enabled_resolved_source() else {
            return;
        };

        if e.key == Key::Space && source_control.is_focused() {
            if self.click_mode() == ClickMode::Release {
                self.execute_click(e.key_modifiers);
            }

            self.is_pressed.set(false);
            self.set_handled(e);
        }
    }

    fn on_root_key_down(&self, e: &KeyEventArgs) {
        let Some(resolved_source_control) = self.resolved_source() else {
            return;
        };
        if (!self.is_default() && !self.is_cancel())
            || !resolved_source_control.is_effectively_visible()
            || !resolved_source_control.is_effectively_enabled()
            || self.is_source_inside_resolved_source(e.source())
        {
            return;
        }

        if self.is_default() && e.key == Key::Enter && self.execute_click(e.key_modifiers) {
            self.set_handled(e);
        } else if self.is_cancel() && e.key == Key::Escape && self.execute_click(e.key_modifiers) {
            self.set_handled(e);
        }
    }

    fn execute_click(&self, current_modifiers: KeyModifiers) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let Some(source_control) = self.enabled_resolved_source() else {
            return false;
        };
        if !self.matches_modifiers(current_modifiers) {
            return false;
        }

        self.toggle_flyout(&source_control);

        let click_args = RoutedEventArgs::with_event_and_source(Button::click_event(), source_control.clone());
        Self::raise_click_event_if_needed(&source_control, &click_args);
        let sender: BoxedValue = Rc::new(source_control.upcast::<FerroObject>());
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(&click_args));
        Interaction::execute_actions(Some(&sender), self.actions().as_ref(), Some(&parameter));
        true
    }

    fn raise_click_event_if_needed(source_control: &Control, click_args: &RoutedEventArgs) {
        // Button-derived controls already raise the click event through native control logic.
        if source_control.to_ref().is::<Button>() {
            return;
        }

        source_control.raise_event(click_args);
    }

    fn toggle_flyout(&self, associated_object: &Control) {
        let Some(flyout) = self.try_get_resolved_flyout() else {
            return;
        };

        if self.is_flyout_open_for_resolved_source(Some(&flyout)) {
            flyout.hide();
        } else {
            flyout.show_at(associated_object);
        }
    }

    fn try_get_resolved_flyout(&self) -> Option<Ref<FlyoutBase>> {
        if let Some(flyout) = self.flyout() {
            return Some(flyout);
        }

        if self.use_attached_flyout() {
            self.resolved_source().and_then(|source_control| FlyoutBase::get_attached_flyout(&source_control))
        } else {
            None
        }
    }

    fn is_flyout_open_for_resolved_source(&self, flyout: Option<&Ref<FlyoutBase>>) -> bool {
        let Some(flyout) = flyout else {
            return false;
        };
        if !flyout.is_open() {
            return false;
        }

        match (flyout.target(), self.resolved_source()) {
            (Some(target), Some(resolved_source_control)) => target.ptr_eq(&resolved_source_control),
            (None, None) => true,
            _ => false,
        }
    }

    fn is_pointer_within_resolved_source(&self, e: &PointerReleasedEventArgs) -> bool {
        let Some(resolved_source_control) = self.resolved_source() else {
            return false;
        };

        let resolved_source_visual: &Visual = &resolved_source_control;
        let position = e.get_position(Some(resolved_source_visual));
        resolved_source_control.get_visuals_at(position).iter().any(|visual| {
            visual.ptr_eq(&resolved_source_control) || resolved_source_control.is_visual_ancestor_of(visual)
        })
    }

    fn matches_modifiers(&self, current_modifiers: KeyModifiers) -> bool {
        self.key_modifiers().is_none_or(|required_modifiers| required_modifiers == current_modifiers)
    }

    fn should_capture_pointer(&self, source_control: &Ref<Control>) -> bool {
        // In non-invasive mode we should not capture the pointer, otherwise controls such as
        // TextBox can lose native drag-selection behavior.
        if !self.handle_event() {
            return false;
        }

        !source_control.is::<TextBox>()
    }

    fn is_source_inside_resolved_source(&self, source: Option<Ref<FerroObject>>) -> bool {
        let Some(resolved_source_control) = self.resolved_source() else {
            return false;
        };
        let Some(source_visual) = source.and_then(|source| source.cast::<Visual>()) else {
            return false;
        };

        source_visual.ptr_eq(&resolved_source_control) || resolved_source_control.is_visual_ancestor_of(&source_visual)
    }

    fn set_handled(&self, e: &dyn IRoutedEventArgs) {
        if self.handle_event() {
            e.set_handled(true);
        }
    }

    fn update_root_subscription(&self, force_reattach: bool) {
        let Some(resolved_source_control) = self.resolved_source() else {
            self.detach_root_subscription();
            return;
        };

        if !self.is_default() && !self.is_cancel() {
            self.detach_root_subscription();
            return;
        }

        let resolved_source_visual: &Visual = &resolved_source_control;
        let root_input_element: Option<Ref<InputElement>> =
            TopLevel::get_top_level(Some(resolved_source_visual)).map(|top_level| top_level.upcast());
        if !force_reattach {
            let current = self.root_input_element.borrow().as_ref().and_then(|(root, _)| root.upgrade());
            let same = match (&current, &root_input_element) {
                (Some(current), Some(root)) => current.ptr_eq(root),
                (None, None) => true,
                _ => false,
            };
            if same {
                return;
            }
        }

        self.detach_root_subscription();

        let Some(root_input_element) = root_input_element else {
            return;
        };

        let trigger: WeakRef<ClickEventTrigger> = self.to_ref().downgrade();
        let token = root_input_element.add_handler_with(
            InputElement::key_down_event(),
            move |_sender: &Interactive, e: &KeyEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_root_key_down(e);
                }
            },
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            self.handled_events_too(),
        );
        *self.root_input_element.borrow_mut() = Some((root_input_element.downgrade(), token));
    }

    fn set_resolved_source(&self, new_source: Option<Ref<Control>>, force_reattach_handlers: bool) {
        let current = self.resolved_source();
        let same = match (&current, &new_source) {
            (Some(current), Some(new_source)) => current.ptr_eq(new_source),
            (None, None) => true,
            _ => false,
        };
        if same {
            if force_reattach_handlers {
                self.is_pressed.set(false);
                self.owns_pointer_capture.set(false);

                if let Some(current) = &current {
                    self.unregister_input_handlers(current);
                    self.register_input_handlers(current);
                }

                self.update_root_subscription(true);
            }

            return;
        }

        self.is_pressed.set(false);
        self.owns_pointer_capture.set(false);

        if let Some(current) = &current {
            self.unregister_input_handlers(current);
        }

        *self.resolved_source_control.borrow_mut() = new_source.as_ref().map(Ref::downgrade);

        if let Some(new_source) = &new_source {
            self.register_input_handlers(new_source);
        }

        self.update_root_subscription(false);
    }

    fn resolve_source_control(&self) -> Option<Ref<Control>> {
        self.source_control().or_else(|| self.associated_object_as::<Control>())
    }

    fn register_input_handlers(&self, source_control: &Control) {
        let input_routing_strategies = self.routing_strategies();
        let handled_events_too = self.handled_events_too();
        let this: WeakRef<ClickEventTrigger> = self.to_ref().downgrade();

        let trigger = this.clone();
        let pointer_pressed = source_control.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_sender: &Interactive, e: &PointerPressedEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_pointer_pressed(e);
                }
            },
            input_routing_strategies,
            handled_events_too,
        );
        let trigger = this.clone();
        let pointer_released = source_control.add_handler_with(
            InputElement::pointer_released_event(),
            move |_sender: &Interactive, e: &PointerReleasedEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_pointer_released(e);
                }
            },
            input_routing_strategies,
            handled_events_too,
        );
        let trigger = this.clone();
        let pointer_capture_lost = source_control.add_handler_with(
            InputElement::pointer_capture_lost_event(),
            move |_sender: &Interactive, e: &PointerCaptureLostEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_pointer_capture_lost(e);
                }
            },
            RoutingStrategies::DIRECT,
            handled_events_too,
        );
        let trigger = this.clone();
        let key_down = source_control.add_handler_with(
            InputElement::key_down_event(),
            move |_sender: &Interactive, e: &KeyEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_key_down(e);
                }
            },
            input_routing_strategies,
            handled_events_too,
        );
        let trigger = this.clone();
        let key_up = source_control.add_handler_with(
            InputElement::key_up_event(),
            move |_sender: &Interactive, e: &KeyEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_key_up(e);
                }
            },
            input_routing_strategies,
            handled_events_too,
        );
        let trigger = this;
        let lost_focus = source_control.add_handler_with(
            InputElement::lost_focus_event(),
            move |_sender: &Interactive, _e: &FocusChangedEventArgs| {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_lost_focus();
                }
            },
            RoutingStrategies::BUBBLE,
            handled_events_too,
        );

        self.input_handlers.set(Some(InputHandlers {
            pointer_pressed,
            pointer_released,
            pointer_capture_lost,
            key_down,
            key_up,
            lost_focus,
        }));
    }

    fn unregister_input_handlers(&self, source_control: &Control) {
        let Some(handlers) = self.input_handlers.take() else {
            return;
        };

        source_control.remove_handler(InputElement::pointer_pressed_event(), handlers.pointer_pressed);
        source_control.remove_handler(InputElement::pointer_released_event(), handlers.pointer_released);
        source_control.remove_handler(InputElement::pointer_capture_lost_event(), handlers.pointer_capture_lost);
        source_control.remove_handler(InputElement::key_down_event(), handlers.key_down);
        source_control.remove_handler(InputElement::key_up_event(), handlers.key_up);
        source_control.remove_handler(InputElement::lost_focus_event(), handlers.lost_focus);
    }

    fn detach_root_subscription(&self) {
        let root_input_element = self.root_input_element.borrow_mut().take();
        let Some((root_input_element, token)) = root_input_element else {
            return;
        };

        if let Some(root_input_element) = root_input_element.upgrade() {
            root_input_element.remove_handler(InputElement::key_down_event(), token);
        }
    }
}
