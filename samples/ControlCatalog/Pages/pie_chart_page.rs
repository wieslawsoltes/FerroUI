//! Not from upstream: the page of the PieChart of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/PieChartPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct PieChartPage {
    base: ContentPage,
}

content_page_class!(PieChartPage);
ferro_class_info!(PieChartPage { new: PieChartPage::new });
xaml_class!(PieChartPage, "/Pages/PieChartPage.xaml");

impl PieChartPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
