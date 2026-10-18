//! Port of the code-behind of `Views/TemplateColumnBugPage` (`.cs`): the class of the document
//! `Views/TemplateColumnBugPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct TemplateColumnBugPage {
    base: UserControl,
}

user_control_class!(TemplateColumnBugPage);
ferro_class_info!(TemplateColumnBugPage { new: TemplateColumnBugPage::new });
xaml_class!(TemplateColumnBugPage, "/Views/TemplateColumnBugPage.xaml");

impl TemplateColumnBugPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
