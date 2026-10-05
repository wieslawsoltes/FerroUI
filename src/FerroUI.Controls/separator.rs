use crate::primitives::{TemplatedControl, TemplatedControlImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};

/// A separator control.
#[repr(C)]
pub struct Separator {
    base: TemplatedControl,
}

ferro_class!(Separator: TemplatedControl);
ferroui_base::ferro_class_info!(Separator { new: Separator::new });
ferro_impl_classes!(
    Separator: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl Separator {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: TemplatedControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
