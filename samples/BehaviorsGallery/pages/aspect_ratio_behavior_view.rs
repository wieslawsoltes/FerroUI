//! Port of `Views/Pages/AspectRatioBehaviorView` of the reference sample: the class of `pages/AspectRatioBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AspectRatioBehaviorView {
    base: UserControl,
}

user_control_class!(AspectRatioBehaviorView);
ferro_class_info!(AspectRatioBehaviorView { new: AspectRatioBehaviorView::new });
xaml_class!(AspectRatioBehaviorView, "/pages/AspectRatioBehaviorView.xaml");

impl AspectRatioBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AspectRatioBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AspectRatioBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AspectRatioBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AspectRatioBehavior",
    reference: "Views/Pages/AspectRatioBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AspectRatioBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
