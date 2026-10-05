//! Port of `Pages/ProgressBarPage.xaml.cs`: the class of the document
//! `Pages/ProgressBarPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ProgressBarPage {
    base: ContentPage,
}

content_page_class!(ProgressBarPage);
ferro_class_info!(ProgressBarPage { new: ProgressBarPage::new });
xaml_class!(ProgressBarPage, "/Pages/ProgressBarPage.xaml");

impl ProgressBarPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
