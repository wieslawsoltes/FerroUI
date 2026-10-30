//! Port of `Views/Pages/StartBuiltAnimationActionView` of the reference sample: the class of `pages/StartBuiltAnimationActionView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::custom_animator_view_model::CustomAnimatorViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct StartBuiltAnimationActionView {
    base: UserControl,
}

user_control_class!(StartBuiltAnimationActionView);
ferro_class_info!(StartBuiltAnimationActionView {
    new: StartBuiltAnimationActionView::new
});
xaml_class!(
    StartBuiltAnimationActionView,
    "/pages/StartBuiltAnimationActionView.xaml"
);

impl StartBuiltAnimationActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `StartBuiltAnimationActionView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(CustomAnimatorViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    StartBuiltAnimationActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[StartBuiltAnimationActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "StartBuiltAnimationAction",
    reference: "Views/Pages/StartBuiltAnimationActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&StartBuiltAnimationActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
