//! Port of `Views/Pages/DisposingBehaviorView` of the reference sample: the class of `pages/DisposingBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::disposing_message_behavior::DisposingMessageBehavior;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DisposingBehaviorView {
    base: UserControl,
}

user_control_class!(DisposingBehaviorView);
ferro_class_info!(DisposingBehaviorView { new: DisposingBehaviorView::new });
xaml_class!(DisposingBehaviorView, "/pages/DisposingBehaviorView.xaml");

impl DisposingBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DisposingBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DisposingBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[DisposingBehaviorView::TYPE, DisposingMessageBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DisposingBehavior",
    reference: "Views/Pages/DisposingBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&DisposingBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
