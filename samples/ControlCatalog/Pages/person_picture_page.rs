//! Not from upstream: the page of WinUI's PersonPicture
//! (docs/porting/winui-controls.md, G2), the class of `Pages/PersonPicturePage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct PersonPicturePage {
    base: ContentPage,
}

content_page_class!(PersonPicturePage);
ferro_class_info!(PersonPicturePage { new: PersonPicturePage::new });
xaml_class!(PersonPicturePage, "/Pages/PersonPicturePage.xaml");

impl PersonPicturePage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
