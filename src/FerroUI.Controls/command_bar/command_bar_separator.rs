use super::ICommandBarElement;
use crate::primitives::TemplatedControlImpl;
use crate::{ControlImpl, Separator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::rc::Rc;

/// A visual separator for use in a [`CommandBar`](super::CommandBar).
#[repr(C)]
pub struct CommandBarSeparator {
    base: Separator,
}

ferro_class!(CommandBarSeparator: Separator);
ferro_class_info!(CommandBarSeparator {
    new: CommandBarSeparator::new,
    interfaces: [
        Rc<dyn ICommandBarElement> => |separator: Ref<CommandBarSeparator>| separator.as_command_bar_element(),
    ],
});
ferro_impl_classes!(
    CommandBarSeparator: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

/// Implements the command bar element contract for the separator behind a
/// class handle.
struct CommandBarSeparatorElement(Ref<CommandBarSeparator>);

impl ICommandBarElement for CommandBarSeparatorElement {
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
    impl CommandBarSeparator {
        /// Defines the `IsCompact` property.
        pub fn is_compact_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBarSeparator, _>("IsCompact", false)
        }

        /// Defines the `IsInOverflow` property.
        pub fn is_in_overflow_property() -> StyledProperty<bool> {
            FerroProperty::register::<CommandBarSeparator, _>("IsInOverflow", false)
        }
    }
}

impl CommandBarSeparator {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Separator::construct() }
    }

    /// Creates a command bar separator.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The separator as a command bar element.
    pub fn as_command_bar_element(&self) -> Rc<dyn ICommandBarElement> {
        Rc::new(CommandBarSeparatorElement(self.to_ref()))
    }

    /// Gets whether the separator is in compact mode.
    pub fn is_compact(&self) -> bool {
        self.get_value(Self::is_compact_property())
    }

    /// Sets whether the separator is in compact mode.
    pub fn set_is_compact(&self, value: bool) {
        self.set_value(Self::is_compact_property(), value)
    }

    /// Gets whether the separator is displayed inside the overflow popup.
    pub fn is_in_overflow(&self) -> bool {
        self.get_value(Self::is_in_overflow_property())
    }

    /// Sets whether the separator is displayed inside the overflow popup.
    /// Set automatically by the command bar when moving items between
    /// primary and overflow.
    pub fn set_is_in_overflow(&self, value: bool) {
        self.set_value(Self::is_in_overflow_property(), value)
    }
}
