//! Port of `Views/Pages/LoseFocusOnEnterBehaviorView` of the reference sample: the class of `pages/LoseFocusOnEnterBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct LoseFocusOnEnterBehaviorView {
    base: UserControl,
}

user_control_class!(LoseFocusOnEnterBehaviorView);
ferro_class_info!(LoseFocusOnEnterBehaviorView { new: LoseFocusOnEnterBehaviorView::new });
xaml_class!(LoseFocusOnEnterBehaviorView, "/pages/LoseFocusOnEnterBehaviorView.xaml");

impl LoseFocusOnEnterBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `LoseFocusOnEnterBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    LoseFocusOnEnterBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[LoseFocusOnEnterBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "LoseFocusOnEnterBehavior",
    reference: "Views/Pages/LoseFocusOnEnterBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&LoseFocusOnEnterBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
