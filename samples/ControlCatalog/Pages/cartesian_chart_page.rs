//! Not from upstream: the page of the CartesianChart of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/CartesianChartPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;

#[repr(C)]
pub struct CartesianChartPage {
    base: ContentPage,
}

content_page_class!(CartesianChartPage);
ferro_class_info!(CartesianChartPage { new: CartesianChartPage::new });
xaml_class!(CartesianChartPage, "/Pages/CartesianChartPage.xaml");

impl CartesianChartPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
