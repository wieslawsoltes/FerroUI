//! Port of `Views/Pages/AttentionAnimationsView` of the reference sample: the class of `pages/AttentionAnimationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AttentionAnimationsView {
    base: UserControl,
}

user_control_class!(AttentionAnimationsView);
ferro_class_info!(AttentionAnimationsView {
    new: AttentionAnimationsView::new
});
xaml_class!(
    AttentionAnimationsView,
    "/pages/AttentionAnimationsView.xaml"
);

impl AttentionAnimationsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AttentionAnimationsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AttentionAnimationsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AttentionAnimationsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Attention Animations",
    reference: "Views/Pages/AttentionAnimationsView",
    create: Some(create),
    types: TYPES,
    classes: &[&AttentionAnimationsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
