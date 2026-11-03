//! Native on-screen keyboard component; the complete view is compiled from XAML.
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{Ref, ferro_class_info, instantiate};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct FerroBoardPage {
    base: ContentPage,
}
content_page_class!(FerroBoardPage);
ferro_class_info!(FerroBoardPage {
    new: FerroBoardPage::new
});
xaml_class!(FerroBoardPage, "/Pages/FerroBoardPage.xaml");

impl FerroBoardPage {
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
