//! Port of `Views/Pages/TiltEffectBehaviorView` of the reference sample: the class of `pages/TiltEffectBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TiltEffectBehaviorView {
    base: UserControl,
}

user_control_class!(TiltEffectBehaviorView);
ferro_class_info!(TiltEffectBehaviorView { new: TiltEffectBehaviorView::new });
xaml_class!(TiltEffectBehaviorView, "/pages/TiltEffectBehaviorView.xaml");

impl TiltEffectBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TiltEffectBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TiltEffectBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TiltEffectBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TiltEffectBehavior",
    reference: "Views/Pages/TiltEffectBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&TiltEffectBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
