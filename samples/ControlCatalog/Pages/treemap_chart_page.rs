//! Not from upstream: the page of the TreemapChart of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/TreemapChartPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct TreemapChartPage {
    base: ContentPage,
}

content_page_class!(TreemapChartPage);
ferro_class_info!(TreemapChartPage { new: TreemapChartPage::new });
xaml_class!(TreemapChartPage, "/Pages/TreemapChartPage.xaml");

impl TreemapChartPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
