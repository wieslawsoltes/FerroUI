//! Port of `Views/Pages/FadeAnimationView` of the reference sample: the class of `pages/FadeAnimationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FadeAnimationView {
    base: UserControl,
}

user_control_class!(FadeAnimationView);
ferro_class_info!(FadeAnimationView {
    new: FadeAnimationView::new
});
xaml_class!(FadeAnimationView, "/pages/FadeAnimationView.xaml");

impl FadeAnimationView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FadeAnimationView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FadeAnimationView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FadeAnimationView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Fade Animation",
    reference: "Views/Pages/FadeAnimationView",
    create: Some(create),
    types: TYPES,
    classes: &[&FadeAnimationView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
