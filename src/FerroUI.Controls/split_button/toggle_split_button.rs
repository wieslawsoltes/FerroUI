use super::split_button::PC_CHECKED;
use super::{SplitButton, SplitButtonImpl, SplitButtonImplExt};
use crate::metadata::PseudoClassesAttribute;
use crate::primitives::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl};
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{
    Interactive, InteractiveImpl, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, ferro_routed_event, instantiate, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl,
    StyledProperty, StyledPropertyOptions, TypeInfo, VisualImpl,
};

/// A button with primary and secondary parts that can each be pressed
/// separately. The primary part behaves like a toggle button with two
/// states and the secondary part opens a flyout.
#[repr(C)]
pub struct ToggleSplitButton {
    base: SplitButton,
}

ferro_class! {
    ToggleSplitButton: SplitButton, virtuals ToggleSplitButtonImpl: SplitButtonImpl {
        /// Invokes the `IsCheckedChanged` event when the `IsChecked`
        /// property changes.
        fn on_is_checked_changed(this);
    }
}
ferroui_base::ferro_class_info!(ToggleSplitButton { new: ToggleSplitButton::new });

ferro_impl_classes!(
    ToggleSplitButton: VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for ToggleSplitButton {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ToggleSplitButtonAutomationPeer::new(this).upcast()
    }
}

impl FerroObjectImpl for ToggleSplitButton {
    fn on_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);

        if e.property() == Self::is_checked_property().as_property() {
            this.on_is_checked_changed();
        }
    }
}

impl StyledElementImpl for ToggleSplitButton {
    /// Both the toggle split button and the split button share the same
    /// exact default style.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <SplitButton as StaticType>::TYPE
    }
}

impl SplitButtonImpl for ToggleSplitButton {
    fn internal_is_checked(this: &Self) -> bool {
        this.is_checked()
    }

    fn on_click_primary(this: &Self, e: Option<&RoutedEventArgs>) {
        this.toggle();

        Self::parent_on_click_primary(this, e);
    }
}

impl ToggleSplitButtonImpl for ToggleSplitButton {
    fn on_is_checked_changed(this: &Self) {
        // IsLoaded check
        if this.parent().is_some() {
            let event_args = RoutedEventArgs::with_event(Self::is_checked_changed_event());
            this.raise_event(&event_args);
        }

        this.update_pseudo_classes();
    }
}

impl ToggleSplitButton {
    /// The pseudoclasses set by the class.
    pub const PSEUDO_CLASSES: PseudoClassesAttribute = PseudoClassesAttribute::new(&[PC_CHECKED]);

    ferro_routed_event!(
        /// Defines the `IsCheckedChanged` event.
        pub fn is_checked_changed_event() -> RoutedEvent<RoutedEventArgs> {
            RoutedEvent::register::<ToggleSplitButton, _>("IsCheckedChanged", RoutingStrategies::BUBBLE)
        }
    );
}

ferroui_base::ferro_properties! { impl ToggleSplitButton {
    ferro_property!(
        /// Defines the `IsChecked` property.
        pub fn is_checked_property() -> StyledProperty<bool> {
            FerroProperty::register_with::<ToggleSplitButton, _>(
                "IsChecked",
                StyledPropertyOptions::new(false).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );
} }

impl ToggleSplitButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: SplitButton::construct() }
    }

    /// Creates a toggle split button.
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

    /// A value indicating whether the toggle split button is checked.
    pub fn is_checked(&self) -> bool {
        self.get_value(Self::is_checked_property())
    }

    pub fn set_is_checked(&self, value: bool) {
        self.set_value(Self::is_checked_property(), value)
    }

    /// Toggles the `IsChecked` property between true and false.
    pub fn toggle(&self) {
        self.set_current_value(Self::is_checked_property(), !self.is_checked());
    }

    /// Toggles the button on behalf of the automation peer (internal in the
    /// reference).
    pub(crate) fn toggle_for_automation(&self) {
        self.on_click_primary(None);
    }
}
