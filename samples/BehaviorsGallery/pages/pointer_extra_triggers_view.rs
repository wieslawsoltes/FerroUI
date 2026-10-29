//! Port of `Views/Pages/PointerExtraTriggersView` of the reference sample: the class of `pages/PointerExtraTriggersView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct PointerExtraTriggersView {
    base: UserControl,
}

user_control_class!(PointerExtraTriggersView);
ferro_class_info!(PointerExtraTriggersView { new: PointerExtraTriggersView::new });
xaml_class!(PointerExtraTriggersView, "/pages/PointerExtraTriggersView.xaml");

impl PointerExtraTriggersView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `PointerExtraTriggersView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PointerExtraTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PointerExtraTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Pointer Extra Triggers",
    reference: "Views/Pages/PointerExtraTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&PointerExtraTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
