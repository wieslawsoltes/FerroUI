//! Not from upstream: the page of WinUI's Pivot
//! (docs/porting/winui-calculator-gaps.md, PV-3), the class of `Pages/PivotPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct PivotPage {
    base: ContentPage,
}

content_page_class!(PivotPage);
ferro_class_info!(PivotPage { new: PivotPage::new });
xaml_class!(PivotPage, "/Pages/PivotPage.xaml");

impl PivotPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
