//! Port of `Pages/CommandBar/CommandBarLabelPositionPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarLabelPositionPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct CommandBarLabelPositionPage {
    base: UserControl,
}

user_control_class!(CommandBarLabelPositionPage);
ferro_class_info!(CommandBarLabelPositionPage { new: CommandBarLabelPositionPage::new });
xaml_class!(CommandBarLabelPositionPage, "/Pages/CommandBar/CommandBarLabelPositionPage.xaml");

impl CommandBarLabelPositionPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

}
