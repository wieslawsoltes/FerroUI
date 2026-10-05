//! Port of `Pages/TransitioningContentControlPage.xaml.cs`: the class of the document
//! `Pages/TransitioningContentControlPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct TransitioningContentControlPage {
    base: ContentPage,
}

content_page_class!(TransitioningContentControlPage);
ferro_class_info!(TransitioningContentControlPage { new: TransitioningContentControlPage::new });
xaml_class!(TransitioningContentControlPage, "/Pages/TransitioningContentControlPage.xaml");

impl TransitioningContentControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
