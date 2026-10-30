//! Port of `Views/Pages/EventTriggerBehaviorView` of the reference sample: the class of
//! `pages/EventTriggerBehaviorView.xaml`, which removes the sample view when the page is pressed.

use crate::markup::xaml_class;
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::input::{InputElementImpl, InputElementImplExt, PointerPressedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControl, ContentControlImpl, Control, ControlImpl, UserControl};

#[repr(C)]
pub struct EventTriggerBehaviorView {
    base: UserControl,
}

ferro_class!(EventTriggerBehaviorView: UserControl);
ferro_impl_classes!(
    EventTriggerBehaviorView: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(EventTriggerBehaviorView {
    new: EventTriggerBehaviorView::new
});
xaml_class!(
    EventTriggerBehaviorView,
    "/pages/EventTriggerBehaviorView.xaml"
);

impl InputElementImpl for EventTriggerBehaviorView {
    /// `OnPointerPressed(PointerPressedEventArgs)`: the content control of the page loses its
    /// content (the sample view, whose behaviors write when it is detached).
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        Self::parent_on_pointer_pressed(this, e);

        if let Some(content_control) = this.find_control::<ContentControl>("ContentControl") {
            content_control.set_content(None);
        }
    }
}

impl EventTriggerBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `EventTriggerBehaviorView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    EventTriggerBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[EventTriggerBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "EventTriggerBehavior",
    reference: "Views/Pages/EventTriggerBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&EventTriggerBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
