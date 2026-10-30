//! Port of `Views/Pages/ItemsControlContainerTriggersView` of the reference sample: the class of `pages/ItemsControlContainerTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ItemsControlContainerTriggersView {
    base: UserControl,
}

user_control_class!(ItemsControlContainerTriggersView);
ferro_class_info!(ItemsControlContainerTriggersView { new: ItemsControlContainerTriggersView::new });
xaml_class!(ItemsControlContainerTriggersView, "/pages/ItemsControlContainerTriggersView.xaml");

impl ItemsControlContainerTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ItemsControlContainerTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ItemsControlContainerTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ItemsControlContainerTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ItemsControl Container Triggers",
    reference: "Views/Pages/ItemsControlContainerTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&ItemsControlContainerTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
