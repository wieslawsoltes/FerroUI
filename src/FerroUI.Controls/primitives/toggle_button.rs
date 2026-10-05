use super::TemplatedControlImpl;
use crate::metadata::PseudoClassesAttribute;
use crate::{Button, ButtonImpl, ButtonImplExt, ContentControlImpl, ControlImpl};
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
    StyledPropertyOptions, VisualImpl,
};

const PC_CHECKED: &str = ":checked";
const PC_UNCHECKED: &str = ":unchecked";
const PC_INDETERMINATE: &str = ":indeterminate";

/// Represents a control that a user can select (check) or clear (uncheck).
/// Base class for controls that can switch states.
#[repr(C)]
pub struct ToggleButton {
    base: Button,
}

ferro_class! {
    ToggleButton: Button, virtuals ToggleButtonImpl: ButtonImpl {
        /// Toggles the `IsChecked` property.
        fn toggle(this);
        /// Called when `IsChecked` changes. `e` are the event arguments for
        /// the routed event that is raised by the default implementation of
        /// this method.
        fn on_is_checked_changed(this, e: &RoutedEventArgs);
    }
}
ferroui_base::ferro_class_info!(ToggleButton { new: ToggleButton::new });

ferro_impl_classes!(
    ToggleButton: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for ToggleButton {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ToggleButtonAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for ToggleButton {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.update_pseudo_classes(this.is_checked());
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::is_checked_property().as_property() {
            let new_value = change.get_new_value::<Option<bool>>();
            this.update_pseudo_classes(new_value);
            this.on_is_checked_changed(&RoutedEventArgs::with_event(Self::is_checked_changed_event()));
        }
    }
}

impl ButtonImpl for ToggleButton {
    fn on_click(this: &Self) {
        if !this.is_effectively_enabled() {
            return;
        }

        this.toggle();
        Self::parent_on_click(this);
    }
}

impl ToggleButtonImpl for ToggleButton {
    fn toggle(this: &Self) {
        let new_value = match this.is_checked() {
            Some(true) => {
                if this.is_three_state() {
                    None
                } else {
                    Some(false)
                }
            }
            Some(false) => Some(true),
            None => Some(false),
        };

        this.set_current_value(Self::is_checked_property(), new_value);
    }

    fn on_is_checked_changed(this: &Self, e: &RoutedEventArgs) {
        this.raise_event(e);
    }
}

impl ToggleButton {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute =
        PseudoClassesAttribute::new(&[PC_CHECKED, PC_UNCHECKED, PC_INDETERMINATE]);
}

ferroui_base::ferro_properties! { impl ToggleButton {
    ferro_property!(
        /// Defines the `IsChecked` property.
        pub fn is_checked_property() -> StyledProperty<Option<bool>> {
            FerroProperty::register_with::<ToggleButton, _>(
                "IsChecked",
                StyledPropertyOptions::new(Some(false)).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsThreeState` property.
        pub fn is_three_state_property() -> StyledProperty<bool> {
            FerroProperty::register::<ToggleButton, _>("IsThreeState", false)
        }
    );
} }

impl ToggleButton {
    ferro_routed_event!(
        /// Defines the `IsCheckedChanged` event.
        pub fn is_checked_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<ToggleButton, _>("IsCheckedChanged", RoutingStrategies::BUBBLE)
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Button::construct() }
    }

    /// Creates a toggle button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Raised when the `IsChecked` property value changes.
    pub fn is_checked_changed(
        &self,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        self.add_handler(Self::is_checked_changed_event(), handler)
    }

    /// Whether the toggle button is checked; `None` is the indeterminate
    /// state.
    pub fn is_checked(&self) -> Option<bool> {
        self.get_value(Self::is_checked_property())
    }

    pub fn set_is_checked(&self, value: Option<bool>) {
        self.set_value(Self::is_checked_property(), value)
    }

    /// A value that indicates whether the control supports three states.
    pub fn is_three_state(&self) -> bool {
        self.get_value(Self::is_three_state_property())
    }

    pub fn set_is_three_state(&self, value: bool) {
        self.set_value(Self::is_three_state_property(), value)
    }

    fn update_pseudo_classes(&self, is_checked: Option<bool>) {
        self.pseudo_classes().set(PC_CHECKED, is_checked == Some(true));
        self.pseudo_classes().set(PC_UNCHECKED, is_checked == Some(false));
        self.pseudo_classes().set(PC_INDETERMINATE, is_checked.is_none());
    }
}
