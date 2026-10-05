//! Port of `Pages/CursorPage.xaml.cs`: the class of the document
//! `Pages/CursorPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::CursorPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct CursorPage {
    base: ContentPage,
}

content_page_class!(CursorPage);
ferro_class_info!(CursorPage { new: CursorPage::new });
xaml_class!(CursorPage, "/Pages/CursorPage.xaml");

impl CursorPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(CursorPageViewModel::new() as BoxedValue));
        this
    }
}
