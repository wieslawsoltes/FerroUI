use crate::i_native_menu_item_exporter_events_impl_bridge::INativeMenuItemExporterEventsImplBridge;
use crate::utils::debug_display::{append_optional_value, debug_type_name};
use crate::{MenuItem, MenuItemToggleType, NativeMenu, NativeMenuItemBase};
use ferroui_base::input::{ICommand, KeyGesture};
use ferroui_base::media::imaging::IBitmap;
use ferroui_base::utilities::WeakEvents;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledProperty, StyledPropertyMetadata,
    StyledPropertyOptions, Visual,
};
use std::cell::RefCell;
use std::rc::Rc;

/// An item of a [`NativeMenu`].
#[repr(C)]
pub struct NativeMenuItem {
    base: NativeMenuItemBase,
    /// The subscription to the "can execute changed" event of the command.
    /// The handler holds the item weakly, so that a command does not keep
    /// the items that use it alive.
    can_execute_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    click: HandlerList<dyn Fn(&NativeMenuItem)>,
}

ferro_class!(NativeMenuItem: NativeMenuItemBase);
ferro_class_info!(NativeMenuItem { new: NativeMenuItem::new });

impl FerroObjectImpl for NativeMenuItem {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();

        if property == Self::menu_property().as_property() {
            if let Some(new_menu) = change.get_new_value::<Option<Ref<NativeMenu>>>() {
                if new_menu.parent().is_some_and(|parent| parent != this.to_ref()) {
                    panic!("NativeMenu already has a parent");
                }
                new_menu.set_parent(Some(this.to_ref()));
            }
        } else if property == Self::command_property().as_property() {
            let (old_command, new_command) = change.get_old_and_new_value::<Option<Rc<dyn ICommand>>>();
            if old_command.is_some() {
                let subscription = this.can_execute_changed_subscription.take();
                if let Some(subscription) = subscription {
                    subscription.dispose();
                }
            }
            if let Some(new_command) = new_command {
                // A weak subscription, as in the reference: it ends with
                // this item.
                let subscription =
                    WeakEvents::subscribe_command_can_execute_changed(&new_command, &this.to_ref(), |this| this.can_execute_changed());
                let old = this.can_execute_changed_subscription.replace(Some(subscription));
                if let Some(old) = old {
                    old.dispose();
                }
            }
            this.can_execute_changed();
        }
    }
}

/// The native menu item viewed through the interface it implements.
struct NativeMenuItemHandle(Ref<NativeMenuItem>);

impl INativeMenuItemExporterEventsImplBridge for NativeMenuItemHandle {
    fn raise_clicked(&self) {
        self.0.raise_clicked();
    }
}

ferro_properties! {
    impl NativeMenuItem {
        /// Defines the `Menu` property.
        pub fn menu_property() -> StyledProperty<Option<Ref<NativeMenu>>> {
            FerroProperty::register_with::<NativeMenuItem, _>(
                "Menu",
                StyledPropertyOptions::new(None).coerce(Self::coerce_menu),
            )
        }

        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<Rc<dyn IBitmap>>> {
            FerroProperty::register::<NativeMenuItem, _>("Icon", None)
        }

        /// Defines the `Header` property.
        pub fn header_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<NativeMenuItem, _>("Header", None)
        }

        /// Defines the `ToolTip` property.
        pub fn tool_tip_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<NativeMenuItem, _>("ToolTip", None)
        }

        /// Defines the `Gesture` property.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<NativeMenuItem, _>("Gesture", None)
        }

        /// Defines the `IsChecked` property.
        pub fn is_checked_property() -> StyledProperty<bool> {
            MenuItem::is_checked_property().add_owner::<NativeMenuItem>()
        }

        /// Defines the `ToggleType` property.
        pub fn toggle_type_property() -> StyledProperty<MenuItemToggleType> {
            FerroProperty::register::<NativeMenuItem, _>("ToggleType", MenuItemToggleType::None)
        }

        /// Defines the `Command` property.
        pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            MenuItem::command_property()
                .add_owner_with::<NativeMenuItem>(StyledPropertyMetadata::new(None).with_enable_data_validation(true))
        }

        /// Defines the `CommandParameter` property.
        pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            MenuItem::command_parameter_property().add_owner::<NativeMenuItem>()
        }

        /// Defines the `IsEnabled` property.
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<NativeMenuItem, _>("IsEnabled", true)
        }

        /// Defines the `IsVisible` property.
        pub fn is_visible_property() -> StyledProperty<bool> {
            Visual::is_visible_property().add_owner::<NativeMenuItem>()
        }
    }
}

impl NativeMenuItem {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: NativeMenuItemBase::construct(),
            can_execute_changed_subscription: RefCell::new(None),
            click: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an item with the given header.
    pub fn with_header(header: &str) -> Ref<Self> {
        let this = Self::new();
        this.set_header(Some(header.to_string()));
        this
    }

    /// The submenu of the item.
    pub fn menu(&self) -> Option<Ref<NativeMenu>> {
        self.get_value(Self::menu_property())
    }

    pub fn set_menu(&self, value: Option<Ref<NativeMenu>>) {
        self.set_value(Self::menu_property(), value)
    }

    fn coerce_menu(sender: &FerroObject, value: Option<Ref<NativeMenu>>) -> Option<Ref<NativeMenu>> {
        if let Some(parent) = value.as_ref().and_then(|value| value.parent()) {
            let parent: &FerroObject = &parent;
            if !std::ptr::eq(parent, sender) {
                panic!("NativeMenu already has a parent");
            }
        }
        value
    }

    /// The icon that appears in the item.
    pub fn icon(&self) -> Option<Rc<dyn IBitmap>> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<Rc<dyn IBitmap>>) {
        self.set_value(Self::icon_property(), value)
    }

    /// The header of the item.
    pub fn header(&self) -> Option<String> {
        self.get_value(Self::header_property())
    }

    pub fn set_header(&self, value: Option<String>) {
        self.set_value(Self::header_property(), value)
    }

    /// Gets the tooltip associated with the menu item. This may not be
    /// supported by the native menu provider, but will be passed on to the
    /// non-native fallback menu item if used.
    pub fn tool_tip(&self) -> Option<String> {
        self.get_value(Self::tool_tip_property())
    }

    pub fn set_tool_tip(&self, value: Option<String>) {
        self.set_value(Self::tool_tip_property(), value)
    }

    /// The input gesture that will be displayed in the item.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }

    /// Whether the item is checked.
    pub fn is_checked(&self) -> bool {
        self.get_value(Self::is_checked_property())
    }

    pub fn set_is_checked(&self, value: bool) {
        self.set_value(Self::is_checked_property(), value)
    }

    /// How the item reacts to clicks.
    pub fn toggle_type(&self) -> MenuItemToggleType {
        self.get_value(Self::toggle_type_property())
    }

    pub fn set_toggle_type(&self, value: MenuItemToggleType) {
        self.set_value(Self::toggle_type_property(), value)
    }

    /// Whether the item is enabled.
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }

    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value)
    }

    /// Gets a value indicating whether this menu item is visible.
    pub fn is_visible(&self) -> bool {
        self.get_value(Self::is_visible_property())
    }

    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value)
    }

    // Objects of the framework belong to the thread that created them, so
    // the notification of the command always arrives on that thread; the
    // reference re-dispatches one that arrives on another thread.
    fn can_execute_changed(&self) {
        let (command, parameter) = (self.command(), self.command_parameter());
        let can_execute = command.is_none_or(|command| command.can_execute(parameter.as_ref()));
        self.set_current_value(Self::is_enabled_property(), can_execute);
    }

    /// Whether the `click` event has handlers.
    pub fn has_click_handlers(&self) -> bool {
        !self.click.is_empty()
    }

    /// The command to invoke when the item is clicked.
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value)
    }

    /// The parameter to pass to the command.
    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value)
    }

    /// Occurs when a [`NativeMenuItem`] is clicked.
    pub fn click(&self, handler: impl Fn(&NativeMenuItem) + 'static) -> Rc<dyn IDisposable> {
        let token = self.click.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.click.remove(token);
            }
        })
    }

    /// The item viewed as the contract through which the platform
    /// exporter of the menu raises the events of the item.
    pub fn to_exporter_events_bridge(&self) -> Rc<dyn INativeMenuItemExporterEventsImplBridge> {
        Rc::new(NativeMenuItemHandle(self.to_ref()))
    }

    fn raise_clicked(&self) {
        for (_, handler) in self.click.snapshot().iter() {
            handler(self);
        }

        // The command and its parameter are read again for the execution,
        // as in the reference: "can execute" may have changed them.
        if self.command().is_some_and(|command| command.can_execute(self.command_parameter().as_ref())) {
            if let Some(command) = self.command() {
                command.execute(self.command_parameter().as_ref());
            }
        }
    }

    /// The text that describes the item in diagnostics.
    pub(crate) fn build_debug_display(&self, builder: &mut String, include_content: bool) {
        builder.push_str(&debug_type_name(self));

        if include_content {
            append_optional_value(builder, "Header", self.header().as_deref());
        }
    }
}
