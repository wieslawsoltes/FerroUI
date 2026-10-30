//! Port of `Views/Pages/TransitionsBehaviorView` of the reference sample: the class of `pages/TransitionsBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct TransitionsBehaviorView {
    base: UserControl,
}

user_control_class!(TransitionsBehaviorView);
ferro_class_info!(TransitionsBehaviorView { new: TransitionsBehaviorView::new });
xaml_class!(TransitionsBehaviorView, "/pages/TransitionsBehaviorView.xaml");

impl TransitionsBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `TransitionsBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    TransitionsBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TransitionsBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Transitions Behavior",
    reference: "Views/Pages/TransitionsBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&TransitionsBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
