//! Port of `Views/Pages/ChangeFerroPropertyActionView` of the reference sample: the class of `pages/ChangeFerroPropertyActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ChangeFerroPropertyActionView {
    base: UserControl,
}

user_control_class!(ChangeFerroPropertyActionView);
ferro_class_info!(ChangeFerroPropertyActionView { new: ChangeFerroPropertyActionView::new });
xaml_class!(ChangeFerroPropertyActionView, "/pages/ChangeFerroPropertyActionView.xaml");

impl ChangeFerroPropertyActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ChangeFerroPropertyActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ChangeFerroPropertyActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ChangeFerroPropertyActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ChangeFerroPropertyAction",
    reference: "Views/Pages/ChangeFerroPropertyActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&ChangeFerroPropertyActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
