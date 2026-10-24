//! Port of `Views/Pages/IconView` of the reference sample: the class of `pages/IconView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::icon_view_model::IconViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct IconView {
    base: UserControl,
}

user_control_class!(IconView);
ferro_class_info!(IconView { new: IconView::new });
xaml_class!(IconView, "/pages/IconView.xaml");

impl IconView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `IconView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(IconViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    IconView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[IconView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Icon Sample",
    reference: "Views/Pages/IconView",
    create: Some(create),
    types: TYPES,
    classes: &[&IconView::XAML_CLASS],
    markup_types: &[<IconViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<IconViewModel>();
}
