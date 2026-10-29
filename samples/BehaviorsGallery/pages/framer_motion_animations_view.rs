//! Port of `Views/Pages/FramerMotionAnimationsView` of the reference sample: the class of `pages/FramerMotionAnimationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct FramerMotionAnimationsView {
    base: UserControl,
}

user_control_class!(FramerMotionAnimationsView);
ferro_class_info!(FramerMotionAnimationsView {
    new: FramerMotionAnimationsView::new
});
xaml_class!(
    FramerMotionAnimationsView,
    "/pages/FramerMotionAnimationsView.xaml"
);

impl FramerMotionAnimationsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `FramerMotionAnimationsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    FramerMotionAnimationsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[FramerMotionAnimationsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Framer Motion Animations",
    reference: "Views/Pages/FramerMotionAnimationsView",
    create: Some(create),
    types: TYPES,
    classes: &[&FramerMotionAnimationsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
