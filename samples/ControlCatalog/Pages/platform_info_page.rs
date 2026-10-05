//! Port of `Pages/PlatformInfoPage.xaml.cs`: the class of the document
//! `Pages/PlatformInfoPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::PlatformInformationViewModel;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct PlatformInfoPage {
    base: ContentPage,
}

content_page_class!(PlatformInfoPage);
ferro_class_info!(PlatformInfoPage { new: PlatformInfoPage::new });
xaml_class!(PlatformInfoPage, "/Pages/PlatformInfoPage.xaml");

impl PlatformInfoPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(PlatformInformationViewModel::new() as BoxedValue));
        this
    }
}
