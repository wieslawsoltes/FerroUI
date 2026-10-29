//! Port of `Views/Pages/RemoveItemsSampleView` of the reference sample: the class of `pages/RemoveItemsSampleView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RemoveItemsSampleView {
    base: UserControl,
}

user_control_class!(RemoveItemsSampleView);
ferro_class_info!(RemoveItemsSampleView { new: RemoveItemsSampleView::new });
xaml_class!(RemoveItemsSampleView, "/pages/RemoveItemsSampleView.xaml");

impl RemoveItemsSampleView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RemoveItemsSampleView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RemoveItemsSampleView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RemoveItemsSampleView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Remove Items Sample",
    reference: "Views/Pages/RemoveItemsSampleView",
    create: Some(create),
    types: TYPES,
    classes: &[&RemoveItemsSampleView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
