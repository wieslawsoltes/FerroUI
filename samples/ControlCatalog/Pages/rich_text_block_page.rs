//! Not from upstream: the page of WinUI's RichTextBlock and RichTextBlockOverflow
//! (docs/porting/winui-rich-text.md), the class of `Pages/RichTextBlockPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RichTextBlockPage {
    base: ContentPage,
}

content_page_class!(RichTextBlockPage);
ferro_class_info!(RichTextBlockPage { new: RichTextBlockPage::new });
xaml_class!(RichTextBlockPage, "/Pages/RichTextBlockPage.xaml");

impl RichTextBlockPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
