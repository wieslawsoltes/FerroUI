//! Port of `Views/Pages/ActionCollectionTemplateView` of the reference sample: the class of `pages/ActionCollectionTemplateView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ActionCollectionTemplateView {
    base: UserControl,
}

user_control_class!(ActionCollectionTemplateView);
ferro_class_info!(ActionCollectionTemplateView {
    new: ActionCollectionTemplateView::new
});
xaml_class!(
    ActionCollectionTemplateView,
    "/pages/ActionCollectionTemplateView.xaml"
);

impl ActionCollectionTemplateView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ActionCollectionTemplateView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ActionCollectionTemplateView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ActionCollectionTemplateView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ActionCollectionTemplate",
    reference: "Views/Pages/ActionCollectionTemplateView",
    create: Some(create),
    types: TYPES,
    classes: &[&ActionCollectionTemplateView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
