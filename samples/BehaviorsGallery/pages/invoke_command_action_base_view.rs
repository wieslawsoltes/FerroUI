//! Port of `Views/Pages/InvokeCommandActionBaseView` of the reference sample: the class of `pages/InvokeCommandActionBaseView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InvokeCommandActionBaseView {
    base: UserControl,
}

user_control_class!(InvokeCommandActionBaseView);
ferro_class_info!(InvokeCommandActionBaseView { new: InvokeCommandActionBaseView::new });
xaml_class!(InvokeCommandActionBaseView, "/pages/InvokeCommandActionBaseView.xaml");

impl InvokeCommandActionBaseView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `InvokeCommandActionBaseView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InvokeCommandActionBaseView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[InvokeCommandActionBaseView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InvokeCommandActionBase",
    reference: "Views/Pages/InvokeCommandActionBaseView",
    create: Some(create),
    types: TYPES,
    classes: &[&InvokeCommandActionBaseView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
