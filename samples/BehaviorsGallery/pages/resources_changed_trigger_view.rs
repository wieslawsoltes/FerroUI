//! Port of `Views/Pages/ResourcesChangedTriggerView` of the reference sample: the class of `pages/ResourcesChangedTriggerView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Border, Button, Control, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct ResourcesChangedTriggerView {
    base: UserControl,
}

user_control_class!(ResourcesChangedTriggerView);
ferro_class_info!(ResourcesChangedTriggerView { new: ResourcesChangedTriggerView::new });
xaml_class!(ResourcesChangedTriggerView, "/pages/ResourcesChangedTriggerView.xaml");

impl ResourcesChangedTriggerView {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    /// `ResourcesChangedTriggerView()`: the button sets the resource `Color` of the border to a blue brush.
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
                    let brush: Rc<dyn IBrush> = Brushes::blue();
                    border.resources().set("Color", Some(Rc::new(brush) as BoxedValue));
                }
            });
        }
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    ResourcesChangedTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ResourcesChangedTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "ResourcesChangedTrigger",
    reference: "Views/Pages/ResourcesChangedTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&ResourcesChangedTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
