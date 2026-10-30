//! Port of `Views/Pages/PickerCanExecuteCommandView` of the reference sample: the class of `pages/PickerCanExecuteCommandView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct PickerCanExecuteCommandView {
    base: UserControl,
}

user_control_class!(PickerCanExecuteCommandView);
ferro_class_info!(PickerCanExecuteCommandView {
    new: PickerCanExecuteCommandView::new
});
xaml_class!(
    PickerCanExecuteCommandView,
    "/pages/PickerCanExecuteCommandView.xaml"
);

impl PickerCanExecuteCommandView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `PickerCanExecuteCommandView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PickerCanExecuteCommandView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PickerCanExecuteCommandView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Picker CanExecuteCommand",
    reference: "Views/Pages/PickerCanExecuteCommandView",
    create: Some(create),
    types: TYPES,
    classes: &[&PickerCanExecuteCommandView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
