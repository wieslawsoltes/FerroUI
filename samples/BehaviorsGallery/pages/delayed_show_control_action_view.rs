//! Port of `Views/Pages/DelayedShowControlActionView` of the reference sample: the class of `pages/DelayedShowControlActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DelayedShowControlActionView {
    base: UserControl,
}

user_control_class!(DelayedShowControlActionView);
ferro_class_info!(DelayedShowControlActionView { new: DelayedShowControlActionView::new });
xaml_class!(DelayedShowControlActionView, "/pages/DelayedShowControlActionView.xaml");

impl DelayedShowControlActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DelayedShowControlActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DelayedShowControlActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DelayedShowControlActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DelayedShowControlAction",
    reference: "Views/Pages/DelayedShowControlActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&DelayedShowControlActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
