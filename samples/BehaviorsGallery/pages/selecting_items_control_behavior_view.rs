//! Port of `Views/Pages/SelectingItemsControlBehaviorView` of the reference sample: the class of `pages/SelectingItemsControlBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SelectingItemsControlBehaviorView {
    base: UserControl,
}

user_control_class!(SelectingItemsControlBehaviorView);
ferro_class_info!(SelectingItemsControlBehaviorView {
    new: SelectingItemsControlBehaviorView::new
});
xaml_class!(
    SelectingItemsControlBehaviorView,
    "/pages/SelectingItemsControlBehaviorView.xaml"
);

impl SelectingItemsControlBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `SelectingItemsControlBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SelectingItemsControlBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SelectingItemsControlBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "SelectingItemsControlBehavior",
    reference: "Views/Pages/SelectingItemsControlBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&SelectingItemsControlBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
