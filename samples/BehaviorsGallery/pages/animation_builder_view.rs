//! Port of `Views/Pages/AnimationBuilderView` of the reference sample: the class of `pages/AnimationBuilderView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::custom_animator_view_model::CustomAnimatorViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AnimationBuilderView {
    base: UserControl,
}

user_control_class!(AnimationBuilderView);
ferro_class_info!(AnimationBuilderView {
    new: AnimationBuilderView::new
});
xaml_class!(AnimationBuilderView, "/pages/AnimationBuilderView.xaml");

impl AnimationBuilderView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `AnimationBuilderView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(CustomAnimatorViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AnimationBuilderView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AnimationBuilderView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AnimationBuilder",
    reference: "Views/Pages/AnimationBuilderView",
    create: Some(create),
    types: TYPES,
    classes: &[&AnimationBuilderView::XAML_CLASS],
    markup_types: &[<CustomAnimatorViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<CustomAnimatorViewModel>();
}
