//! Port of `Views/Pages/MouseDragBehaviorView` of the reference sample: the class of `pages/MouseDragBehaviorView.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, UserControl};
use ferroui_xaml_interactions_draggable::MultiMouseDragElementBehavior;
use ferroui_xaml_interactivity::Interaction;

#[repr(C)]
pub struct MouseDragBehaviorView {
    base: UserControl,
}

user_control_class!(MouseDragBehaviorView);
ferro_class_info!(MouseDragBehaviorView {
    new: MouseDragBehaviorView::new
});
xaml_class!(MouseDragBehaviorView, "/pages/MouseDragBehaviorView.xaml");

impl MouseDragBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `MouseDragBehaviorView()`: the multi drag behavior of the first rectangle also moves the
    /// two others.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let rect1 = this.find_control::<Control>("MultiRect1");
        let rect2 = this.find_control::<Control>("MultiRect2");
        let rect3 = this.find_control::<Control>("MultiRect3");
        if let (Some(rect1), Some(rect2), Some(rect3)) = (rect1, rect2, rect3) {
            let behavior = Interaction::get_behaviors(&rect1)
                .list()
                .to_vec()
                .into_iter()
                .find_map(|behavior| behavior.cast::<MultiMouseDragElementBehavior>());
            if let Some(behavior) = behavior {
                behavior.target_controls().add(rect2);
                behavior.target_controls().add(rect3);
            }
        }
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    MouseDragBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[MouseDragBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Mouse Drag Behaviors",
    reference: "Views/Pages/MouseDragBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&MouseDragBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
