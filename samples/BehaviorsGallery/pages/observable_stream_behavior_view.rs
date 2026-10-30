//! Port of `Views/Pages/ObservableStreamBehaviorView` of the reference sample: the class of `pages/ObservableStreamBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ObservableStreamBehaviorView {
    base: UserControl,
}

user_control_class!(ObservableStreamBehaviorView);
ferro_class_info!(ObservableStreamBehaviorView { new: ObservableStreamBehaviorView::new });
xaml_class!(ObservableStreamBehaviorView, "/pages/ObservableStreamBehaviorView.xaml");

impl ObservableStreamBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ObservableStreamBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ObservableStreamBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ObservableStreamBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ObservableStreamBehavior",
    reference: "Views/Pages/ObservableStreamBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ObservableStreamBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
