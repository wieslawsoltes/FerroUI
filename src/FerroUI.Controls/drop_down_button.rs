use crate::primitives::TemplatedControlImpl;
use crate::{Button, ButtonImpl, ContentControlImpl, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};

/// A button with an added drop-down chevron to visually indicate it has a
/// flyout with additional actions.
#[repr(C)]
pub struct DropDownButton {
    base: Button,
}

ferro_class!(DropDownButton: Button);
ferroui_base::ferro_class_info!(DropDownButton { new: DropDownButton::new });
ferro_impl_classes!(
    DropDownButton: FerroObjectImpl,
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

impl DropDownButton {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Button::construct() }
    }

    /// Creates a drop-down button.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
