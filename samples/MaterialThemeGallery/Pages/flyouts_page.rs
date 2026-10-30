//! The class of the document `Pages/FlyoutsPage.xaml`: FlyoutPresenter, ToolTip, NotificationCard and WindowNotificationManager.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, Control, ControlImpl, UserControl};

#[repr(C)]
pub struct FlyoutsPage {
    base: UserControl,
}

ferro_class!(FlyoutsPage: UserControl);
ferro_impl_classes!(
    FlyoutsPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(FlyoutsPage { new: FlyoutsPage::new });
xaml_class!(FlyoutsPage, "/Pages/FlyoutsPage.xaml");

impl FlyoutsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}

/// The page as a control (the entry of the page table).
pub fn create() -> Ref<Control> {
    FlyoutsPage::new().upcast()
}
