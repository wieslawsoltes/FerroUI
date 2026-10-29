//! Port of `Views/Pages/DataContextChangedTriggerView` of the reference sample: the class of `pages/DataContextChangedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DataContextChangedTriggerView {
    base: UserControl,
}

user_control_class!(DataContextChangedTriggerView);
ferro_class_info!(DataContextChangedTriggerView { new: DataContextChangedTriggerView::new });
xaml_class!(DataContextChangedTriggerView, "/pages/DataContextChangedTriggerView.xaml");

impl DataContextChangedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DataContextChangedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DataContextChangedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DataContextChangedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DataContextChangedTrigger",
    reference: "Views/Pages/DataContextChangedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&DataContextChangedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
