//! Port of `Pages/SplitViewPage.xaml.cs`: the class of the document
//! `Pages/SplitViewPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::SplitViewPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct SplitViewPage {
    base: ContentPage,
}

content_page_class!(SplitViewPage);
ferro_class_info!(SplitViewPage { new: SplitViewPage::new });
xaml_class!(SplitViewPage, "/Pages/SplitViewPage.xaml");

impl SplitViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(SplitViewPageViewModel::new() as BoxedValue));
        this
    }
}
