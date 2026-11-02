//! Not from upstream: the page of the PolarChart of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/PolarChartPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct PolarChartPage {
    base: ContentPage,
}

content_page_class!(PolarChartPage);
ferro_class_info!(PolarChartPage { new: PolarChartPage::new });
xaml_class!(PolarChartPage, "/Pages/PolarChartPage.xaml");

impl PolarChartPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
