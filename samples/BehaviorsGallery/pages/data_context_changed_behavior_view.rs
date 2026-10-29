//! Port of `Views/Pages/DataContextChangedBehaviorView` of the reference sample: the class of `pages/DataContextChangedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::data_context_changed_message_behavior::DataContextChangedMessageBehavior;
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct DataContextChangedBehaviorView {
    base: UserControl,
}

user_control_class!(DataContextChangedBehaviorView);
ferro_class_info!(DataContextChangedBehaviorView { new: DataContextChangedBehaviorView::new });
xaml_class!(DataContextChangedBehaviorView, "/pages/DataContextChangedBehaviorView.xaml");

impl DataContextChangedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `DataContextChangedBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DataContextChangedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[DataContextChangedBehaviorView::TYPE, DataContextChangedMessageBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "DataContextChangedBehavior",
    reference: "Views/Pages/DataContextChangedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&DataContextChangedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
