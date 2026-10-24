//! Port of `Views/Pages/ObjectTemplateView` of the reference sample: the class of `pages/ObjectTemplateView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ObjectTemplateView {
    base: UserControl,
}

user_control_class!(ObjectTemplateView);
ferro_class_info!(ObjectTemplateView { new: ObjectTemplateView::new });
xaml_class!(ObjectTemplateView, "/pages/ObjectTemplateView.xaml");

impl ObjectTemplateView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ObjectTemplateView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ObjectTemplateView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ObjectTemplateView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ObjectTemplate",
    reference: "Views/Pages/ObjectTemplateView",
    create: Some(create),
    types: TYPES,
    classes: &[&ObjectTemplateView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
