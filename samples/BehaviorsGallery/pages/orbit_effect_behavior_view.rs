//! Port of `Views/Pages/OrbitEffectBehaviorView` of the reference sample: the class of `pages/OrbitEffectBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct OrbitEffectBehaviorView {
    base: UserControl,
}

user_control_class!(OrbitEffectBehaviorView);
ferro_class_info!(OrbitEffectBehaviorView { new: OrbitEffectBehaviorView::new });
xaml_class!(OrbitEffectBehaviorView, "/pages/OrbitEffectBehaviorView.xaml");

impl OrbitEffectBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `OrbitEffectBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    OrbitEffectBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[OrbitEffectBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "OrbitEffectBehavior",
    reference: "Views/Pages/OrbitEffectBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&OrbitEffectBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
