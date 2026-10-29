//! Port of `Views/Pages/CursorView` of the reference sample: the class of `pages/CursorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::cursor_view_model::{CursorViewModel, StandardCursorModel};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct CursorView {
    base: UserControl,
}

user_control_class!(CursorView);
ferro_class_info!(CursorView {
    new: CursorView::new
});
xaml_class!(CursorView, "/pages/CursorView.xaml");

impl CursorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `CursorView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(CursorViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    CursorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CursorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Cursor Behavior",
    reference: "Views/Pages/CursorView",
    create: Some(create),
    types: TYPES,
    classes: &[&CursorView::XAML_CLASS],
    markup_types: &[
        <CursorViewModel as MarkupTyped>::MARKUP,
        <StandardCursorModel as MarkupTyped>::MARKUP,
    ],
    register_value_types,
};

/// What the untyped value conversions must know about the view models of the page.
fn register_value_types() {
    ValueTypes::register_reference::<CursorViewModel>();
    ValueTypes::register_reference::<StandardCursorModel>();
}
