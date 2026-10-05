use crate::i_clickable_control::register_clickable_control;
use crate::i_command_source::register_command_source;
use crate::metadata::PseudoClassesAttribute;
use crate::platform::{FeedbackAction, FeedbackType, PlatformFeedback, PlatformFeedbackExtensions};
use crate::primitives::{
    FlyoutBase, PopupFlyoutBase, TemplateAppliedEventArgs, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::{ContentControl, ContentControlImpl, ControlImpl, HotKeyManager};
use ferroui_base::data::{BindingError, BindingValueType};
use ferroui_base::input::{
    AccessKeyEventArgs, AccessKeyHandler, AccessKeyPressedEventArgs, FocusChangedEventArgs, IClickableControl,
    ICommand, ICommandSource, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs, KeyGesture, MouseButton, PointerCaptureLostEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{
    IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken,
    RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::utilities::WeakEvents;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StyledElementImpl,
    StyledElementImplExt, StyledProperty, StyledPropertyOptions, Visual, VisualImpl, VisualImplExt,
    VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Defines how a [`Button`] reacts to clicks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClickMode {
    /// The `Click` event is raised when the pointer is released.
    #[default]
    Release,

    /// The `Click` event is raised when the pointer is pressed.
    Press,
}

const PC_PRESSED: &str = ":pressed";
const PC_FLYOUT_OPEN: &str = ":flyout-open";

/// A standard button control.
#[repr(C)]
pub struct Button {
    base: ContentControl,
    can_execute_change_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    default_key_down_handlers: RefCell<Vec<RoutedEventHandlerToken>>,
    cancel_key_down_handlers: RefCell<Vec<RoutedEventHandlerToken>>,
    command_can_execute: Cell<bool>,
    hotkey: Cell<Option<KeyGesture>>,
    is_flyout_open: Cell<bool>,
    is_pressed: Cell<bool>,
    /// The subscriptions to the `Opened` and `Closed` events of the flyout.
    flyout_events: RefCell<Option<(Rc<dyn IDisposable>, Rc<dyn IDisposable>)>>,
}

ferro_class! {
    Button: ContentControl, virtuals ButtonImpl: ContentControlImpl {
        /// Invokes the `Click` event.
        fn on_click(this);
        /// Opens the button's flyout.
        fn open_flyout(this);
        /// Closes the button's flyout.
        fn close_flyout(this);
        /// Invoked when the button's flyout is opened.
        fn on_flyout_opened(this);
        /// Invoked when the button's flyout is closed.
        fn on_flyout_closed(this);
    }
}
ferroui_base::ferro_class_info!(Button { new: Button::new });

ferro_impl_classes!(Button: LayoutableImpl, InteractiveImpl, ContentControlImpl);

impl ControlImpl for Button {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ButtonAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for Button {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::command_property().as_property() {
            let (old_value, new_value) = change.get_old_and_new_value::<Option<Rc<dyn ICommand>>>();
            if this.is_attached_to_logical_tree() {
                if old_value.is_some() {
                    this.unsubscribe_can_execute_changed();
                }

                if let Some(new_command) = &new_value {
                    this.subscribe_can_execute_changed(new_command);
                }
            }
            this.can_execute_changed(new_value.as_ref(), this.command_parameter().as_ref());
        } else if property == Self::command_parameter_property().as_property() {
            let new_value = change.get_new_value::<Option<BoxedValue>>();
            this.can_execute_changed(this.command().as_ref(), new_value.as_ref());
        } else if property == Self::is_cancel_property().as_property() {
            let is_cancel = change.get_new_value::<bool>();

            if let Some(input_root) = this.visual_root().and_then(|root| root.downcast::<InputElement>().ok()) {
                if is_cancel {
                    this.listen_for_cancel(&input_root);
                } else {
                    this.stop_listening_for_cancel(&input_root);
                }
            }
        } else if property == Self::is_default_property().as_property() {
            let is_default = change.get_new_value::<bool>();

            if let Some(input_root) = this.visual_root().and_then(|root| root.downcast::<InputElement>().ok()) {
                if is_default {
                    this.listen_for_default(&input_root);
                } else {
                    this.stop_listening_for_default(&input_root);
                }
            }
        } else if property == Self::is_pressed_property().as_property() {
            this.update_pseudo_classes();
        } else if property == Self::flyout_property().as_property() {
            let (old_flyout, new_flyout) = change.get_old_and_new_value::<Option<Ref<FlyoutBase>>>();

            // If flyout is changed while one is already open, make sure we
            // close the old one first
            if let Some(old_flyout) = &old_flyout {
                if old_flyout.is_open() {
                    old_flyout.hide();
                }

                if let Some(old_flyout) = old_flyout.downcast_ref::<PopupFlyoutBase>() {
                    old_flyout.set_default_placement_target(None);
                }
            }

            // Must unregister events here while a reference to the old flyout still exists
            this.unregister_flyout_events(old_flyout.as_ref());

            this.register_flyout_events(new_flyout.as_ref());
            if let Some(new_flyout) = new_flyout.as_ref().and_then(|flyout| flyout.downcast_ref::<PopupFlyoutBase>()) {
                new_flyout.set_default_placement_target(Some(this));
            }
            this.update_pseudo_classes();
        } else if property == InputElement::is_effectively_enabled_property().as_property()
            && !change.get_new_value::<bool>()
        {
            this.set_is_pressed(false);
        }
    }

    fn update_data_validation(
        this: &Self,
        property: &'static FerroProperty,
        state: BindingValueType,
        error: Option<&BindingError>,
    ) {
        Self::parent_update_data_validation(this, property, state, error);

        if property == Self::command_property().as_property()
            && state == BindingValueType::BINDING_ERROR
            && this.command_can_execute.get()
        {
            this.command_can_execute.set(false);
            this.update_is_effectively_enabled();
        }
    }
}

impl StyledElementImpl for Button {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        // Control attached again, set the hot key to create a hot key
        // manager for this control.
        if let Some(hotkey) = this.hotkey.get() {
            this.set_current_value(Self::hot_key_property(), Some(hotkey));
        }

        Self::parent_on_attached_to_logical_tree(this, e);

        let (command, parameter) = (this.command(), this.command_parameter());
        if let Some(command) = &command {
            this.subscribe_can_execute_changed(command);
            this.can_execute_changed(Some(command), parameter.as_ref());
        }
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        // This will cause the hot key manager to dispose the observer and
        // the reference to this control.
        if let Some(hotkey) = this.hot_key() {
            this.hotkey.set(Some(hotkey));
            this.set_current_value(Self::hot_key_property(), None);
        }

        Self::parent_on_detached_from_logical_tree(this, e);

        if this.command().is_some() {
            this.unsubscribe_can_execute_changed();
        }
    }
}

impl VisualImpl for Button {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.is_default() {
            if let Some(input_element) = e.root_visual().cast::<InputElement>() {
                this.listen_for_default(&input_element);
            }
        }
        if this.is_cancel() {
            if let Some(input_element) = e.root_visual().cast::<InputElement>() {
                this.listen_for_cancel(&input_element);
            }
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if this.is_default() {
            if let Some(input_element) = e.root_visual().cast::<InputElement>() {
                this.stop_listening_for_default(&input_element);
            }
        }
        if this.is_cancel() {
            if let Some(input_element) = e.root_visual().cast::<InputElement>() {
                this.stop_listening_for_cancel(&input_element);
            }
        }
    }
}

impl InputElementImpl for Button {
    fn is_enabled_core(this: &Self) -> bool {
        Self::parent_is_enabled_core(this) && this.command_can_execute.get()
    }

    fn on_access_key(this: &Self, e: &dyn IRoutedEventArgs) {
        if e.downcast_ref::<AccessKeyEventArgs>().is_some_and(AccessKeyEventArgs::is_multiple) {
            Self::parent_on_access_key(this, e);
        } else {
            this.on_click();
        }
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        match e.key {
            Key::Enter => {
                this.on_click();
                e.set_handled(true);
            }
            Key::Space => {
                // Avoid handling Space if the button isn't focused: a child
                // text box might need it for text input.
                if this.is_focused() {
                    if this.click_mode() == ClickMode::Press {
                        this.on_click();
                    }

                    this.set_is_pressed(true);
                    e.set_handled(true);
                }
            }
            Key::Escape if this.flyout().is_some() => {
                // If Flyout doesn't have focusable content, close the flyout here
                this.close_flyout();
            }
            _ => {}
        }

        Self::parent_on_key_down(this, e);
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        // Avoid handling Space if the button isn't focused: a child text
        // box might need it for text input.
        if e.key == Key::Space && this.is_focused() {
            if this.click_mode() == ClickMode::Release {
                this.on_click();
            }
            this.set_is_pressed(false);
            e.set_handled(true);
        }

        Self::parent_on_key_up(this, e);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if e.get_current_point(Some(this)).properties.is_left_button_pressed {
            if this.is_flyout_open.get() && this.is_effectively_enabled() {
                // When a flyout is open with pass-through of the overlay
                // dismiss event enabled and the button is pressed, close
                // the flyout, but do not transition to a pressed state.
                e.set_handled(true);
                this.on_click();
            } else {
                this.set_is_pressed(true);
                e.set_handled(true);

                if this.click_mode() == ClickMode::Press {
                    this.on_click();
                }
            }
        }
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        Self::parent_on_pointer_released(this, e);

        if this.is_pressed() && e.initial_press_mouse_button() == MouseButton::Left {
            this.set_is_pressed(false);
            e.set_handled(true);

            if this.click_mode() == ClickMode::Release {
                let this_visual: &Visual = this;
                let is_over = this.get_visuals_at(e.get_position(Some(this))).iter().any(|c| {
                    std::ptr::eq::<Visual>(&**c, this_visual) || this.is_visual_ancestor_of(c)
                });

                if is_over {
                    this.on_click();
                }
            }
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        Self::parent_on_pointer_capture_lost(this, e);

        this.set_is_pressed(false);
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);

        this.set_is_pressed(false);
    }
}

impl TemplatedControlImpl for Button {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let flyout = this.flyout();
        this.unregister_flyout_events(flyout.as_ref());
        this.register_flyout_events(flyout.as_ref());
        this.update_pseudo_classes();
    }
}

impl ButtonImpl for Button {
    fn on_click(this: &Self) {
        if this.is_effectively_enabled() {
            if this.is_flyout_open.get() {
                this.close_flyout();
            } else {
                this.open_flyout();
            }

            this.perform_feedback(FeedbackAction::click());

            let e = RoutedEventArgs::with_event(Self::click_event());
            this.raise_event(&e);

            let (command, parameter) = (this.command(), this.command_parameter());
            if let Some(command) = command {
                if !e.handled() && command.can_execute(parameter.as_ref()) {
                    command.execute(parameter.as_ref());
                    e.set_handled(true);
                }
            }
        }
    }

    fn open_flyout(this: &Self) {
        if let Some(flyout) = this.flyout() {
            flyout.show_at(this);
        }
    }

    fn close_flyout(this: &Self) {
        if let Some(flyout) = this.flyout() {
            flyout.hide();
        }
    }

    fn on_flyout_opened(_this: &Self) {
        // Available for derived types
    }

    fn on_flyout_closed(_this: &Self) {
        // Available for derived types
    }
}

/// The button viewed as a command source and as a clickable control.
struct ButtonHandle(Ref<Button>);

impl ICommandSource for ButtonHandle {
    fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.0.command()
    }

    fn command_parameter(&self) -> Option<BoxedValue> {
        self.0.command_parameter()
    }

    fn can_execute_changed(&self) {
        self.0.can_execute_changed_handler();
    }

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

impl IClickableControl for ButtonHandle {
    fn click(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.0.click(move |_, e| handler(e));
        let button = self.0.downgrade();
        Disposable::create(move || {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        })
    }

    fn raise_click(&self) {
        self.0.on_click();
    }

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

impl Button {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_FLYOUT_OPEN, PC_PRESSED]);
}

ferroui_base::ferro_properties! { impl Button, also [
    Button::is_pressed_property,
    Button::flyout_property,
] {
    ferro_property!(
        /// Defines the `ClickMode` property.
        pub fn click_mode_property() -> StyledProperty<ClickMode> {
            FerroProperty::register::<Button, _>("ClickMode", ClickMode::Release)
        }
    );

    ferro_property!(
        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register_with::<Button, _>(
                "Command",
                StyledPropertyOptions::new(None).enable_data_validation(true),
            )
        }
    );

    ferro_property!(
        /// Defines the `HotKey` property.
        pub fn hot_key_property() -> StyledProperty<Option<KeyGesture>> {
            HotKeyManager::hot_key_property().add_owner::<Button>()
        }
    );

    ferro_property!(
        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<Button, _>("CommandParameter", None)
        }
    );

    ferro_property!(
        /// Defines the `IsDefault` property.
        pub fn is_default_property() -> StyledProperty<bool> {
            FerroProperty::register::<Button, _>("IsDefault", false)
        }
    );

    ferro_property!(
        /// Defines the `IsCancel` property.
        pub fn is_cancel_property() -> StyledProperty<bool> {
            FerroProperty::register::<Button, _>("IsCancel", false)
        }
    );
} }

impl Button {
    ferro_routed_event!(
        /// Defines the `Click` event.
        pub fn click_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<Button, _>("Click", RoutingStrategies::BUBBLE)
        }
    );

    ferro_property!(for Button;
        /// Defines the `IsPressed` property.
        pub fn is_pressed_property() -> DirectProperty<Button, bool> {
            FerroProperty::register_direct::<Button, bool>("IsPressed", |b| b.is_pressed(), None, false)
        }
    );

    ferro_property!(for Button;
        /// Defines the `Flyout` property.
        pub fn flyout_property() -> StyledProperty<Option<Ref<FlyoutBase>>> {
            FerroProperty::register::<Button, _>("Flyout", None)
        }
    );

    fn static_constructor() {
        register_command_source::<Button>(|button| Rc::new(ButtonHandle(button)));
        register_clickable_control::<Button>(|button| Rc::new(ButtonHandle(button)));

        InputElement::focusable_property().override_default_value::<Button>(true);
        PlatformFeedback::feedback_type_property().override_default_value::<Button>(FeedbackType::Auto);
        AccessKeyHandler::access_key_pressed_event().add_class_handler::<Button>(Self::on_access_key_pressed);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            can_execute_change_subscription: RefCell::new(None),
            default_key_down_handlers: RefCell::new(Vec::new()),
            cancel_key_down_handlers: RefCell::new(Vec::new()),
            command_can_execute: Cell::new(true),
            hotkey: Cell::new(None),
            is_flyout_open: Cell::new(false),
            is_pressed: Cell::new(false),
            flyout_events: RefCell::new(None),
        }
    }

    /// Creates a button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the user clicks the button.
    pub fn click(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::click_event(), handler)
    }

    /// A value indicating how the button should react to clicks.
    pub fn click_mode(&self) -> ClickMode {
        self.get_value(Self::click_mode_property())
    }

    pub fn set_click_mode(&self, value: ClickMode) {
        self.set_value(Self::click_mode_property(), value)
    }

    /// A command to be invoked when the button is clicked.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// A key gesture associated with this control.
    pub fn hot_key(&self) -> Option<KeyGesture> {
        self.get_value(Self::hot_key_property())
    }

    pub fn set_hot_key(&self, value: Option<KeyGesture>) {
        self.set_value(Self::hot_key_property(), value)
    }

    /// A parameter to be passed to the `Command`.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// A value indicating whether the button is the default button for the
    /// root it is attached to.
    pub fn is_default(&self) -> bool {
        self.get_value(Self::is_default_property())
    }

    pub fn set_is_default(&self, value: bool) {
        self.set_value(Self::is_default_property(), value)
    }

    /// A value indicating whether the button is the Cancel button for the
    /// root it is attached to.
    pub fn is_cancel(&self) -> bool {
        self.get_value(Self::is_cancel_property())
    }

    pub fn set_is_cancel(&self, value: bool) {
        self.set_value(Self::is_cancel_property(), value)
    }

    /// A value indicating whether the button is currently pressed.
    #[inline]
    pub fn is_pressed(&self) -> bool {
        self.is_pressed.get()
    }

    fn set_is_pressed(&self, value: bool) {
        self.set_and_raise_cell(Self::is_pressed_property(), &self.is_pressed, value);
    }

    /// The flyout that should be shown with this button.
    pub fn flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::flyout_property())
    }

    pub fn set_flyout(&self, value: impl Into<Nullable<FlyoutBase>>) {
        self.set_value(Self::flyout_property(), value.into().0)
    }

    /// Clicks the button.
    #[allow(dead_code)]
    pub(crate) fn perform_click(&self) {
        self.on_click();
    }

    fn on_access_key_pressed(sender: &Button, e: &AccessKeyPressedEventArgs) {
        if e.handled() || e.target().is_some() {
            return;
        }
        e.set_target(Some(sender.to_ref().upcast()));
        e.set_handled(true);
    }

    fn subscribe_can_execute_changed(&self, command: &Rc<dyn ICommand>) {
        // A weak subscription: it ends with this control, so a command
        // that outlives the control keeps no handler of it.
        let subscription = WeakEvents::subscribe_command_can_execute_changed(command, &self.to_ref(), |this| {
            this.can_execute_changed_handler()
        });
        let old = self.can_execute_change_subscription.replace(Some(subscription));
        if let Some(old) = old {
            old.dispose();
        }
    }

    fn unsubscribe_can_execute_changed(&self) {
        let subscription = self.can_execute_change_subscription.take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    /// Called when the "can execute changed" event of the command fires.
    fn can_execute_changed_handler(&self) {
        self.can_execute_changed(self.command().as_ref(), self.command_parameter().as_ref());
    }

    fn can_execute_changed(&self, command: Option<&Rc<dyn ICommand>>, parameter: Option<&BoxedValue>) {
        if !self.is_attached_to_logical_tree() {
            return;
        }

        let can_execute = command.is_none_or(|command| command.can_execute(parameter));

        if can_execute != self.command_can_execute.get() {
            self.command_can_execute.set(can_execute);
            self.update_is_effectively_enabled();
        }
    }

    /// Starts listening for the Enter key when the button `is_default`.
    fn listen_for_default(&self, root: &InputElement) {
        let weak = self.to_ref().downgrade();
        let token = root.add_handler(InputElement::key_down_event(), move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.root_default_key_down(e);
            }
        });
        self.default_key_down_handlers.borrow_mut().push(token);
    }

    /// Starts listening for the Escape key when the button `is_cancel`.
    fn listen_for_cancel(&self, root: &InputElement) {
        let weak = self.to_ref().downgrade();
        let token = root.add_handler(InputElement::key_down_event(), move |_, e| {
            if let Some(this) = weak.upgrade() {
                this.root_cancel_key_down(e);
            }
        });
        self.cancel_key_down_handlers.borrow_mut().push(token);
    }

    /// Stops listening for the Enter key when the button is no longer
    /// `is_default`.
    fn stop_listening_for_default(&self, root: &InputElement) {
        let token = self.default_key_down_handlers.borrow_mut().pop();
        if let Some(token) = token {
            root.remove_handler(InputElement::key_down_event(), token);
        }
    }

    /// Stops listening for the Escape key when the button is no longer
    /// `is_cancel`.
    fn stop_listening_for_cancel(&self, root: &InputElement) {
        let token = self.cancel_key_down_handlers.borrow_mut().pop();
        if let Some(token) = token {
            root.remove_handler(InputElement::key_down_event(), token);
        }
    }

    /// Called when a key is pressed on the input root and the button
    /// `is_default`.
    fn root_default_key_down(&self, e: &KeyEventArgs) {
        if e.key == Key::Enter && self.is_effectively_visible() && self.is_effectively_enabled() {
            self.on_click();
            e.set_handled(true);
        }
    }

    /// Called when a key is pressed on the input root and the button
    /// `is_cancel`.
    fn root_cancel_key_down(&self, e: &KeyEventArgs) {
        if e.key == Key::Escape && self.is_effectively_visible() && self.is_effectively_enabled() {
            self.on_click();
            e.set_handled(true);
        }
    }

    /// Registers all flyout events.
    fn register_flyout_events(&self, flyout: Option<&Ref<FlyoutBase>>) {
        if let Some(flyout) = flyout {
            let weak = self.to_ref().downgrade();
            let weak_flyout = flyout.downgrade();
            let opened = flyout.opened({
                let (weak, weak_flyout) = (weak.clone(), weak_flyout.clone());
                move || {
                    if let (Some(this), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) {
                        this.flyout_opened(&flyout);
                    }
                }
            });
            let closed = flyout.closed(move || {
                if let (Some(this), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) {
                    this.flyout_closed(&flyout);
                }
            });
            *self.flyout_events.borrow_mut() = Some((opened, closed));
        }
    }

    /// Explicitly unregisters all flyout events.
    fn unregister_flyout_events(&self, flyout: Option<&Ref<FlyoutBase>>) {
        if flyout.is_some() {
            let flyout_events = self.flyout_events.take();
            if let Some((opened, closed)) = flyout_events {
                opened.dispose();
                closed.dispose();
            }
        }
    }

    /// Whether `flyout` is currently shown at this button.
    fn is_flyout_target(&self, flyout: &FlyoutBase) -> bool {
        let this: Ref<crate::Control> = self.to_ref().upcast();
        flyout.target() == Some(this)
    }

    /// Event handler for when the button's flyout is opened.
    fn flyout_opened(&self, flyout: &FlyoutBase) {
        // It is possible to share flyouts among multiple controls including
        // buttons. This can cause a problem here since all controls that
        // share a flyout receive the same Opened/Closed events at the same
        // time. For buttons that means they all would be updating their
        // pseudoclasses accordingly. In other words, all buttons with a
        // shared flyout would have the backgrounds changed together. To fix
        // this, only continue here if the flyout target matches this button
        // instance.
        if self.is_flyout_target(flyout) {
            self.is_flyout_open.set(true);
            self.update_pseudo_classes();

            self.on_flyout_opened();
        }
    }

    /// Event handler for when the button's flyout is closed.
    fn flyout_closed(&self, flyout: &FlyoutBase) {
        // See comments in `flyout_opened`
        if self.is_flyout_target(flyout) {
            self.is_flyout_open.set(false);
            self.update_pseudo_classes();

            self.on_flyout_closed();
        }
    }

    /// Updates the visual state of the control by applying the latest
    /// pseudoclasses.
    fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(PC_FLYOUT_OPEN, self.is_flyout_open.get());
        self.pseudo_classes().set(PC_PRESSED, self.is_pressed());
    }
}
