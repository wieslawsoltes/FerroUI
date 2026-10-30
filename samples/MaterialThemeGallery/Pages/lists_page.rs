//! The class of the document `Pages/ListsPage.xaml`: ListBox, ListBoxItem, ItemsControl, TreeView and TreeViewItem.

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
pub struct ListsPage {
    base: UserControl,
}

ferro_class!(ListsPage: UserControl);
ferro_impl_classes!(
    ListsPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(ListsPage { new: ListsPage::new });
xaml_class!(ListsPage, "/Pages/ListsPage.xaml");

impl ListsPage {
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
    ListsPage::new().upcast()
}
