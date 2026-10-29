//! Port of `Views/Pages/StartAnimationActionView` of the reference sample: the class of `pages/StartAnimationActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct StartAnimationActionView {
    base: UserControl,
}

user_control_class!(StartAnimationActionView);
ferro_class_info!(StartAnimationActionView {
    new: StartAnimationActionView::new
});
xaml_class!(
    StartAnimationActionView,
    "/pages/StartAnimationActionView.xaml"
);

impl StartAnimationActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `StartAnimationActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    StartAnimationActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[StartAnimationActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "StartAnimationAction",
    reference: "Views/Pages/StartAnimationActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&StartAnimationActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
