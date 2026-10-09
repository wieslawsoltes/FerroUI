use super::gesture_recognizers::GestureRecognizerCollection;
use super::text_input::TextInputMethodClientRequestedEventArgs;
use super::{
    AccessKeyHandler, KeyBinding,
    ContextRequestedEventArgs, Cursor, ICustomKeyboardNavigation, IScrollable, FocusChangedEventArgs, FocusChangingEventArgs, FocusManager, Gestures,
    HoldingRoutedEventArgs, IInputElement, KeyEventArgs, KeyModifiers, KeyboardNavigation, NavigationMethod,
    PointerCaptureChangingEventArgs, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerWheelEventArgs, TappedEventArgs, TextInputEventArgs,
};
use crate::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use crate::layout::LayoutableImpl;
use crate::visual_tree::IHostedVisualTreeRoot;
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use crate::collections::FerroList;
use std::cell::{Cell, OnceCell};
use std::rc::Rc;

/// Implements input-related functionality for a control.
///
/// The pseudoclasses `:disabled`, `:focus`, `:focus-visible`,
/// `:focus-within` and `:pointerover` reflect the input state of the
/// element.
#[repr(C)]
pub struct InputElement {
    base: Interactive,
    is_effectively_enabled: Cell<bool>,
    is_focused: Cell<bool>,
    is_keyboard_focus_within: Cell<bool>,
    is_focus_visible: Cell<bool>,
    is_pointer_over: Cell<bool>,
    pub(super) is_context_menu_on_holding: Cell<bool>,
    key_bindings: OnceCell<FerroList<Ref<KeyBinding>>>,
    gesture_recognizers: OnceCell<GestureRecognizerCollection>,
}

ferro_class! {
    InputElement: Interactive, virtuals InputElementImpl: InteractiveImpl {
        /// Whether the control is enabled, as far as the control itself is
        /// concerned. The default is the value of `is_enabled`; controls
        /// with additional conditions (e.g. a command that cannot execute)
        /// override this.
        fn is_enabled_core(this) -> bool;
        /// Whether the element is a focus scope: an element that remembers
        /// which of its descendants was focused last.
        fn is_focus_scope(this) -> bool;
        /// Called when the control or one of its child elements is about to
        /// get focus.
        fn on_getting_focus(this, e: &FocusChangingEventArgs);
        /// Called when the control or one of its child elements is about to
        /// lose focus.
        fn on_losing_focus(this, e: &FocusChangingEventArgs);
        /// Called when the control or one of its child elements gets focus.
        fn on_got_focus(this, e: &FocusChangedEventArgs);
        /// Called when the control or one of its child elements loses
        /// focus.
        fn on_lost_focus(this, e: &FocusChangedEventArgs);
        /// Called before the key down event occurs.
        fn on_key_down(this, e: &KeyEventArgs);
        /// Called before the key up event occurs.
        fn on_key_up(this, e: &KeyEventArgs);
        /// Called before the text input event occurs.
        fn on_text_input(this, e: &TextInputEventArgs);
        /// Called before the pointer entered event occurs.
        fn on_pointer_entered(this, e: &PointerEventArgs);
        /// Called before the pointer exited event occurs.
        fn on_pointer_exited(this, e: &PointerEventArgs);
        /// Called before the pointer moved event occurs.
        fn on_pointer_moved(this, e: &PointerEventArgs);
        /// Called before the pointer pressed event occurs.
        fn on_pointer_pressed(this, e: &PointerPressedEventArgs);
        /// Called before the pointer released event occurs.
        fn on_pointer_released(this, e: &PointerReleasedEventArgs);
        /// Called when the capture of a pointer is about to change on the
        /// control or one of its descendants.
        fn on_pointer_capture_changing(this, e: &PointerCaptureChangingEventArgs);
        /// Called before the pointer capture lost event occurs.
        fn on_pointer_capture_lost(this, e: &PointerCaptureLostEventArgs);
        /// Called before the pointer wheel changed event occurs.
        fn on_pointer_wheel_changed(this, e: &PointerWheelEventArgs);
        /// Called when a tap gesture occurs on the control.
        fn on_tapped(this, e: &TappedEventArgs);
        /// Called when a right tap gesture occurs on the control.
        fn on_right_tapped(this, e: &TappedEventArgs);
        /// Called when a double-tap gesture occurs on the control.
        fn on_double_tapped(this, e: &TappedEventArgs);
        /// Called when a hold gesture occurs on the control.
        fn on_holding(this, e: &HoldingRoutedEventArgs);
        /// Called when the access key of the control is pressed. The
        /// default focuses the control.
        fn on_access_key(this, e: &dyn IRoutedEventArgs);
        /// Called when focus is moving forward out of this element: lets it
        /// name the element that should receive focus next.
        fn get_next_tab_stop_override(this) -> Option<Ref<InputElement>>;
        /// Called when focus is moving backward out of this element: lets
        /// it name the element that should receive focus next.
        fn get_previous_tab_stop_override(this) -> Option<Ref<InputElement>>;
        /// Lets the element name the first focusable element within it.
        fn get_first_focusable_element_override(this) -> Option<Ref<InputElement>>;
        /// Lets the element name the last focusable element within it.
        fn get_last_focusable_element_override(this) -> Option<Ref<InputElement>>;
        /// Lets the element (an ancestor of the focused element) override
        /// the tab stop chosen by tab navigation. Returns true when the tab
        /// stop is overridden; the override is written to `new_tab_stop`.
        fn process_tab_stop_override(
            this,
            focused_element: Option<&Ref<InputElement>>,
            candidate_tab_stop_element: Option<&Ref<InputElement>>,
            is_reverse: bool,
            did_cycle_focus_at_root_visual: bool,
            new_tab_stop: &mut Option<Ref<InputElement>>
        ) -> bool;
        /// Lets the element (an ancestor of the candidate tab stop)
        /// override the tab stop chosen by tab navigation. Returns true
        /// when the tab stop is overridden; the override is written to
        /// `new_tab_stop`.
        fn process_candidate_tab_stop_override(
            this,
            focused_element: Option<&Ref<InputElement>>,
            candidate_tab_stop_element: Option<&Ref<InputElement>>,
            overriden_candidate_tab_stop_element: Option<&Ref<InputElement>>,
            is_reverse: bool,
            new_tab_stop: &mut Option<Ref<InputElement>>
        ) -> bool;
        /// The element as a custom keyboard navigation handler, if it is
        /// one.
        fn as_custom_keyboard_navigation(this) -> Option<&dyn ICustomKeyboardNavigation>;
        /// The element as a scrollable, if it is one.
        fn as_scrollable(this) -> Option<&dyn IScrollable>;
        /// The element as the root of a hosted visual tree (such as a
        /// popup root), if it is one.
        fn as_hosted_visual_tree_root(this) -> Option<&dyn IHostedVisualTreeRoot>;
    }
}
crate::ferro_class_info!(InputElement { new: InputElement::new });

ferro_impl_classes!(InputElement: StyledElementImpl, LayoutableImpl, InteractiveImpl);

crate::ferro_overrides! { impl FerroObjectImpl for InputElement {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(Some(this.is_focused()), Some(this.is_pointer_over()));
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::is_focused_property().as_property() {
            this.update_pseudo_classes(Some(change.get_new_value::<bool>()), None);
        } else if property == Self::is_pointer_over_property().as_property() {
            this.update_pseudo_classes(None, Some(change.get_new_value::<bool>()));
        } else if property == Self::is_keyboard_focus_within_property().as_property() {
            this.pseudo_classes().set(":focus-within", change.get_new_value::<bool>());
        } else if property == Visual::is_visible_property().as_property()
            && !change.get_new_value::<bool>()
            && this.is_keyboard_focus_within()
        {
            if let Some(focus_manager) = FocusManager::get_focus_manager(this) {
                match (focus_manager.get_focused_element(), this.visual_parent()) {
                    (Some(focused_element), Some(visual_parent)) => {
                        focus_manager.clear_focus_on_element_removed(&focused_element, &visual_parent);
                    }
                    _ => {
                        focus_manager.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
                    }
                }
            }
        }
    }
} }

impl VisualImpl for InputElement {
    fn on_detached_from_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree_core(this, e);

        if this.is_focused() {
            let root = e.attachment_point().unwrap_or(e.root_visual());
            let focus_manager = e
                .presentation_source()
                .input_root()
                .focus_manager()
                .or_else(|| FocusManager::get_focus_manager(this));

            if let Some(focus_manager) = focus_manager {
                focus_manager.clear_focus_on_element_removed(this, root);
            }
        }

        this.set_is_keyboard_focus_within(false);
    }

    fn on_attached_to_visual_tree_core(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree_core(this, e);
        this.update_is_effectively_enabled();
    }
}

impl InputElementImpl for InputElement {
    fn is_enabled_core(this: &Self) -> bool {
        this.is_enabled()
    }

    fn is_focus_scope(_this: &Self) -> bool {
        false
    }

    fn on_getting_focus(_this: &Self, _e: &FocusChangingEventArgs) {}

    fn on_losing_focus(_this: &Self, _e: &FocusChangingEventArgs) {}

    fn on_got_focus(_this: &Self, _e: &FocusChangedEventArgs) {}

    fn on_lost_focus(_this: &Self, _e: &FocusChangedEventArgs) {}

    fn on_key_down(_this: &Self, _e: &KeyEventArgs) {}

    fn on_key_up(_this: &Self, _e: &KeyEventArgs) {}

    fn on_text_input(_this: &Self, _e: &TextInputEventArgs) {}

    fn on_pointer_entered(_this: &Self, _e: &PointerEventArgs) {}

    fn on_pointer_exited(_this: &Self, _e: &PointerEventArgs) {}

    fn on_pointer_moved(_this: &Self, _e: &PointerEventArgs) {}

    fn on_pointer_pressed(_this: &Self, _e: &PointerPressedEventArgs) {}

    fn on_pointer_released(_this: &Self, _e: &PointerReleasedEventArgs) {}

    fn on_pointer_capture_changing(_this: &Self, _e: &PointerCaptureChangingEventArgs) {}

    fn on_pointer_capture_lost(_this: &Self, _e: &PointerCaptureLostEventArgs) {}

    fn on_pointer_wheel_changed(_this: &Self, _e: &PointerWheelEventArgs) {}

    fn on_tapped(_this: &Self, _e: &TappedEventArgs) {}

    fn on_right_tapped(_this: &Self, _e: &TappedEventArgs) {}

    fn on_double_tapped(_this: &Self, _e: &TappedEventArgs) {}

    fn on_holding(_this: &Self, _e: &HoldingRoutedEventArgs) {}

    fn on_access_key(this: &Self, _e: &dyn IRoutedEventArgs) {
        this.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
    }

    fn get_next_tab_stop_override(_this: &Self) -> Option<Ref<InputElement>> {
        None
    }

    fn get_previous_tab_stop_override(_this: &Self) -> Option<Ref<InputElement>> {
        None
    }

    fn get_first_focusable_element_override(_this: &Self) -> Option<Ref<InputElement>> {
        None
    }

    fn get_last_focusable_element_override(_this: &Self) -> Option<Ref<InputElement>> {
        None
    }

    fn process_tab_stop_override(
        _this: &Self,
        _focused_element: Option<&Ref<InputElement>>,
        _candidate_tab_stop_element: Option<&Ref<InputElement>>,
        _is_reverse: bool,
        _did_cycle_focus_at_root_visual: bool,
        _new_tab_stop: &mut Option<Ref<InputElement>>,
    ) -> bool {
        false
    }

    fn process_candidate_tab_stop_override(
        _this: &Self,
        _focused_element: Option<&Ref<InputElement>>,
        _candidate_tab_stop_element: Option<&Ref<InputElement>>,
        _overriden_candidate_tab_stop_element: Option<&Ref<InputElement>>,
        _is_reverse: bool,
        _new_tab_stop: &mut Option<Ref<InputElement>>,
    ) -> bool {
        false
    }

    fn as_custom_keyboard_navigation(_this: &Self) -> Option<&dyn ICustomKeyboardNavigation> {
        None
    }

    fn as_scrollable(_this: &Self) -> Option<&dyn IScrollable> {
        None
    }

    fn as_hosted_visual_tree_root(_this: &Self) -> Option<&dyn IHostedVisualTreeRoot> {
        None
    }
}

/// Declares the `fn name(&self, handler) -> token` method that adds a
/// handler for a routed event, like a C# event accessor.
macro_rules! routed_event_accessor {
    ($(#[$meta:meta])* $name:ident, $event:ident, $args:ty) => {
        $(#[$meta])*
        pub fn $name(&self, handler: impl Fn(&Interactive, &$args) + 'static) -> RoutedEventHandlerToken {
            self.add_handler(Self::$event(), handler)
        }
    };
}

pub(super) use routed_event_accessor;

crate::ferro_properties! { impl InputElement, also [InputElement::register_gesture_properties] {
    // --- properties ---------------------------------------------------------

    ferro_property!(
        /// Defines the `Focusable` property.
        pub fn focusable_property() -> StyledProperty<bool> {
            FerroProperty::register::<InputElement, _>("Focusable", false)
        }
    );

    ferro_property!(
        /// Defines the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<InputElement, _>("IsEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsEffectivelyEnabled` property.
        pub fn is_effectively_enabled_property() -> DirectProperty<InputElement, bool> {
            FerroProperty::register_direct::<InputElement, _>(
                "IsEffectivelyEnabled",
                |o| o.is_effectively_enabled(),
                None,
                false,
            )
        }
    );

    ferro_property!(
        /// Gets or sets associated mouse cursor.
        pub fn cursor_property() -> StyledProperty<Option<Rc<Cursor>>> {
            FerroProperty::register_with::<InputElement, _>("Cursor", StyledPropertyOptions::new(None).inherits(true))
        }
    );

    ferro_property!(
        /// Defines the `IsKeyboardFocusWithin` property.
        pub fn is_keyboard_focus_within_property() -> DirectProperty<InputElement, bool> {
            FerroProperty::register_direct::<InputElement, _>(
                "IsKeyboardFocusWithin",
                |o| o.is_keyboard_focus_within(),
                None,
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `IsFocused` property.
        pub fn is_focused_property() -> DirectProperty<InputElement, bool> {
            FerroProperty::register_direct::<InputElement, _>("IsFocused", |o| o.is_focused(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `IsHitTestVisible` property.
        pub fn is_hit_test_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<InputElement, _>("IsHitTestVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsPointerOver` property.
        pub fn is_pointer_over_property() -> DirectProperty<InputElement, bool> {
            FerroProperty::register_direct::<InputElement, _>("IsPointerOver", |o| o.is_pointer_over(), None, false)
        }
    );

    ferro_property!(
        /// Defines the `IsTabStop` property.
        pub fn is_tab_stop_property() -> StyledProperty<bool> {
            KeyboardNavigation::is_tab_stop_property().add_owner::<InputElement>()
        }
    );

    ferro_property!(
        /// Defines the `TabIndex` property.
        pub fn tab_index_property() -> StyledProperty<i32> {
            KeyboardNavigation::tab_index_property().add_owner::<InputElement>()
        }
    );
} }

impl InputElement {
    // --- routed events ------------------------------------------------------

    ferro_routed_event!(
        /// Defines the `GotFocus` event.
        pub fn got_focus_event() -> RoutedEvent<FocusChangedEventArgs> {
            RoutedEvent::register::<InputElement, _>("GotFocus", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `GettingFocus` event.
        pub fn getting_focus_event() -> RoutedEvent<FocusChangingEventArgs> {
            RoutedEvent::register::<InputElement, _>("GettingFocus", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `LostFocus` event.
        pub fn lost_focus_event() -> RoutedEvent<FocusChangedEventArgs> {
            RoutedEvent::register::<InputElement, _>("LostFocus", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `LosingFocus` event.
        pub fn losing_focus_event() -> RoutedEvent<FocusChangingEventArgs> {
            RoutedEvent::register::<InputElement, _>("LosingFocus", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `KeyDown` event.
        pub fn key_down_event() -> RoutedEvent<KeyEventArgs> {
            RoutedEvent::register::<InputElement, _>("KeyDown", RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `KeyUp` event.
        pub fn key_up_event() -> RoutedEvent<KeyEventArgs> {
            RoutedEvent::register::<InputElement, _>("KeyUp", RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `TextInput` event.
        pub fn text_input_event() -> RoutedEvent<TextInputEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "TextInput",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Defines the `TextInputMethodClientRequested` event.
        pub fn text_input_method_client_requested_event() -> RoutedEvent<TextInputMethodClientRequestedEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "TextInputMethodClientRequested",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerEntered` event.
        pub fn pointer_entered_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerEntered", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerExited` event.
        pub fn pointer_exited_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerExited", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerMoved` event.
        pub fn pointer_moved_event() -> RoutedEvent<PointerEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "PointerMoved",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerPressed` event.
        pub fn pointer_pressed_event() -> RoutedEvent<PointerPressedEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "PointerPressed",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerReleased` event.
        pub fn pointer_released_event() -> RoutedEvent<PointerReleasedEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "PointerReleased",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerCaptureChanging` routed event.
        pub fn pointer_capture_changing_event() -> RoutedEvent<PointerCaptureChangingEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerCaptureChanging", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerCaptureLost` routed event.
        pub fn pointer_capture_lost_event() -> RoutedEvent<PointerCaptureLostEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerCaptureLost", RoutingStrategies::DIRECT)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerWheelChanged` event.
        pub fn pointer_wheel_changed_event() -> RoutedEvent<PointerWheelEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "PointerWheelChanged",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Provides event data for the `ContextRequested` event.
        pub fn context_requested_event() -> RoutedEvent<ContextRequestedEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "ContextRequested",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    ferro_routed_event!(
        /// Provides event data for the `ContextCanceled` event.
        pub fn context_canceled_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "ContextCanceled",
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
            )
        }
    );

    /// The static initialisation of the class: registers the class handlers
    /// that route the input events to the `on_*` virtual members and the
    /// gesture synthesis.
    fn static_constructor() {
        // Register the routed events that no class handler below refers
        // to, so that the registry lists every event of the class once the
        // class is initialised.
        Self::context_requested_event();
        Self::context_canceled_event();
        Self::register_gesture_events();

        Self::is_enabled_property().changed().subscribe(|e| {
            if let Some(sender) = e.sender().downcast_ref::<InputElement>() {
                sender.update_is_effectively_enabled();
            }
        });

        Self::got_focus_event().add_class_handler::<InputElement>(|x, e| x.on_got_focus_core(e));
        Self::lost_focus_event().add_class_handler::<InputElement>(|x, e| x.on_lost_focus_core(e));
        Self::getting_focus_event().add_class_handler::<InputElement>(|x, e| x.on_getting_focus(e));
        Self::losing_focus_event().add_class_handler::<InputElement>(|x, e| x.on_losing_focus(e));
        Self::key_down_event().add_class_handler::<InputElement>(|x, e| x.on_key_down(e));
        Self::key_up_event().add_class_handler::<InputElement>(|x, e| x.on_key_up(e));
        Self::text_input_event().add_class_handler::<InputElement>(|x, e| x.on_text_input(e));
        Self::pointer_entered_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_entered_core(e));
        Self::pointer_exited_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_exited_core(e));
        Self::pointer_moved_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_moved(e));
        Self::pointer_pressed_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_pressed(e));
        Self::pointer_released_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_released(e));
        Self::pointer_capture_changing_event()
            .add_class_handler::<InputElement>(|x, e| x.on_pointer_capture_changing(e));
        Self::pointer_capture_lost_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_capture_lost(e));
        Self::pointer_wheel_changed_event().add_class_handler::<InputElement>(|x, e| x.on_pointer_wheel_changed(e));
        Self::tapped_event().add_class_handler::<InputElement>(|x, e| x.on_tapped(e));
        Self::right_tapped_event().add_class_handler::<InputElement>(|x, e| x.on_right_tapped(e));
        Self::double_tapped_event().add_class_handler::<InputElement>(|x, e| x.on_double_tapped(e));
        Self::holding_event().add_class_handler::<InputElement>(|x, e| x.on_holding(e));

        Gestures::TYPE.ensure_class_init();
        Gestures::add_tapped(|s, e| {
            if s.is::<InputElement>() {
                s.raise_event(e);
            }
        });
        Gestures::add_right_tapped(|s, e| {
            if s.is::<InputElement>() {
                s.raise_event(e);
            }
        });
        Gestures::add_double_tapped(|s, e| {
            if s.is::<InputElement>() {
                s.raise_event(e);
            }
        });
        Gestures::add_holding(Self::on_preview_holding);

        // Access Key Handling
        AccessKeyHandler::access_key_event().add_class_handler::<InputElement>(|x, e| x.on_access_key(e));

        // Gesture only handlers
        Self::pointer_moved_event().add_class_handler_with::<InputElement>(
            |x, e| x.on_gesture_pointer_moved(e),
            Interactive::DEFAULT_ROUTES,
            true,
        );
        Self::pointer_pressed_event().add_class_handler_with::<InputElement>(
            |x, e| x.on_gesture_pointer_pressed(e),
            Interactive::DEFAULT_ROUTES,
            true,
        );
        Self::pointer_released_event().add_class_handler_with::<InputElement>(
            |x, e| x.on_gesture_pointer_released(e),
            Interactive::DEFAULT_ROUTES,
            true,
        );
        Self::pointer_capture_lost_event().add_class_handler_with::<InputElement>(
            |x, e| x.on_gesture_pointer_capture_lost(e),
            Interactive::DEFAULT_ROUTES,
            true,
        );
    }

    /// Creates the class data; see [`crate::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Interactive::construct(),
            is_effectively_enabled: Cell::new(true),
            is_focused: Cell::new(false),
            is_keyboard_focus_within: Cell::new(false),
            is_focus_visible: Cell::new(false),
            is_pointer_over: Cell::new(false),
            is_context_menu_on_holding: Cell::new(false),
            key_bindings: OnceCell::new(),
            gesture_recognizers: OnceCell::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // --- events -------------------------------------------------------------

    routed_event_accessor!(
        /// Occurs when the control receives focus.
        got_focus, got_focus_event, FocusChangedEventArgs
    );

    routed_event_accessor!(
        /// Occurs before the control receives focus.
        getting_focus, getting_focus_event, FocusChangingEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the control loses focus.
        lost_focus, lost_focus_event, FocusChangedEventArgs
    );

    routed_event_accessor!(
        /// Occurs before the control loses focus.
        losing_focus, losing_focus_event, FocusChangingEventArgs
    );

    routed_event_accessor!(
        /// Occurs when a key is pressed while the control has focus.
        key_down, key_down_event, KeyEventArgs
    );

    routed_event_accessor!(
        /// Occurs when a key is released while the control has focus.
        key_up, key_up_event, KeyEventArgs
    );

    routed_event_accessor!(
        /// Occurs when a user typed some text while the control has focus.
        text_input, text_input_event, TextInputEventArgs
    );

    routed_event_accessor!(
        /// Occurs when an input element gains input focus and the input
        /// method is looking for the corresponding client.
        text_input_method_client_requested,
        text_input_method_client_requested_event,
        TextInputMethodClientRequestedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the pointer enters the control.
        pointer_entered, pointer_entered_event, PointerEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the pointer leaves the control.
        pointer_exited, pointer_exited_event, PointerEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the pointer moves over the control.
        pointer_moved, pointer_moved_event, PointerEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the pointer is pressed over the control.
        pointer_pressed, pointer_pressed_event, PointerPressedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the pointer is released over the control.
        pointer_released, pointer_released_event, PointerReleasedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the capture of a pointer is about to change on the
        /// control.
        pointer_capture_changing, pointer_capture_changing_event, PointerCaptureChangingEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the control or its child control loses the pointer
        /// capture for any reason; the event will not be triggered for a
        /// parent control if capture was transferred to another child of
        /// that parent control.
        pointer_capture_lost, pointer_capture_lost_event, PointerCaptureLostEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the mouse is scrolled over the control.
        pointer_wheel_changed, pointer_wheel_changed_event, PointerWheelEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user has completed a context input gesture, such
        /// as a right-click.
        context_requested, context_requested_event, ContextRequestedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the context input gesture continues into another
        /// gesture, to notify the element that the context flyout should
        /// not be opened.
        context_canceled, context_canceled_event, RoutedEventArgs
    );

    // --- property accessors -------------------------------------------------

    /// Whether the control can receive focus.
    pub fn focusable(&self) -> bool {
        self.get_value(Self::focusable_property())
    }

    pub fn set_focusable(&self, value: bool) {
        self.set_value(Self::focusable_property(), value)
    }

    /// Whether the control is enabled for user interaction.
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value)
    }

    /// The associated mouse cursor.
    pub fn cursor(&self) -> Option<Rc<Cursor>> {
        self.get_value(Self::cursor_property())
    }

    pub fn set_cursor(&self, value: Option<Rc<Cursor>>) {
        self.set_value(Self::cursor_property(), value)
    }

    /// Whether keyboard focus is anywhere within the element or its visual
    /// tree child elements.
    #[inline]
    pub fn is_keyboard_focus_within(&self) -> bool {
        self.is_keyboard_focus_within.get()
    }

    pub(crate) fn set_is_keyboard_focus_within(&self, value: bool) {
        self.set_and_raise_cell(Self::is_keyboard_focus_within_property(), &self.is_keyboard_focus_within, value);
    }

    /// Whether the control is focused.
    #[inline]
    pub fn is_focused(&self) -> bool {
        self.is_focused.get()
    }

    fn set_is_focused(&self, value: bool) {
        self.set_and_raise_cell(Self::is_focused_property(), &self.is_focused, value);
    }

    /// Whether the control is considered for hit testing.
    pub fn is_hit_test_visible(&self) -> bool {
        self.get_value(Self::is_hit_test_visible_property())
    }

    pub fn set_is_hit_test_visible(&self, value: bool) {
        self.set_value(Self::is_hit_test_visible_property(), value)
    }

    /// Whether the pointer is currently over the control.
    #[inline]
    pub fn is_pointer_over(&self) -> bool {
        self.is_pointer_over.get()
    }

    pub(crate) fn set_is_pointer_over(&self, value: bool) {
        self.set_and_raise_cell(Self::is_pointer_over_property(), &self.is_pointer_over, value);
    }

    /// Whether the control is included in tab navigation.
    pub fn is_tab_stop(&self) -> bool {
        self.get_value(Self::is_tab_stop_property())
    }

    pub fn set_is_tab_stop(&self, value: bool) {
        self.set_value(Self::is_tab_stop_property(), value)
    }

    /// Whether this control and all its parents are enabled.
    #[inline]
    pub fn is_effectively_enabled(&self) -> bool {
        self.is_effectively_enabled.get()
    }

    fn set_is_effectively_enabled(&self, value: bool) {
        self.set_and_raise_cell(Self::is_effectively_enabled_property(), &self.is_effectively_enabled, value);
        self.pseudo_classes().set(":disabled", !value);

        if !self.is_effectively_enabled() {
            if let Some(focus_manager) = FocusManager::get_focus_manager(self) {
                let is_focused_element = focus_manager
                    .get_focused_element()
                    .is_some_and(|focused| std::ptr::eq::<InputElement>(&*focused, self));

                if is_focused_element {
                    focus_manager.focus(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
                }
            }
        }
    }

    /// A value that determines the order in which elements receive focus
    /// when the user navigates through controls by pressing the Tab key.
    pub fn tab_index(&self) -> i32 {
        self.get_value(Self::tab_index_property())
    }

    pub fn set_tab_index(&self, value: i32) {
        self.set_value(Self::tab_index_property(), value)
    }

    /// The key bindings for this input element.
    ///
    /// The list is returned by handle: clones refer to the same list.
    pub fn key_bindings(&self) -> FerroList<Ref<KeyBinding>> {
        self.key_bindings.get_or_init(FerroList::new).clone()
    }

    /// The gesture recognizers attached to this input element.
    pub fn gesture_recognizers(&self) -> GestureRecognizerCollection {
        self.gesture_recognizers.get_or_init(|| GestureRecognizerCollection::new(&self.to_ref())).clone()
    }

    fn on_gesture_pointer_released(&self, e: &PointerReleasedEventArgs) {
        if !e.is_gesture_recognition_skipped() {
            if let Some(recognizers) = self.gesture_recognizers.get() {
                if recognizers.handle_pointer_released(e) {
                    e.set_handled(true);
                }
            }
        }
    }

    fn on_gesture_pointer_capture_lost(&self, e: &PointerCaptureLostEventArgs) {
        if let Some(recognizers) = self.gesture_recognizers.get() {
            recognizers.handle_capture_lost(e.pointer());
        }
    }

    fn on_gesture_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        if !e.is_gesture_recognition_skipped() {
            if let Some(recognizers) = self.gesture_recognizers.get() {
                if recognizers.handle_pointer_pressed(e) {
                    e.set_handled(true);
                }
            }
        }
    }

    fn on_gesture_pointer_moved(&self, e: &PointerEventArgs) {
        if !e.is_gesture_recognition_skipped() {
            if let Some(recognizers) = self.gesture_recognizers.get() {
                if recognizers.handle_pointer_moved(e) {
                    e.set_handled(true);
                }
            }
        }
    }

    // --- methods ------------------------------------------------------------

    /// Focuses the control, with an unspecified navigation method and no
    /// key modifiers. Returns whether the control was focused.
    pub fn focus(&self) -> bool {
        self.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE)
    }

    /// Focuses the control.
    ///
    /// `method` is the method by which focus was changed and
    /// `key_modifiers` any key modifiers active at the time of focus.
    /// Returns whether the control was focused.
    pub fn focus_with(&self, method: NavigationMethod, key_modifiers: KeyModifiers) -> bool {
        match FocusManager::get_focus_manager(self) {
            Some(focus_manager) => focus_manager.focus(Some(&self.to_ref()), method, key_modifiers),
            None => false,
        }
    }

    /// Runs the tab stop overrides of the focused element's ancestors and of
    /// the candidate's ancestors. Returns whether the tab stop was
    /// overridden and the overriding tab stop.
    pub(crate) fn process_tab_stop(
        _content_root: Option<&Ref<InputElement>>,
        focused_element: Option<&Ref<InputElement>>,
        candidate_tab_stop_element: Option<&Ref<InputElement>>,
        is_reverse: bool,
        did_cycle_focus_at_root_visual: bool,
    ) -> (bool, Option<Ref<InputElement>>) {
        // The override results are kept in locals and only published under
        // the conditions below, as upstream does.
        let new_tab_stop: Option<Ref<InputElement>> = None;
        let mut is_tab_stop_overridden = false;
        let mut sp_new_tab_stop: Option<Ref<InputElement>> = None;

        if let Some(focused) = focused_element {
            let (overridden, tab_stop) = focused.process_tab_stop_internal(
                candidate_tab_stop_element,
                is_reverse,
                did_cycle_focus_at_root_visual,
            );
            is_tab_stop_overridden = overridden;
            sp_new_tab_stop = tab_stop;
        }

        if !is_tab_stop_overridden {
            if let Some(candidate) = candidate_tab_stop_element {
                let (overridden, tab_stop) =
                    candidate.process_candidate_tab_stop_internal(focused_element, None, is_reverse);
                is_tab_stop_overridden = overridden;
                sp_new_tab_stop = tab_stop;
            }
        }

        // The remaining upstream branches only act on a tab stop that was
        // already published, which never is at this point: an override is
        // reported without a replacement element.
        let _ = sp_new_tab_stop;

        (is_tab_stop_overridden, new_tab_stop)
    }

    fn process_tab_stop_internal(
        &self,
        candidate_tab_stop_element: Option<&Ref<InputElement>>,
        is_reverse: bool,
        did_cycle_focus_at_root_visual: bool,
    ) -> (bool, Option<Ref<InputElement>>) {
        let this = self.to_ref();
        let mut current = Some(this.clone());
        let mut new_tab_stop = None;
        let mut candidate_tab_stop_overridden = false;

        while let Some(c) = current {
            if candidate_tab_stop_overridden {
                break;
            }

            candidate_tab_stop_overridden = c.process_tab_stop_override(
                Some(&this),
                candidate_tab_stop_element,
                is_reverse,
                did_cycle_focus_at_root_visual,
                &mut new_tab_stop,
            );
            current = c.parent().and_then(|parent| parent.downcast::<InputElement>().ok());
        }

        (candidate_tab_stop_overridden, new_tab_stop)
    }

    fn process_candidate_tab_stop_internal(
        &self,
        current_tab_stop: Option<&Ref<InputElement>>,
        overriden_candidate_tab_stop_element: Option<&Ref<InputElement>>,
        is_reverse: bool,
    ) -> (bool, Option<Ref<InputElement>>) {
        let this = self.to_ref();
        let mut current = Some(this.clone());
        let mut new_tab_stop = None;
        let mut candidate_tab_stop_overridden = false;

        while let Some(c) = current {
            if candidate_tab_stop_overridden {
                break;
            }

            candidate_tab_stop_overridden = c.process_candidate_tab_stop_override(
                current_tab_stop,
                Some(&this),
                overriden_candidate_tab_stop_element,
                is_reverse,
                &mut new_tab_stop,
            );
            current = c.parent().and_then(|parent| parent.downcast::<InputElement>().ok());
        }

        (candidate_tab_stop_overridden, new_tab_stop)
    }

    fn on_got_focus_core(&self, e: &FocusChangedEventArgs) {
        let is_focused = e.is_source(self);
        self.is_focus_visible.set(
            is_focused
                && (e.navigation_method == NavigationMethod::Directional
                    || e.navigation_method == NavigationMethod::Tab),
        );
        self.set_is_focused(is_focused);
        self.on_got_focus(e);
    }

    fn on_lost_focus_core(&self, e: &FocusChangedEventArgs) {
        self.is_focus_visible.set(false);
        self.set_is_focused(false);
        self.on_lost_focus(e);
    }

    /// Updates the `is_effectively_enabled` property based on the parent's
    /// `is_effectively_enabled`.
    pub fn update_is_effectively_enabled(&self) {
        let parent = self.get_visual_parent_of_type::<InputElement>();
        self.update_is_effectively_enabled_from(parent.as_deref());
    }

    fn on_pointer_entered_core(&self, e: &PointerEventArgs) {
        self.set_is_pointer_over(true);
        self.on_pointer_entered(e);
    }

    fn on_pointer_exited_core(&self, e: &PointerEventArgs) {
        self.set_is_pointer_over(false);
        self.on_pointer_exited(e);
    }

    /// Updates the `is_effectively_enabled` property based on the parent's
    /// `is_effectively_enabled`.
    fn update_is_effectively_enabled_from(&self, parent: Option<&InputElement>) {
        self.set_is_effectively_enabled(
            self.is_enabled_core() && parent.is_none_or(InputElement::is_effectively_enabled),
        );

        // PERF-SENSITIVE: this is called on the entire hierarchy.
        if let Some(children) = self.visual_children_snapshot() {
            for child in children.iter() {
                if let Some(child) = child.downcast_ref::<InputElement>() {
                    child.update_is_effectively_enabled_from(Some(self));
                }
            }
        }
    }

    fn update_pseudo_classes(&self, is_focused: Option<bool>, is_pointer_over: Option<bool>) {
        if let Some(is_focused) = is_focused {
            self.pseudo_classes().set(":focus", is_focused);
            self.pseudo_classes().set(":focus-visible", self.is_focus_visible.get());
        }

        if let Some(is_pointer_over) = is_pointer_over {
            self.pseudo_classes().set(":pointerover", is_pointer_over);
        }
    }
}

impl IInputElement for InputElement {
    fn focusable(&self) -> bool {
        InputElement::focusable(self)
    }

    fn is_enabled(&self) -> bool {
        InputElement::is_enabled(self)
    }

    fn cursor(&self) -> Option<Rc<Cursor>> {
        InputElement::cursor(self)
    }

    fn is_effectively_enabled(&self) -> bool {
        InputElement::is_effectively_enabled(self)
    }

    fn is_effectively_visible(&self) -> bool {
        Visual::is_effectively_visible(self)
    }

    fn is_keyboard_focus_within(&self) -> bool {
        InputElement::is_keyboard_focus_within(self)
    }

    fn is_focused(&self) -> bool {
        InputElement::is_focused(self)
    }

    fn is_hit_test_visible(&self) -> bool {
        InputElement::is_hit_test_visible(self)
    }

    fn is_pointer_over(&self) -> bool {
        InputElement::is_pointer_over(self)
    }

    fn focus(&self, method: NavigationMethod, key_modifiers: KeyModifiers) -> bool {
        InputElement::focus_with(self, method, key_modifiers)
    }
}
