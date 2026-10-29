//! Port of `Views/Pages/PointerTriggersView` of the reference sample: the class of `pages/PointerTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::pointer_triggers_view_model::PointerTriggersViewModel;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct PointerTriggersView {
    base: UserControl,
}

user_control_class!(PointerTriggersView);
ferro_class_info!(PointerTriggersView {
    new: PointerTriggersView::new
});
xaml_class!(PointerTriggersView, "/pages/PointerTriggersView.xaml");

impl PointerTriggersView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `PointerTriggersView()`: the page has its own view model.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(PointerTriggersViewModel::new() as BoxedValue));
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PointerTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PointerTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Pointer Triggers",
    reference: "Views/Pages/PointerTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&PointerTriggersView::XAML_CLASS],
    markup_types: &[<PointerTriggersViewModel as MarkupTyped>::MARKUP],
    register_value_types,
};

/// What the untyped value conversions must know about the view model of the page.
fn register_value_types() {
    ValueTypes::register_reference::<PointerTriggersViewModel>();
}
