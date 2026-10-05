use super::{CommandBarDefaultLabelPosition, ICommandBarElement};
use crate::primitives::TemplatedControlImpl;
use crate::Button;
use crate::{ButtonImpl, ContentControlImpl, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::rc::Rc;

/// A button for use in a [`CommandBar`](super::CommandBar).
#[repr(C)]
pub struct CommandBarButton {
    base: Button,
}

ferro_class!(CommandBarButton: Button);
ferro_class_info!(CommandBarButton {
    new: CommandBarButton::new,
    interfaces: [
        Rc<dyn ICommandBarElement> => |button: Ref<CommandBarButton>| button.as_command_bar_element(),
    ],
});
ferro_impl_classes!(
    CommandBarButton: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl
);

/// Implements the command bar element contract for the button behind a
/// class handle.
struct CommandBarButtonElement(Ref<CommandBarButton>);

impl ICommandBarElement for CommandBarButtonElement {
    fn is_compact(&self) -> bool {
        self.0.is_compact()
    }

    fn set_is_compact(&self, value: bool) {
        self.0.set_is_compact(value)
    }

    fn is_in_overflow(&self) -> bool {
        self.0.is_in_overflow()
    }

    fn set_is_in_overflow(&self, value: bool) {
        self.0.set_is_in_overflow(value)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        let object: &FerroObject = &self.0;
        Some(object)
    }
}

ferro_properties! {
    impl CommandBarButton {
        /// Defines the `Label` property.
        pub fn label_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<CommandBarButton, _>("Label", None)
        }

        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<CommandBarButton, _>("Icon", None)
        }

        /// Defines the `IsCompact` property.
        pub fn is_compact_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBarButton, _>("IsCompact", false)
        }

        /// Defines the `DynamicOverflowOrder` property.
        pub fn dynamic_overflow_order_property() -> StyledProperty<i32> {
            FerroProperty::register::<CommandBarButton, _>("DynamicOverflowOrder", 0)
        }

        /// Defines the `LabelPosition` property.
        pub fn label_position_property() -> StyledProperty<CommandBarDefaultLabelPosition> {
            FerroProperty::register::<CommandBarButton, _>("LabelPosition", CommandBarDefaultLabelPosition::Bottom)
        }

        /// Defines the `IsInOverflow` property.
        pub fn is_in_overflow_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBarButton, _>("IsInOverflow", false)
        }
    }
}

impl CommandBarButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Button::construct() }
    }

    /// Creates the button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The button as a command bar element.
    pub fn as_command_bar_element(&self) -> Rc<dyn ICommandBarElement> {
        Rc::new(CommandBarButtonElement(self.to_ref()))
    }

    /// Gets the text label for the button.
    pub fn label(&self) -> Option<String> {
        self.get_value(Self::label_property())
    }

    /// Sets the text label for the button.
    pub fn set_label(&self, value: Option<&str>) {
        self.set_value(Self::label_property(), value.map(str::to_string))
    }

    /// Gets the icon content for the button.
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }

    /// Sets the icon content for the button.
    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }

    /// Gets whether the button is in compact mode (icon only, label
    /// hidden).
    pub fn is_compact(&self) -> bool {
        self.get_value(Self::is_compact_property())
    }

    /// Sets whether the button is in compact mode (icon only, label
    /// hidden).
    pub fn set_is_compact(&self, value: bool) {
        self.set_value(Self::is_compact_property(), value)
    }

    /// Gets the order in which this button moves to the overflow menu when
    /// space is limited. Lower values have higher priority (stay visible
    /// longer).
    pub fn dynamic_overflow_order(&self) -> i32 {
        self.get_value(Self::dynamic_overflow_order_property())
    }

    /// Sets the order in which this button moves to the overflow menu when
    /// space is limited.
    pub fn set_dynamic_overflow_order(&self, value: i32) {
        self.set_value(Self::dynamic_overflow_order_property(), value)
    }

    /// Gets the label position. This is set automatically by the parent
    /// command bar.
    pub fn label_position(&self) -> CommandBarDefaultLabelPosition {
        self.get_value(Self::label_position_property())
    }

    /// Sets the label position. This is set automatically by the parent
    /// command bar.
    pub fn set_label_position(&self, value: CommandBarDefaultLabelPosition) {
        self.set_value(Self::label_position_property(), value)
    }

    /// Gets whether this button is displayed inside the overflow popup.
    pub fn is_in_overflow(&self) -> bool {
        self.get_value(Self::is_in_overflow_property())
    }

    /// Sets whether this button is displayed inside the overflow popup. Set
    /// automatically by the command bar when moving items between primary
    /// and overflow.
    pub fn set_is_in_overflow(&self, value: bool) {
        self.set_value(Self::is_in_overflow_property(), value)
    }
}
