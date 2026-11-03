//! Not from upstream: the page of the GeoMap of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/GeoMapPage.xaml`. The button of the page
//! turns its globe with `rotate_to`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ContentPage};
use ferroui_controls_livecharts::GeoMap;

#[repr(C)]
pub struct GeoMapPage {
    base: ContentPage,
}

content_page_class!(GeoMapPage);
ferro_class_info!(GeoMapPage { new: GeoMapPage::new });
xaml_class!(GeoMapPage, "/Pages/GeoMapPage.xaml");

impl GeoMapPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        if let Some(button) = this.find_control::<Button>("RotateGlobe") {
            let weak = this.downgrade();
            button.click(move |_, _| {
                let Some(this) = weak.upgrade() else {
                    return;
                };
                if let Some(globe) = this.find_control::<GeoMap>("Globe") {
                    // 60 degrees east in the 800 ms of the source's RotateTo.
                    let (longitude, latitude) = (globe.center_longitude(), globe.center_latitude());
                    globe.rotate_to(longitude + 60.0, latitude, 800.0);
                }
            });
        }
        this
    }
}
