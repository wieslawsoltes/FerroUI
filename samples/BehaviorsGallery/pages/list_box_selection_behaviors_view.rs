//! Port of `Views/Pages/ListBoxSelectionBehaviorsView` of the reference sample: the class of `pages/ListBoxSelectionBehaviorsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ListBoxSelectionBehaviorsView {
    base: UserControl,
}

user_control_class!(ListBoxSelectionBehaviorsView);
ferro_class_info!(ListBoxSelectionBehaviorsView {
    new: ListBoxSelectionBehaviorsView::new
});
xaml_class!(
    ListBoxSelectionBehaviorsView,
    "/pages/ListBoxSelectionBehaviorsView.xaml"
);

impl ListBoxSelectionBehaviorsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ListBoxSelectionBehaviorsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ListBoxSelectionBehaviorsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ListBoxSelectionBehaviorsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ListBox Selection Behaviors",
    reference: "Views/Pages/ListBoxSelectionBehaviorsView",
    create: Some(create),
    types: TYPES,
    classes: &[&ListBoxSelectionBehaviorsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
