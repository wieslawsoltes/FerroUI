use crate::primitives::{TemplatedControlImpl, ToggleButton, ToggleButtonImpl};
use crate::{ButtonImpl, ContentControlImpl, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};

/// A check box control.
#[repr(C)]
pub struct CheckBox {
    base: ToggleButton,
}

ferro_class!(CheckBox: ToggleButton);
ferroui_base::ferro_class_info!(CheckBox { new: CheckBox::new });
ferro_impl_classes!(
    CheckBox: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    ButtonImpl,
    ToggleButtonImpl
);

impl CheckBox {
    fn static_constructor() {
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<CheckBox>(Some(crate::automation::peers::AutomationControlType::CheckBox));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ToggleButton::construct() }
    }

    /// Creates a check box.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
