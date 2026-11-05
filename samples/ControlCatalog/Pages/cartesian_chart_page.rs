//! Not from upstream: the page of the CartesianChart of the LiveCharts2 port
//! (docs/porting/livecharts2.md), the class of `Pages/CartesianChartPage.xaml`. The page adds
//! the daily prices of its OHLC chart, whose X values are the ticks of their dates, and the
//! colours of its heat map.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::media::Color;
use ferroui_base::utilities::DateTime;
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::ContentPage;
use ferroui_controls_livecharts::{CartesianChart, HeatSeries};
use ferroui_livecharts::kernel::Coordinate;

/// High, open, close and low of six days, from the upstream OHLC snapshot.
const PRICES: &[(f64, f64, f64, f64)] = &[
    (523.0, 500.0, 450.0, 400.0),
    (500.0, 450.0, 425.0, 400.0),
    (490.0, 425.0, 400.0, 380.0),
    (420.0, 400.0, 420.0, 380.0),
    (520.0, 420.0, 490.0, 400.0),
    (580.0, 490.0, 560.0, 440.0),
];

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
        if let Some(chart) = this.find_control::<CartesianChart>("PricesChart") {
            let first = DateTime::new(2021, 1, 1);
            if let Some(series) = chart.series().to_vec().first() {
                series.set_coordinates(PRICES.iter().enumerate().map(
                    |(day, &(high, open, close, low))| {
                        let date = first.add_days(day as f64);
                        Coordinate::financial(date.ticks() as f64, high, open, close, low)
                    },
                ));
            }
        }
        if let Some(chart) = this.find_control::<CartesianChart>("HeatChart") {
            let series = chart.series().to_vec();
            if let Some(heat) = series.first().and_then(|s| s.cast::<HeatSeries>()) {
                heat.set_heat_map(vec![
                    Color::from_rgb(255, 241, 118),
                    Color::from_rgb(47, 79, 79),
                    Color::from_rgb(0, 0, 255),
                ]);
                heat.set_color_stops(Some(vec![0.0, 0.5, 1.0]));
            }
        }
        this
    }
}
