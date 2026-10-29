//! Port of `Views/Pages/ExitAnimationsView` of the reference sample: the class of `pages/ExitAnimationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ExitAnimationsView {
    base: UserControl,
}

user_control_class!(ExitAnimationsView);
ferro_class_info!(ExitAnimationsView {
    new: ExitAnimationsView::new
});
xaml_class!(ExitAnimationsView, "/pages/ExitAnimationsView.xaml");

impl ExitAnimationsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ExitAnimationsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ExitAnimationsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ExitAnimationsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Exit Animations",
    reference: "Views/Pages/ExitAnimationsView",
    create: Some(create),
    types: TYPES,
    classes: &[&ExitAnimationsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
