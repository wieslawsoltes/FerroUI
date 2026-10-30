//! Port of `Views/Pages/ScaleAnimationView` of the reference sample: the class of `pages/ScaleAnimationView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScaleAnimationView {
    base: UserControl,
}

user_control_class!(ScaleAnimationView);
ferro_class_info!(ScaleAnimationView { new: ScaleAnimationView::new });
xaml_class!(ScaleAnimationView, "/pages/ScaleAnimationView.xaml");

impl ScaleAnimationView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScaleAnimationView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScaleAnimationView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScaleAnimationView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Scale Animation",
    reference: "Views/Pages/ScaleAnimationView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScaleAnimationView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
