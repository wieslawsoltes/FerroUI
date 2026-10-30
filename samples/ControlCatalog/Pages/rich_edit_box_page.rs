//! Not from upstream: the page of WinUI's RichEditBox (docs/porting/winui-rich-text.md), the
//! class of `Pages/RichEditBoxPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RichEditBoxPage {
    base: ContentPage,
}

content_page_class!(RichEditBoxPage);
ferro_class_info!(RichEditBoxPage { new: RichEditBoxPage::new });
xaml_class!(RichEditBoxPage, "/Pages/RichEditBoxPage.xaml");

impl RichEditBoxPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
