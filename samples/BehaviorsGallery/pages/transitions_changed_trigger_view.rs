//! Port of `Views/Pages/TransitionsChangedTriggerView` of the reference sample: the class of `pages/TransitionsChangedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TransitionsChangedTriggerView {
    base: UserControl,
}

user_control_class!(TransitionsChangedTriggerView);
ferro_class_info!(TransitionsChangedTriggerView { new: TransitionsChangedTriggerView::new });
xaml_class!(TransitionsChangedTriggerView, "/pages/TransitionsChangedTriggerView.xaml");

impl TransitionsChangedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TransitionsChangedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TransitionsChangedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TransitionsChangedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Transitions Trigger",
    reference: "Views/Pages/TransitionsChangedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&TransitionsChangedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
