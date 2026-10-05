//! Port of `Pages/WrapPanelPage.xaml.cs`: the class of the document
//! `Pages/WrapPanelPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::WrapPanelPageViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct WrapPanelPage {
    base: ContentPage,
}

content_page_class!(WrapPanelPage);
ferro_class_info!(WrapPanelPage { new: WrapPanelPage::new });
xaml_class!(WrapPanelPage, "/Pages/WrapPanelPage.xaml");

impl WrapPanelPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(WrapPanelPageViewModel::new() as BoxedValue));
        this
    }
}
