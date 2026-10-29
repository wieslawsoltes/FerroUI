//! Port of `Views/Pages/BehaviorCollectionTemplateView` of the reference sample: the class of `pages/BehaviorCollectionTemplateView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BehaviorCollectionTemplateView {
    base: UserControl,
}

user_control_class!(BehaviorCollectionTemplateView);
ferro_class_info!(BehaviorCollectionTemplateView {
    new: BehaviorCollectionTemplateView::new
});
xaml_class!(
    BehaviorCollectionTemplateView,
    "/pages/BehaviorCollectionTemplateView.xaml"
);

impl BehaviorCollectionTemplateView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `BehaviorCollectionTemplateView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BehaviorCollectionTemplateView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BehaviorCollectionTemplateView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BehaviorCollectionTemplate",
    reference: "Views/Pages/BehaviorCollectionTemplateView",
    create: Some(create),
    types: TYPES,
    classes: &[&BehaviorCollectionTemplateView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
