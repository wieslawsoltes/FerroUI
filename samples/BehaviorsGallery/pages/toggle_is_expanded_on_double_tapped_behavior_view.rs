//! Port of `Views/Pages/ToggleIsExpandedOnDoubleTappedBehaviorView` of the reference sample: the class of `pages/ToggleIsExpandedOnDoubleTappedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ToggleIsExpandedOnDoubleTappedBehaviorView {
    base: UserControl,
}

user_control_class!(ToggleIsExpandedOnDoubleTappedBehaviorView);
ferro_class_info!(ToggleIsExpandedOnDoubleTappedBehaviorView { new: ToggleIsExpandedOnDoubleTappedBehaviorView::new });
xaml_class!(ToggleIsExpandedOnDoubleTappedBehaviorView, "/pages/ToggleIsExpandedOnDoubleTappedBehaviorView.xaml");

impl ToggleIsExpandedOnDoubleTappedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ToggleIsExpandedOnDoubleTappedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ToggleIsExpandedOnDoubleTappedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ToggleIsExpandedOnDoubleTappedBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ToggleIsExpandedOnDoubleTappedBehavior",
    reference: "Views/Pages/ToggleIsExpandedOnDoubleTappedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ToggleIsExpandedOnDoubleTappedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
