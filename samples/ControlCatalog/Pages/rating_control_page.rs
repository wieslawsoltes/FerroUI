//! Not from upstream: the page of WinUI's RatingControl
//! (docs/porting/winui-controls.md), the class of `Pages/RatingControlPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct RatingControlPage {
    base: ContentPage,
}

content_page_class!(RatingControlPage);
ferro_class_info!(RatingControlPage { new: RatingControlPage::new });
xaml_class!(RatingControlPage, "/Pages/RatingControlPage.xaml");

impl RatingControlPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
