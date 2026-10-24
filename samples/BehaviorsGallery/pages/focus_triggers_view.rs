//! Port of `Views/Pages/FocusTriggersView` of the reference sample: the class of `pages/FocusTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FocusTriggersView {
    base: UserControl,
}

user_control_class!(FocusTriggersView);
ferro_class_info!(FocusTriggersView { new: FocusTriggersView::new });
xaml_class!(FocusTriggersView, "/pages/FocusTriggersView.xaml");

impl FocusTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `FocusTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FocusTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FocusTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Focus Triggers",
    reference: "Views/Pages/FocusTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&FocusTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
