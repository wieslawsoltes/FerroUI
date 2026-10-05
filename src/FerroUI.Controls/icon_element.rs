use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, FerroObjectImpl, StyledElementImpl, VisualImpl};

/// Represents the base class for an icon UI element.
#[repr(C)]
pub struct IconElement {
    base: TemplatedControl,
}

ferro_class!(IconElement: TemplatedControl);
ferro_impl_classes!(
    IconElement: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl IconElement {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct() }
    }
}
