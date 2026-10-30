//! Port of `Views/Pages/ParallaxBehaviorView` of the reference sample: the class of `pages/ParallaxBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ParallaxBehaviorView {
    base: UserControl,
}

user_control_class!(ParallaxBehaviorView);
ferro_class_info!(ParallaxBehaviorView { new: ParallaxBehaviorView::new });
xaml_class!(ParallaxBehaviorView, "/pages/ParallaxBehaviorView.xaml");

impl ParallaxBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ParallaxBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ParallaxBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ParallaxBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ParallaxBehavior",
    reference: "Views/Pages/ParallaxBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ParallaxBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
