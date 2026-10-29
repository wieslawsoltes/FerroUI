//! Port of `Views/Pages/DisposableActionView` of the reference sample: the class of `pages/DisposableActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::disposable_action_behavior::DisposableActionBehavior;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DisposableActionView {
    base: UserControl,
}

user_control_class!(DisposableActionView);
ferro_class_info!(DisposableActionView { new: DisposableActionView::new });
xaml_class!(DisposableActionView, "/pages/DisposableActionView.xaml");

impl DisposableActionView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DisposableActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DisposableActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[DisposableActionView::TYPE, DisposableActionBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DisposableAction",
    reference: "Views/Pages/DisposableActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&DisposableActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
