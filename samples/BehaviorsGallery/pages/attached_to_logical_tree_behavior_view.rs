//! Port of `Views/Pages/AttachedToLogicalTreeBehaviorView` of the reference sample: the class of `pages/AttachedToLogicalTreeBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::attached_to_logical_tree_message_behavior::AttachedToLogicalTreeMessageBehavior;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct AttachedToLogicalTreeBehaviorView {
    base: UserControl,
}

user_control_class!(AttachedToLogicalTreeBehaviorView);
ferro_class_info!(AttachedToLogicalTreeBehaviorView { new: AttachedToLogicalTreeBehaviorView::new });
xaml_class!(AttachedToLogicalTreeBehaviorView, "/pages/AttachedToLogicalTreeBehaviorView.xaml");

impl AttachedToLogicalTreeBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `AttachedToLogicalTreeBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AttachedToLogicalTreeBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[AttachedToLogicalTreeBehaviorView::TYPE, AttachedToLogicalTreeMessageBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AttachedToLogicalTreeBehavior",
    reference: "Views/Pages/AttachedToLogicalTreeBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AttachedToLogicalTreeBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
