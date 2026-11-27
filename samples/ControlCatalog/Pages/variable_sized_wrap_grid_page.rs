//! Not from upstream: the page of WinUI's VariableSizedWrapGrid (docs/porting/winui-controls.md,
//! section 10.19), the class of `Pages/VariableSizedWrapGridPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct VariableSizedWrapGridPage {
    base: ContentPage,
}

content_page_class!(VariableSizedWrapGridPage);
ferro_class_info!(VariableSizedWrapGridPage { new: VariableSizedWrapGridPage::new });
xaml_class!(VariableSizedWrapGridPage, "/Pages/VariableSizedWrapGridPage.xaml");

impl VariableSizedWrapGridPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
