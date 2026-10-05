use crate::primitives::{HeaderedContentControl, TemplatedControlImpl};
use crate::{ContentControlImpl, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};

/// A headered content control that groups related content.
#[repr(C)]
pub struct GroupBox {
    base: HeaderedContentControl,
}

ferro_class!(GroupBox: HeaderedContentControl);
ferroui_base::ferro_class_info!(GroupBox { new: GroupBox::new });
ferro_impl_classes!(
    GroupBox: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl GroupBox {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: HeaderedContentControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
