//! Port of `Views/Pages/AsyncActionGroupView` of the reference sample: the class of `pages/AsyncActionGroupView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AsyncActionGroupView {
    base: UserControl,
}

user_control_class!(AsyncActionGroupView);
ferro_class_info!(AsyncActionGroupView { new: AsyncActionGroupView::new });
xaml_class!(AsyncActionGroupView, "/pages/AsyncActionGroupView.xaml");

impl AsyncActionGroupView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AsyncActionGroupView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AsyncActionGroupView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AsyncActionGroupView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AsyncActionGroup",
    reference: "Views/Pages/AsyncActionGroupView",
    create: Some(create),
    types: TYPES,
    classes: &[&AsyncActionGroupView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
