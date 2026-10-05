use crate::i_clickable_control::{as_clickable_control, register_clickable_control};
use crate::i_command_source::register_command_source;
use crate::metadata::{PseudoClassesAttribute, TemplatePartAttribute};
use crate::primitives::{
    FlyoutBase, Popup, PopupFlyoutBase, TemplateAppliedEventArgs, TemplatedControlImpl,
    TemplatedControlImplExt,
};
use crate::{Button, ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::navigation::XYFocusHelpers;
use ferroui_base::input::{
    IClickableControl, ICommand, ICommandSource, InputElement, InputElementImpl, InputElementImplExt, Key,
    KeyEventArgs, KeyGesture, KeyModifiers, PointerPressedEventArgs,
};
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::utilities::WeakEvents;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, BoxedValue,
    FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Nullable, Ref, StaticType, StyledElementImpl,
    StyledElementImplExt, StyledProperty, VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) const PC_CHECKED: &str = ":checked";
pub(crate) const PC_PRESSED: &str = ":pressed";
pub(crate) const PC_FLYOUT_OPEN: &str = ":flyout-open";

/// The secondary button of the template and the handlers attached to it:
/// the click handler and the pointer pressed (tunnel) handler.
type SecondaryButton = (Ref<Button>, RoutedEventHandlerToken, RoutedEventHandlerToken);

/// A button with primary and secondary parts that can each be pressed
/// separately. The primary part behaves like a [`Button`] and the secondary
/// part opens a flyout.
#[repr(C)]
pub struct SplitButton {
    base: ContentControl,
    primary_button: RefCell<Option<(Ref<Button>, RoutedEventHandlerToken)>>,
    secondary_button: RefCell<Option<SecondaryButton>>,
    hotkey: Cell<Option<KeyGesture>>,
    can_execute_change_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    command_can_execute: Cell<bool>,
    is_attached_to_logical_tree: Cell<bool>,
    is_flyout_open: Cell<bool>,
    is_keyboard_pressed: Cell<bool>,
    /// The subscriptions to the `Opened` and `Closed` events of the flyout.
    flyout_events: RefCell<Option<(Rc<dyn IDisposable>, Rc<dyn IDisposable>)>>,
    flyout_property_changed_disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    flyout_state_changed: HandlerList<dyn Fn()>,
}

ferro_class! {
    SplitButton: ContentControl, virtuals SplitButtonImpl: ContentControlImpl {
        /// A value indicating whether the button is currently checked.
        ///
        /// This member exists only for the derived toggle split button and
        /// is unused (false) within the split button. Doing this allows the
        /// two controls to share a default style.
        fn internal_is_checked(this) -> bool;
        /// Invokes the `Click` event when the primary button part is
        /// clicked. `e` are the event args from the internal click event.
        fn on_click_primary(this, e: Option<&RoutedEventArgs>);
        /// Invoked when the secondary button part is clicked. `e` are the
        /// event args from the internal click event.
        fn on_click_secondary(this, e: Option<&RoutedEventArgs>);
        /// Invoked when the split button's flyout is opened.
        fn on_flyout_opened(this);
        /// Invoked when the split button's flyout is closed.
        fn on_flyout_closed(this);
    }
}
ferroui_base::ferro_class_info!(SplitButton { new: SplitButton::new });

ferro_impl_classes!(SplitButton: VisualImpl, LayoutableImpl, InteractiveImpl, ContentControlImpl);

impl ControlImpl for SplitButton {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::SplitButtonAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for SplitButton {
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == Self::command_property().as_property() {
            // Must unregister events here while a reference to the old
            // command still exists.
            let (old_value, new_value) = e.get_old_and_new_value::<Option<Rc<dyn ICommand>>>();

            if this.is_attached_to_logical_tree.get() {
                if old_value.is_some() {
                    this.unsubscribe_can_execute_changed();
                }

                if let Some(new_command) = &new_value {
                    this.subscribe_can_execute_changed(new_command);
                }
            }

            this.can_execute_changed(new_value.as_ref(), this.command_parameter().as_ref());
        } else if e.property() == Self::command_parameter_property().as_property() && this.is_loaded() {
            let new_value = e.get_new_value::<Option<BoxedValue>>();
            this.can_execute_changed(this.command().as_ref(), new_value.as_ref());
        } else if e.property() == Self::flyout_property().as_property() {
            let (old_flyout, new_flyout) = e.get_old_and_new_value::<Option<Ref<FlyoutBase>>>();

            // If flyout is changed while one is already open, make sure we
            // close the old one first.
            // This is the same behavior as the button.
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
        }

        Self::parent_on_property_changed(this, e);
    }
}

impl StyledElementImpl for SplitButton {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);

        // Control attached again, set the hot key to create a hot key
        // manager for this control.
        this.set_current_value(Self::hot_key_property(), this.hotkey.get());

        if let Some(command) = this.command() {
            this.subscribe_can_execute_changed(&command);
            this.can_execute_changed_handler();
        }

        this.is_attached_to_logical_tree.set(true);
    }

    fn on_detached_from_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_logical_tree(this, e);

        // This will cause the hot key manager to dispose the observer and
        // the reference to this control.
        this.hotkey.set(this.hot_key());
        this.set_current_value(Self::hot_key_property(), None);

        if this.command().is_some() {
            this.unsubscribe_can_execute_changed();
        }

        this.is_attached_to_logical_tree.set(false);
    }
}

impl InputElementImpl for SplitButton {
    fn is_enabled_core(this: &Self) -> bool {
        Self::parent_is_enabled_core(this) && this.command_can_execute.get()
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let key = e.key;

        if (this.is_focused() && key == Key::Space) || key == Key::Enter {
            this.is_keyboard_pressed.set(true);
            this.update_pseudo_classes();
        }

        Self::parent_on_key_down(this, e);
    }

    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        let key = e.key;

        if (this.is_focused() && key == Key::Space) || key == Key::Enter {
            this.is_keyboard_pressed.set(false);
            this.update_pseudo_classes();

            // Consider this a click on the primary button.
            if this.is_effectively_enabled() {
                this.on_click_primary(None);
                e.set_handled(true);
            }
        } else if (key == Key::Down
            && e.key_modifiers.contains(KeyModifiers::ALT)
            && this.is_effectively_enabled()
            && !XYFocusHelpers::is_allowed_xy_navigation_mode(this, Some(e.key_device_type)))
            || (key == Key::F4 && this.is_effectively_enabled())
        {
            this.open_flyout();
            e.set_handled(true);
        } else if e.key == Key::Escape && this.is_flyout_open.get() {
            // If the flyout doesn't have focusable content, close the
            // flyout here. This is the same behavior as the button.
            this.close_flyout();
            e.set_handled(true);
        }

        Self::parent_on_key_up(this, e);
    }
}

impl TemplatedControlImpl for SplitButton {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        this.unregister_events();
        let flyout = this.flyout();
        this.unregister_flyout_events(flyout.as_ref());

        let primary_button = e.name_scope().find_as::<Button>("PART_PrimaryButton");
        let secondary_button = e.name_scope().find_as::<Button>("PART_SecondaryButton");

        let primary_button = primary_button.map(|primary_button| {
            let weak = this.to_ref().downgrade();
            let token = primary_button.click(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.primary_button_click(e);
                }
            });
            (primary_button, token)
        });
        *this.primary_button.borrow_mut() = primary_button;

        let secondary_button = secondary_button.map(|secondary_button| {
            let weak = this.to_ref().downgrade();
            let click = secondary_button.click(move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.secondary_button_click(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let pressed = secondary_button.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |_, e: &PointerPressedEventArgs| {
                    if let Some(this) = weak.upgrade() {
                        this.secondary_button_preview_pointer_pressed(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            (secondary_button, click, pressed)
        });
        *this.secondary_button.borrow_mut() = secondary_button;

        this.register_flyout_events(flyout.as_ref());
        this.update_pseudo_classes();
    }
}

impl SplitButtonImpl for SplitButton {
    fn internal_is_checked(_this: &Self) -> bool {
        false
    }

    fn on_click_primary(this: &Self, _e: Option<&RoutedEventArgs>) {
        let (command, parameter) = (this.command(), this.command_parameter());
        // Note: It is not currently required to check enabled status;
        // however, this is a failsafe.
        if this.is_effectively_enabled() {
            let event_args = RoutedEventArgs::with_event(Self::click_event());
            this.raise_event(&event_args);

            if let Some(command) = command {
                if !event_args.handled() && command.can_execute(parameter.as_ref()) {
                    command.execute(parameter.as_ref());
                    event_args.set_handled(true);
                }
            }
        }
    }

    fn on_click_secondary(this: &Self, _e: Option<&RoutedEventArgs>) {
        // Note: It is not currently required to check enabled status;
        // however, this is a failsafe.
        if this.is_effectively_enabled() {
            if this.is_flyout_open.get() {
                this.close_flyout();
            } else {
                this.open_flyout();
            }
        }
    }

    fn on_flyout_opened(_this: &Self) {
        // Available for derived types
    }

    fn on_flyout_closed(_this: &Self) {
        // Available for derived types
    }
}

/// The split button viewed as a command source and as a clickable control.
struct SplitButtonHandle(Ref<SplitButton>);

impl ICommandSource for SplitButtonHandle {
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

impl IClickableControl for SplitButtonHandle {
    fn click(&self, handler: Rc<dyn Fn(&RoutedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.0.click(move |_, e| handler(e));
        let button = self.0.downgrade();
        Disposable::create(move || {
            if let Some(button) = button.upgrade() {
                button.remove_handler(SplitButton::click_event(), token);
            }
        })
    }

    fn raise_click(&self) {
        let primary_button = self.0.primary_button();
        if let Some(primary_button) = primary_button.and_then(|button| as_clickable_control(&button)) {
            primary_button.raise_click();
        }
    }

    fn is_effectively_enabled(&self) -> bool {
        self.0.is_effectively_enabled()
    }
}

impl SplitButton {
    /// The named parts expected in the control template, in addition to
    /// those of the base class.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] = &[
        TemplatePartAttribute::new("PART_PrimaryButton", <Button as StaticType>::TYPE),
        TemplatePartAttribute::new("PART_SecondaryButton", <Button as StaticType>::TYPE),
    ];

    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_FLYOUT_OPEN, PC_PRESSED]);

    ferro_routed_event!(
        /// Defines the `Click` event.
        pub fn click_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<SplitButton, _>("Click", RoutingStrategies::BUBBLE)
        }
    );
}

ferroui_base::ferro_properties! { impl SplitButton {
    ferro_property!(
        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            Button::command_property().add_owner::<SplitButton>()
        }
    );

    ferro_property!(
        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            Button::command_parameter_property().add_owner::<SplitButton>()
        }
    );

    ferro_property!(
        /// Defines the `Flyout` property.
        pub fn flyout_property() -> StyledProperty<Option<Ref<FlyoutBase>>> {
            Button::flyout_property().add_owner::<SplitButton>()
        }
    );

    ferro_property!(
        /// Defines the `HotKey` property.
        pub fn hot_key_property() -> StyledProperty<Option<KeyGesture>> {
            Button::hot_key_property().add_owner::<SplitButton>()
        }
    );
} }

impl SplitButton {
    fn static_constructor() {
        register_command_source::<SplitButton>(|button| Rc::new(SplitButtonHandle(button)));
        register_clickable_control::<SplitButton>(|button| Rc::new(SplitButtonHandle(button)));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            primary_button: RefCell::new(None),
            secondary_button: RefCell::new(None),
            hotkey: Cell::new(None),
            can_execute_change_subscription: RefCell::new(None),
            command_can_execute: Cell::new(true),
            is_attached_to_logical_tree: Cell::new(false),
            is_flyout_open: Cell::new(false),
            is_keyboard_pressed: Cell::new(false),
            flyout_events: RefCell::new(None),
            flyout_property_changed_disposable: RefCell::new(None),
            flyout_state_changed: HandlerList::new(),
        }
    }

    /// Creates a split button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the user presses the primary part of the split button.
    pub fn click(&self, handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static) -> RoutedEventHandlerToken {
        self.add_handler(Self::click_event(), handler)
    }

    /// The command invoked when the primary part is pressed.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// A parameter to be passed to the `Command`.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// The flyout that will be shown when the secondary part is pressed.
    pub fn flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::flyout_property())
    }

    pub fn set_flyout(&self, value: impl Into<Nullable<FlyoutBase>>) {
        self.set_value(Self::flyout_property(), value.into().0)
    }

    /// Raised when the flyout of the split button opens or closes
    /// (internal in the reference). Disposing the returned handle
    /// unsubscribes.
    pub(crate) fn flyout_state_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.flyout_state_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.flyout_state_changed.remove(token);
            }
        })
    }

    /// Invokes the primary button part (internal in the reference).
    pub(crate) fn invoke_primary(&self) {
        self.on_click_primary(None);
    }

    /// Whether the flyout of the split button is open (internal in the
    /// reference).
    pub(crate) fn is_flyout_open(&self) -> bool {
        self.is_flyout_open.get()
    }

    /// Opens the flyout on behalf of the automation peer (internal in the
    /// reference).
    pub(crate) fn open_flyout_for_automation(&self) {
        self.open_flyout();
    }

    /// Closes the flyout on behalf of the automation peer (internal in the
    /// reference).
    pub(crate) fn close_flyout_for_automation(&self) {
        self.close_flyout();
    }

    /// A key gesture associated with this control.
    pub fn hot_key(&self) -> Option<KeyGesture> {
        self.get_value(Self::hot_key_property())
    }

    pub fn set_hot_key(&self, value: Option<KeyGesture>) {
        self.set_value(Self::hot_key_property(), value)
    }

    fn primary_button(&self) -> Option<Ref<Button>> {
        self.primary_button.borrow().as_ref().map(|(button, _)| button.clone())
    }

    fn subscribe_can_execute_changed(&self, command: &Rc<dyn ICommand>) {
        // A weak subscription: it ends with this control, so a command
        // that outlives the control keeps no handler of it.
        let subscription = WeakEvents::command_can_execute_changed(command, &self.to_ref(), |this| {
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
        let (command, parameter) = (self.command(), self.command_parameter());
        self.can_execute_changed(command.as_ref(), parameter.as_ref());
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

    /// Updates the visual state of the control by applying the latest
    /// pseudoclasses.
    pub fn update_pseudo_classes(&self) {
        self.pseudo_classes().set(PC_FLYOUT_OPEN, self.is_flyout_open.get());
        self.pseudo_classes().set(PC_PRESSED, self.is_keyboard_pressed.get());
        self.pseudo_classes().set(PC_CHECKED, self.internal_is_checked());
    }

    /// Opens the secondary button's flyout.
    pub fn open_flyout(&self) {
        if let Some(flyout) = self.flyout() {
            flyout.show_at(self);
        }
    }

    /// Closes the secondary button's flyout.
    pub fn close_flyout(&self) {
        if let Some(flyout) = self.flyout() {
            flyout.hide();
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
            let closed = flyout.closed({
                let weak = weak.clone();
                move || {
                    if let (Some(this), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) {
                        this.flyout_closed(&flyout);
                    }
                }
            });
            *self.flyout_events.borrow_mut() = Some((opened, closed));

            let subscription =
                flyout.get_property_changed_observable(Popup::placement_property().as_property()).subscribe(move |_| {
                    if let Some(this) = weak.upgrade() {
                        this.flyout_placement_property_changed();
                    }
                });
            *self.flyout_property_changed_disposable.borrow_mut() = Some(subscription);
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

            let subscription = self.flyout_property_changed_disposable.take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }

    /// Explicitly unregisters all events related to the two buttons of the
    /// template.
    fn unregister_events(&self) {
        let primary_button = self.primary_button.borrow_mut().take();
        if let Some((primary_button, click)) = primary_button {
            primary_button.remove_handler(Button::click_event(), click);
        }

        let secondary_button = self.secondary_button.borrow_mut().take();
        if let Some((secondary_button, click, pressed)) = secondary_button {
            secondary_button.remove_handler(Button::click_event(), click);
            secondary_button.remove_handler(InputElement::pointer_pressed_event(), pressed);
        }
    }


    /// Called when the placement of the flyout changes.
    fn flyout_placement_property_changed(&self) {
        self.update_pseudo_classes();
    }

    /// Whether `flyout` is currently shown at this split button.
    fn is_flyout_target(&self, flyout: &FlyoutBase) -> bool {
        let this: Ref<crate::Control> = self.to_ref().upcast();
        flyout.target() == Some(this)
    }

    /// Event handler for when the split button's flyout is opened.
    fn flyout_opened(&self, flyout: &FlyoutBase) {
        // It is possible to share flyouts among multiple controls including
        // split buttons. This can cause a problem here since all controls
        // that share a flyout receive the same Opened/Closed events at the
        // same time. For split buttons that means they all would be
        // updating their pseudoclasses accordingly. In other words, all
        // split buttons with a shared flyout would have the backgrounds
        // changed together. To fix this, only continue here if the flyout
        // target matches this split button instance.
        if self.is_flyout_target(flyout) {
            self.is_flyout_open.set(true);
            self.update_pseudo_classes();
            for (_, handler) in self.flyout_state_changed.snapshot().iter() {
                handler();
            }

            self.on_flyout_opened();
        }
    }

    /// Event handler for when the split button's flyout is closed.
    fn flyout_closed(&self, flyout: &FlyoutBase) {
        // See comments in `flyout_opened`
        if self.is_flyout_target(flyout) {
            self.is_flyout_open.set(false);
            self.update_pseudo_classes();
            for (_, handler) in self.flyout_state_changed.snapshot().iter() {
                handler();
            }

            self.on_flyout_closed();
        }
    }

    /// Event handler for when the internal primary button part is clicked.
    fn primary_button_click(&self, e: &RoutedEventArgs) {
        // Handle the internal button click, so it won't bubble outside
        // together with the click event of the split button.
        e.set_handled(true);
        self.on_click_primary(Some(e));
    }

    /// Event handler for when the internal secondary button part is
    /// clicked.
    fn secondary_button_click(&self, e: &RoutedEventArgs) {
        // Handle the internal button click, so it won't bubble outside.
        e.set_handled(true);
        self.on_click_secondary(Some(e));
    }

    /// Event handler for when the internal secondary button part is
    /// pressed.
    fn secondary_button_preview_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let secondary_button = self.secondary_button.borrow().as_ref().map(|(button, _, _)| button.clone());
        let Some(secondary_button) = secondary_button else { return };

        if self.is_flyout_open.get()
            && secondary_button.is_effectively_enabled()
            && e.get_current_point(Some(&secondary_button)).properties.is_left_button_pressed
        {
            // When a flyout is open with pass-through of the overlay
            // dismiss event enabled and the secondary button is pressed,
            // close the flyout.
            e.set_handled(true);
            self.on_click_secondary(Some(e));
        }
    }
}
