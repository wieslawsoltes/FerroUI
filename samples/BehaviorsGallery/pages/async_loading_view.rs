//! Port of `Views/Pages/AsyncLoadingView` of the reference sample: the class of `pages/AsyncLoadingView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AsyncLoadingView {
    base: UserControl,
}

user_control_class!(AsyncLoadingView);
ferro_class_info!(AsyncLoadingView { new: AsyncLoadingView::new });
xaml_class!(AsyncLoadingView, "/pages/AsyncLoadingView.xaml");

impl AsyncLoadingView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AsyncLoadingView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AsyncLoadingView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AsyncLoadingView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Async Loading",
    reference: "Views/Pages/AsyncLoadingView",
    create: Some(create),
    types: TYPES,
    classes: &[&AsyncLoadingView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
