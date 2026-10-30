//! Port of `Views/Pages/RotateAnimationView` of the reference sample: the class of `pages/RotateAnimationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RotateAnimationView {
    base: UserControl,
}

user_control_class!(RotateAnimationView);
ferro_class_info!(RotateAnimationView {
    new: RotateAnimationView::new
});
xaml_class!(RotateAnimationView, "/pages/RotateAnimationView.xaml");

impl RotateAnimationView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `RotateAnimationView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RotateAnimationView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RotateAnimationView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Rotate Animation",
    reference: "Views/Pages/RotateAnimationView",
    create: Some(create),
    types: TYPES,
    classes: &[&RotateAnimationView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
