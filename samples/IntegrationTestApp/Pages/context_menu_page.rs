//! Port of `Pages/ContextMenuPage.xaml.cs`: the class of the document
//! `Pages/ContextMenuPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct ContextMenuPage {
    base: UserControl,
}

user_control_class!(ContextMenuPage);
ferro_class_info!(ContextMenuPage { new: ContextMenuPage::new });
xaml_class!(ContextMenuPage, "/Pages/ContextMenuPage.xaml");

impl ContextMenuPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
