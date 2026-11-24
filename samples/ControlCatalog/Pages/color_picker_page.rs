//! Port of `Pages/ColorPickerPage.xaml.cs`: the class of the document
//! `Pages/ColorPickerPage.xaml`. Since upstream 74b4e97962 the page is markup only: its
//! sections show every picker, the code-behind adds none.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct ColorPickerPage {
    base: ContentPage,
}

content_page_class!(ColorPickerPage);
ferro_class_info!(ColorPickerPage { new: ColorPickerPage::new });
xaml_class!(ColorPickerPage, "/Pages/ColorPickerPage.xaml");

impl ColorPickerPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
