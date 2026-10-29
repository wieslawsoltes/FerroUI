//! Port of `Views/Pages/LoadedBehaviorView` of the reference sample: the class of `pages/LoadedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::loaded_message_behavior::LoadedMessageBehavior;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct LoadedBehaviorView {
    base: UserControl,
}

user_control_class!(LoadedBehaviorView);
ferro_class_info!(LoadedBehaviorView { new: LoadedBehaviorView::new });
xaml_class!(LoadedBehaviorView, "/pages/LoadedBehaviorView.xaml");

impl LoadedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `LoadedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    LoadedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[LoadedBehaviorView::TYPE, LoadedMessageBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "LoadedBehavior",
    reference: "Views/Pages/LoadedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&LoadedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
