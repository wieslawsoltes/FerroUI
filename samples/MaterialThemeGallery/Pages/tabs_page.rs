//! The class of the document `Pages/TabsPage.xaml`: TabControl, TabItem, TabStrip and TabStripItem.

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
pub struct TabsPage {
    base: UserControl,
}

ferro_class!(TabsPage: UserControl);
ferro_impl_classes!(
    TabsPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(TabsPage { new: TabsPage::new });
xaml_class!(TabsPage, "/Pages/TabsPage.xaml");

impl TabsPage {
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
    TabsPage::new().upcast()
}
