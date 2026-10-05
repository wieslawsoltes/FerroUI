//! Port of `Pages/ListBoxPage.xaml.cs`: the class of the document
//! `Pages/ListBoxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::ListBoxPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ListBoxPage {
    base: ContentPage,
}

content_page_class!(ListBoxPage);
ferro_class_info!(ListBoxPage { new: ListBoxPage::new });
xaml_class!(ListBoxPage, "/Pages/ListBoxPage.xaml");

impl ListBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(ListBoxPageViewModel::new() as BoxedValue));
        this
    }
}
