use crate::primitives::TemplatedControlImpl;
use crate::{ContentControl, ContentControlImpl, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};

/// Provides the base class for defining a new control that encapsulates
/// related existing controls and provides its own logic.
#[repr(C)]
pub struct UserControl {
    base: ContentControl,
}

ferro_class!(UserControl: ContentControl);
ferroui_base::ferro_class_info!(UserControl { new: UserControl::new });
ferro_impl_classes!(
    UserControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl ControlImpl for UserControl {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::UserControlAutomationPeer::new(this).upcast()
    }
}

impl UserControl {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ContentControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
