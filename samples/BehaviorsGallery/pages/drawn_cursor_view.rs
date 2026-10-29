//! Port of `Views/Pages/DrawnCursorView` of the reference sample: the class of `pages/DrawnCursorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::drawn_cursor_view_model::DrawnCursorViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DrawnCursorView {
    base: UserControl,
}

user_control_class!(DrawnCursorView);
ferro_class_info!(DrawnCursorView {
    new: DrawnCursorView::new
});
xaml_class!(DrawnCursorView, "/pages/DrawnCursorView.xaml");

impl DrawnCursorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `DrawnCursorView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(DrawnCursorViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DrawnCursorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DrawnCursorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Drawn Cursor",
    reference: "Views/Pages/DrawnCursorView",
    create: Some(create),
    types: TYPES,
    classes: &[&DrawnCursorView::XAML_CLASS],
    markup_types: &[<DrawnCursorViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<DrawnCursorViewModel>();
}
