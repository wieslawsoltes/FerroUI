//! Port of `Controls/SampleGroup.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::{HeaderedItemsControl, HeaderedItemsControlImpl, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, ItemsControlImpl};

/// A titled run of `SampleSection`s on a `SamplePage`. Its `Header` names
/// the group ("Overview", "Appearance", ...), so every page presents the
/// same groups in the same order.
#[repr(C)]
pub struct SampleGroup {
    base: HeaderedItemsControl,
}

ferro_class!(SampleGroup: HeaderedItemsControl);
ferro_class_info!(SampleGroup { new: SampleGroup::new });
ferro_impl_classes!(
    SampleGroup: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    HeaderedItemsControlImpl
);

impl SampleGroup {
    pub fn construct() -> Self {
        Self { base: HeaderedItemsControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
