//! Port of `Views/Pages/BoundsObserverBehaviorView` of the reference sample: the class of `pages/BoundsObserverBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct BoundsObserverBehaviorView {
    base: UserControl,
}

user_control_class!(BoundsObserverBehaviorView);
ferro_class_info!(BoundsObserverBehaviorView { new: BoundsObserverBehaviorView::new });
xaml_class!(BoundsObserverBehaviorView, "/pages/BoundsObserverBehaviorView.xaml");

impl BoundsObserverBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `BoundsObserverBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    BoundsObserverBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[BoundsObserverBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "BoundsObserverBehavior",
    reference: "Views/Pages/BoundsObserverBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&BoundsObserverBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
