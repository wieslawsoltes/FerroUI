//! Port of `Views/Pages/ShowOnEventBehaviorsView` of the reference sample: the class of `pages/ShowOnEventBehaviorsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct ShowOnEventBehaviorsView {
    base: UserControl,
}

user_control_class!(ShowOnEventBehaviorsView);
ferro_class_info!(ShowOnEventBehaviorsView {
    new: ShowOnEventBehaviorsView::new
});
xaml_class!(
    ShowOnEventBehaviorsView,
    "/pages/ShowOnEventBehaviorsView.xaml"
);

impl ShowOnEventBehaviorsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `ShowOnEventBehaviorsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ShowOnEventBehaviorsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ShowOnEventBehaviorsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ShowOn Event Behaviors",
    reference: "Views/Pages/ShowOnEventBehaviorsView",
    create: Some(create),
    types: TYPES,
    classes: &[&ShowOnEventBehaviorsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
