//! The gauge builders of the pinned SkiaSharp extensions (`GaugeGenerator`, `GaugeItem`,
//! `PieChartExtensions.AsSeries` with `GaugeOptions`): the pie series of a solid gauge or of the
//! sections of an angular gauge, and the fill series behind them. The theme's gauge rules
//! (`HasRuleForGaugeSeries`, `HasRuleForGaugeFillSeries`) are applied by the chart to every gauge
//! series (`Chart`), so a gauge built here or by hand looks the same.

use crate::{ChartPoint, ChartPoints, PieSeries};
use ferroui_base::Ref;
use ferroui_livecharts::kernel::Coordinate;
use std::rc::Rc;

/// An item of a gauge (`GaugeItem`): its value and a builder that styles its series. The value
/// [`GaugeItem::BACKGROUND`] makes the item the builder of the fill series behind the gauge.
#[derive(Clone)]
pub struct GaugeItem {
    value: f64,
    builder: Option<Rc<dyn Fn(&PieSeries)>>,
}

impl GaugeItem {
    /// The value of the item that styles the fill series (`GaugeItem.Background`).
    pub const BACKGROUND: f64 = f64::MAX;

    pub fn new(value: f64) -> Self {
        Self {
            value,
            builder: None,
        }
    }
    /// The item with a builder of its series (`GaugeItem(value, builder)`), which runs after the
    /// generator has set the series up.
    pub fn with_builder(mut self, builder: impl Fn(&PieSeries) + 'static) -> Self {
        self.builder = Some(Rc::new(builder));
        self
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Whether the item styles the fill series (`IsFillSeriesBuilder`).
    pub fn is_fill_series_builder(&self) -> bool {
        self.value == Self::BACKGROUND
    }
}

impl std::fmt::Debug for GaugeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GaugeItem")
            .field("value", &self.value)
            .field("builder", &self.builder.is_some())
            .finish()
    }
}

/// The builders of the pie series of a gauge (`GaugeGenerator`).
pub struct GaugeGenerator;

impl GaugeGenerator {
    /// The series of a solid gauge (`BuildSolidGauge`): a gauge series for each item, whose value
    /// is at the place of the item among empty values (one ring each), and the fill series, a
    /// zero for each item, styled by the background items. A fill series is added when no item
    /// styles it.
    pub fn build_solid_gauge(items: &[GaugeItem]) -> Vec<Ref<PieSeries>> {
        Self::build(false, items)
    }

    /// The series of the sections of an angular gauge (`BuildAngularGaugeSections`): a gauge
    /// series of one value for each item, relative to the minimum value of the chart, without a
    /// hover push-out or data labels, and the fill series, which is not hovered either.
    pub fn build_angular_gauge_sections(items: &[GaugeItem]) -> Vec<Ref<PieSeries>> {
        Self::build(true, items)
    }

    fn build(angular: bool, items: &[GaugeItem]) -> Vec<Ref<PieSeries>> {
        let mut items = items.to_vec();
        if !items.iter().any(GaugeItem::is_fill_series_builder) {
            items.push(GaugeItem::new(GaugeItem::BACKGROUND));
        }
        let (backgrounds, sections): (Vec<_>, Vec<_>) =
            items.iter().partition(|item| item.is_fill_series_builder());
        let count = sections.len();
        let mut result: Vec<_> = sections
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let series = PieSeries::new();
                series.set_is_gauge(true);
                if angular {
                    series.set_values(points([Some(item.value)]));
                    series.set_data_labels(false);
                    series.set_is_relative_to_min_value(true);
                } else {
                    series.set_values(points(
                        (0..count).map(|ring| (ring == i).then_some(item.value)),
                    ));
                }
                series.set_hover_push_out(0.0);
                if let Some(builder) = &item.builder {
                    builder(&series);
                }
                series
            })
            .collect();
        let background = PieSeries::new();
        background.set_is_gauge(true);
        background.set_is_fill_series(true);
        background.set_values(points((0..items.len() - 1).map(|_| Some(0.0))));
        if angular {
            background.set_hover_push_out(0.0);
            background.set_data_labels(false);
        }
        for item in backgrounds {
            if let Some(builder) = &item.builder {
                builder(&background);
            }
        }
        result.push(background);
        result
    }
}

/// The values of a gauge series: a number, or an empty value (the source's `null` model).
fn points(values: impl IntoIterator<Item = Option<f64>>) -> ChartPoints {
    ChartPoints::from_items(values.into_iter().enumerate().map(|(i, value)| {
        ChartPoint::from_coordinate(match value {
            Some(y) => Coordinate::new(i as f64, y),
            None => Coordinate {
                secondary_value: i as f64,
                ..Coordinate::EMPTY
            },
        })
    }))
}
