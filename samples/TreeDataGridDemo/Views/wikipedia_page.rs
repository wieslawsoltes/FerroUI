//! The Wikipedia page: the tab "Wikipedia" of the reference demo's main window (`MainWindow`)
//! as a view of its own, which the main window and the ControlCatalog both show. The tab has no
//! handlers in the reference's code-behind.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::UserControl;

#[repr(C)]
pub struct WikipediaPage {
    base: UserControl,
}

user_control_class!(WikipediaPage);
ferro_class_info!(WikipediaPage { new: WikipediaPage::new });
xaml_class!(WikipediaPage, "/Views/WikipediaPage.xaml");

impl WikipediaPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
