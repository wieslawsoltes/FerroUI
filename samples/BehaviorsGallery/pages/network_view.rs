//! Port of `Views/Pages/NetworkView` of the reference sample: the class of `pages/NetworkView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::network_view_model::NetworkViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct NetworkView {
    base: UserControl,
}

user_control_class!(NetworkView);
ferro_class_info!(NetworkView { new: NetworkView::new });
xaml_class!(NetworkView, "/pages/NetworkView.xaml");

impl NetworkView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `NetworkView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    NetworkView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[NetworkView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Network",
    reference: "Views/Pages/NetworkView",
    create: Some(create),
    types: TYPES,
    classes: &[&NetworkView::XAML_CLASS],
    markup_types: &[<NetworkViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page (the document
/// creates it as its data context).
fn register_value_types() {
    ValueTypes::register_reference::<NetworkViewModel>();
}
