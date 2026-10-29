//! Port of `Views/Pages/ScrollViewerOffsetBehaviorView` of the reference sample: the class of `pages/ScrollViewerOffsetBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ScrollViewerOffsetBehaviorView {
    base: UserControl,
}

user_control_class!(ScrollViewerOffsetBehaviorView);
ferro_class_info!(ScrollViewerOffsetBehaviorView { new: ScrollViewerOffsetBehaviorView::new });
xaml_class!(ScrollViewerOffsetBehaviorView, "/pages/ScrollViewerOffsetBehaviorView.xaml");

impl ScrollViewerOffsetBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ScrollViewerOffsetBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ScrollViewerOffsetBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScrollViewerOffsetBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ScrollViewerOffsetBehavior",
    reference: "Views/Pages/ScrollViewerOffsetBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScrollViewerOffsetBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
