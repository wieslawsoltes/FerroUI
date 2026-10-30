//! Port of `Views/Pages/TemplateBindingView` of the reference sample: the class of `pages/TemplateBindingView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TemplateBindingView {
    base: UserControl,
}

user_control_class!(TemplateBindingView);
ferro_class_info!(TemplateBindingView {
    new: TemplateBindingView::new
});
xaml_class!(TemplateBindingView, "/pages/TemplateBindingView.xaml");

impl TemplateBindingView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `TemplateBindingView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TemplateBindingView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TemplateBindingView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TemplateBinding",
    reference: "Views/Pages/TemplateBindingView",
    create: Some(create),
    types: TYPES,
    classes: &[&TemplateBindingView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
