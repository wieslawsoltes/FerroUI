//! Port of `Views/Pages/SlidingAnimationView` of the reference sample: the class of `pages/SlidingAnimationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct SlidingAnimationView {
    base: UserControl,
}

user_control_class!(SlidingAnimationView);
ferro_class_info!(SlidingAnimationView { new: SlidingAnimationView::new });
xaml_class!(SlidingAnimationView, "/pages/SlidingAnimationView.xaml");

impl SlidingAnimationView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `SlidingAnimationView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    SlidingAnimationView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[SlidingAnimationView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Sliding Animation",
    reference: "Views/Pages/SlidingAnimationView",
    create: Some(create),
    types: TYPES,
    classes: &[&SlidingAnimationView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
