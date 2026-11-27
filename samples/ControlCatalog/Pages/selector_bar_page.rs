//! The class of the document `Pages/SelectorBarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct SelectorBarPage {
    base: ContentPage,
}

content_page_class!(SelectorBarPage);
ferro_class_info!(SelectorBarPage { new: SelectorBarPage::new });
xaml_class!(SelectorBarPage, "/Pages/SelectorBarPage.xaml");

impl SelectorBarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
