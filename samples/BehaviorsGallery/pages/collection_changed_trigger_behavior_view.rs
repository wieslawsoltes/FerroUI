//! Port of `Views/Pages/CollectionChangedTriggerBehaviorView` of the reference sample: the class of `pages/CollectionChangedTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct CollectionChangedTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(CollectionChangedTriggerBehaviorView);
ferro_class_info!(CollectionChangedTriggerBehaviorView { new: CollectionChangedTriggerBehaviorView::new });
xaml_class!(CollectionChangedTriggerBehaviorView, "/pages/CollectionChangedTriggerBehaviorView.xaml");

impl CollectionChangedTriggerBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `CollectionChangedTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    CollectionChangedTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CollectionChangedTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "CollectionChangedTrigger",
    reference: "Views/Pages/CollectionChangedTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&CollectionChangedTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
