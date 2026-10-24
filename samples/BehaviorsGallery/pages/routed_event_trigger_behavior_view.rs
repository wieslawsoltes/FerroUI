//! Port of `Views/Pages/RoutedEventTriggerBehaviorView` of the reference sample: the class of `pages/RoutedEventTriggerBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};

#[repr(C)]
pub struct RoutedEventTriggerBehaviorView {
    base: UserControl,
}

user_control_class!(RoutedEventTriggerBehaviorView);
ferro_class_info!(RoutedEventTriggerBehaviorView { new: RoutedEventTriggerBehaviorView::new });
xaml_class!(RoutedEventTriggerBehaviorView, "/pages/RoutedEventTriggerBehaviorView.xaml");

impl RoutedEventTriggerBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `RoutedEventTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    RoutedEventTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[RoutedEventTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "RoutedEventTriggerBehavior",
    reference: "Views/Pages/RoutedEventTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&RoutedEventTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
