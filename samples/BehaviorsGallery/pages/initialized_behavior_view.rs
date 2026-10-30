//! Port of `Views/Pages/InitializedBehaviorView` of the reference sample: the class of `pages/InitializedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::initialized_message_behavior::InitializedMessageBehavior;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct InitializedBehaviorView {
    base: UserControl,
}

user_control_class!(InitializedBehaviorView);
ferro_class_info!(InitializedBehaviorView { new: InitializedBehaviorView::new });
xaml_class!(InitializedBehaviorView, "/pages/InitializedBehaviorView.xaml");

impl InitializedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `InitializedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    InitializedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[InitializedBehaviorView::TYPE, InitializedMessageBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "InitializedBehavior",
    reference: "Views/Pages/InitializedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&InitializedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
