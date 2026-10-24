//! Port of `Views/Pages/ResourcesChangedBehaviorView` of the reference sample: the class of `pages/ResourcesChangedBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::behaviors::resources_changed_message_behavior::ResourcesChangedMessageBehavior;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Border, Button, Control, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct ResourcesChangedBehaviorView {
    base: UserControl,
}

user_control_class!(ResourcesChangedBehaviorView);
ferro_class_info!(ResourcesChangedBehaviorView { new: ResourcesChangedBehaviorView::new });
xaml_class!(ResourcesChangedBehaviorView, "/pages/ResourcesChangedBehaviorView.xaml");

impl ResourcesChangedBehaviorView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ResourcesChangedBehaviorView()`: the button sets the resource `Color` of the border to a red brush.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        if let (Some(button), Some(border)) = (
            this.find_control::<Button>("ChangeButton"),
            this.find_control::<Border>("Target"),
        ) {
            let border = border.downgrade();
            button.add_handler(&Button::click_event(), move |_, _| {
                if let Some(border) = border.upgrade() {
                    let brush: Rc<dyn IBrush> = Brushes::red();
                    border.resources().set("Color", Some(Rc::new(brush) as BoxedValue));
                }
            });
        }
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ResourcesChangedBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`): the page and the behavior of the sample it uses.
pub(crate) const TYPES: &[&TypeInfo] = &[ResourcesChangedBehaviorView::TYPE, ResourcesChangedMessageBehavior::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ResourcesChangedBehavior",
    reference: "Views/Pages/ResourcesChangedBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&ResourcesChangedBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
