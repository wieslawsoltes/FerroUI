//! Not from upstream: the page of the icons of WinUI (SymbolIcon, FontIcon) and PathIcon
//! (docs/porting/winui-controls.md), the class of `Pages/IconElementPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct IconElementPage {
    base: ContentPage,
}

content_page_class!(IconElementPage);
ferro_class_info!(IconElementPage { new: IconElementPage::new });
xaml_class!(IconElementPage, "/Pages/IconElementPage.xaml");

impl IconElementPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
