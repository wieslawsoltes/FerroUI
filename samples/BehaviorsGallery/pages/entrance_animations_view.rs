//! Port of `Views/Pages/EntranceAnimationsView` of the reference sample: the class of `pages/EntranceAnimationsView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct EntranceAnimationsView {
    base: UserControl,
}

user_control_class!(EntranceAnimationsView);
ferro_class_info!(EntranceAnimationsView {
    new: EntranceAnimationsView::new
});
xaml_class!(EntranceAnimationsView, "/pages/EntranceAnimationsView.xaml");

impl EntranceAnimationsView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `EntranceAnimationsView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EntranceAnimationsView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EntranceAnimationsView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Entrance Animations",
    reference: "Views/Pages/EntranceAnimationsView",
    create: Some(create),
    types: TYPES,
    classes: &[&EntranceAnimationsView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
