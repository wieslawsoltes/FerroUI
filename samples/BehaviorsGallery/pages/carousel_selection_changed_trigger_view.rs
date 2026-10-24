//! Port of `Views/Pages/CarouselSelectionChangedTriggerView` of the reference sample: the class of `pages/CarouselSelectionChangedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct CarouselSelectionChangedTriggerView {
    base: UserControl,
}

user_control_class!(CarouselSelectionChangedTriggerView);
ferro_class_info!(CarouselSelectionChangedTriggerView { new: CarouselSelectionChangedTriggerView::new });
xaml_class!(CarouselSelectionChangedTriggerView, "/pages/CarouselSelectionChangedTriggerView.xaml");

impl CarouselSelectionChangedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `CarouselSelectionChangedTriggerView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    CarouselSelectionChangedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[CarouselSelectionChangedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "CarouselSelectionChangedTrigger",
    reference: "Views/Pages/CarouselSelectionChangedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&CarouselSelectionChangedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
