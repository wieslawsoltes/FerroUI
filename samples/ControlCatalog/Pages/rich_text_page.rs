//! Catalog examples of the WinUI rich document and editor port. Presentation,
//! document sharing and formatting commands are declared in the page's XAML.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RichTextPage {
    base: ContentPage,
}

content_page_class!(RichTextPage);
ferro_class_info!(RichTextPage {
    new: RichTextPage::new
});
xaml_class!(RichTextPage, "/Pages/RichTextPage.xaml");

impl RichTextPage {
    pub fn construct() -> Self {
        Self {
            base: ContentPage::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
