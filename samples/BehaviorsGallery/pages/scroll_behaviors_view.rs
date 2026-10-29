//! Port of `Views/Pages/ScrollBehaviorsView` of the reference sample: the class of `pages/ScrollBehaviorsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScrollBehaviorsView {
    base: UserControl,
}

user_control_class!(ScrollBehaviorsView);
ferro_class_info!(ScrollBehaviorsView { new: ScrollBehaviorsView::new });
xaml_class!(ScrollBehaviorsView, "/pages/ScrollBehaviorsView.xaml");

impl ScrollBehaviorsView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScrollBehaviorsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScrollBehaviorsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScrollBehaviorsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Scroll Behaviors",
    reference: "Views/Pages/ScrollBehaviorsView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScrollBehaviorsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
