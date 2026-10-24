//! Charts sample shared by desktop, browser and embedded FerroUI hosts.
//!
//! The controls, axes, series, resources and layout come from compiled XAML.
//! Rust attaches shared observable data and the five interactive commands.
//! The default library has no desktop backend dependency.

extern crate self as charts_demo;

mod register_types;
pub use register_types::{register_types, ASSEMBLY};

ferroui_markup_xaml::include_compiled_xaml!();

#[cfg(feature = "desktop")]
mod app;
#[cfg(feature = "desktop")]
pub use app::App;

use ferroui_base::animation::TimeSpan;
use ferroui_base::collections::FerroList;
use ferroui_base::controls::NameScope;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ObjectType, Ref, WeakRef};
use ferroui_controls::{Button, ItemsSource, TextBlock, ThemeVariantScope, UserControl};
use ferroui_controls_charts::{
    BarOrientation, BarSeries, Chart, DataLabelOverride, DataMarkerOverride, MarkerShape, Samples,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Sources {
    categories: Rc<FerroList<String>>,
    dates: Rc<FerroList<DateTimeOffset>>,
    actual: Rc<FerroList<f64>>,
    target: Rc<FerroList<f64>>,
}

impl Sources {
    fn new(generation: usize) -> Self {
        let base = DateTimeOffset::new(2026, 1, 1, 0, 0, 0, TimeSpan::ZERO);
        let bump = generation as f64 * 3.0;
        Self {
            categories: Rc::new(FerroList::from_items((1..=6).map(|i| format!("P{i}")))),
            dates: Rc::new(FerroList::from_items(
                (0..6).map(|i| base.add_days(i as f64)),
            )),
            actual: Rc::new(FerroList::from_items(
                [24., 37., 31., 49., 42., 58.].map(|x| x + bump),
            )),
            target: Rc::new(FerroList::from_items(
                [30., 33., 36., 39., 42., 45.].map(|x| x + bump),
            )),
        }
    }
}

struct DemoState {
    sources: RefCell<Sources>,
    categories: Ref<Samples>,
    dates: Ref<Samples>,
    actual: Ref<Samples>,
    target: Ref<Samples>,
    charts: [Ref<Chart>; 4],
    bars: [Ref<BarSeries>; 2],
    theme: WeakRef<ThemeVariantScope>,
    status: WeakRef<TextBlock>,
    generation: Cell<usize>,
}

impl DemoState {
    fn bind_sources(&self) {
        let sources = self.sources.borrow();
        self.categories
            .set_source(Some(ItemsSource::from(sources.categories.clone())));
        self.dates
            .set_source(Some(ItemsSource::from(sources.dates.clone())));
        self.actual
            .set_source(Some(ItemsSource::from(sources.actual.clone())));
        self.target
            .set_source(Some(ItemsSource::from(sources.target.clone())));
    }

    fn update_status(&self) {
        if let Some(status) = self.status.upgrade() {
            status.set_text(Some(&format!(
                "{} samples · source generation {} · one category source and two value sources shared across charts",
                self.actual.count(), self.generation.get() + 1,
            )));
        }
    }

    fn add_sample(&self) {
        let sources = self.sources.borrow();
        let n = sources.actual.count();
        let base = DateTimeOffset::new(2026, 1, 1, 0, 0, 0, TimeSpan::ZERO);
        sources.categories.add(format!("P{}", n + 1));
        sources.dates.add(base.add_days(n as f64));
        sources
            .actual
            .add(36. + (n * 13 % 37) as f64 + self.generation.get() as f64 * 3.);
        sources
            .target
            .add(30. + n as f64 * 3. + self.generation.get() as f64 * 3.);
        drop(sources);
        self.update_status();
    }

    fn replace_sources(&self) {
        self.generation.set(self.generation.get() + 1);
        *self.sources.borrow_mut() = Sources::new(self.generation.get());
        self.bind_sources();
        self.update_status();
    }

    fn toggle_theme(&self) {
        if let Some(theme) = self.theme.upgrade() {
            let dark = theme.requested_theme_variant() == Some(ThemeVariant::dark());
            theme.set_requested_theme_variant(Some(if dark {
                ThemeVariant::light()
            } else {
                ThemeVariant::dark()
            }));
        }
    }

    fn toggle_legend(&self) {
        let show = !self.charts[0].show_legend();
        for chart in &self.charts {
            chart.set_show_legend(show);
        }
    }

    fn toggle_bars(&self) {
        let orientation = match self.bars[0].orientation() {
            BarOrientation::Horizontal => BarOrientation::Vertical,
            BarOrientation::Vertical => BarOrientation::Horizontal,
        };
        for bar in &self.bars {
            bar.set_orientation(orientation);
        }
    }
}

/// A view plus the same commands used by its buttons, for hosting and smoke tests.
/// All mutation stays on the host's FerroUI UI thread.
pub struct ChartsDemo {
    view: Ref<UserControl>,
    state: Rc<DemoState>,
}

impl ChartsDemo {
    pub fn new() -> Self {
        register_types();
        let value = compiled_markup::try_load(None, "ferres://ChartsDemo/MainView.xaml")
            .expect("compiled Charts sample should load");
        let view: Ref<UserControl> = from_markup_value(&value).expect("MainView is a UserControl");
        let scope = NameScope::get_name_scope(&view).expect("compiled document has a name scope");
        let charts = ["LineChart", "AreaChart", "BarChart", "DateChart"]
            .map(|name| scope.get_as::<Chart>(name));
        for chart in &charts {
            let x_axis = chart
                .axes()
                .get(0)
                .expect("declared category/calendar axis");
            let y_axis = chart.axes().get(1).expect("declared numeric axis");
            for series in chart.series().snapshot().iter() {
                series
                    .set_x_axis(Some(x_axis.clone()))
                    .expect("axis belongs to this chart");
                series
                    .set_y_axis(Some(y_axis.clone()))
                    .expect("axis belongs to this chart");
            }
        }
        let categories = Samples::new();
        let dates = Samples::new();
        let actual = Samples::new();
        let target = Samples::new();
        for (index, chart) in charts.iter().enumerate() {
            let series = chart.series().get(0).expect("first declared series");
            series.set_x_values(Some(if index == 3 {
                dates.clone()
            } else {
                categories.clone()
            }));
            series.set_y_values(Some(actual.clone()));
        }
        for chart in &charts[..3] {
            let series = chart.series().get(1).expect("second declared series");
            series.set_x_values(Some(categories.clone()));
            series.set_y_values(Some(target.clone()));
        }
        let first = charts[0].series().get(0).unwrap();
        first
            .data_label_overrides()
            .set(0, DataLabelOverride::new("Launch", None));
        first
            .data_marker_overrides()
            .set(0, DataMarkerOverride::new(MarkerShape::Asterisk, None));
        let bars = [0, 1].map(|index| {
            charts[2]
                .series()
                .get(index)
                .unwrap()
                .cast::<BarSeries>()
                .unwrap()
        });
        let state = Rc::new(DemoState {
            sources: RefCell::new(Sources::new(0)),
            categories,
            dates,
            actual,
            target,
            charts,
            bars,
            theme: scope.get_as::<ThemeVariantScope>("ThemeScope").downgrade(),
            status: scope.get_as::<TextBlock>("Status").downgrade(),
            generation: Cell::new(0),
        });
        state.bind_sources();
        state.update_status();
        for (name, action) in [
            ("AddPoint", DemoState::add_sample as fn(&DemoState)),
            ("ReplaceData", DemoState::replace_sources),
            ("ToggleTheme", DemoState::toggle_theme),
            ("ToggleLegend", DemoState::toggle_legend),
            ("ToggleBars", DemoState::toggle_bars),
        ] {
            let state = state.clone();
            scope
                .get_as::<Button>(name)
                .click(move |_, _| action(&state));
        }
        Self { view, state }
    }

    pub fn view(&self) -> Ref<UserControl> {
        self.view.clone()
    }
    pub fn charts(&self) -> &[Ref<Chart>; 4] {
        &self.state.charts
    }
    pub fn sample_count(&self) -> usize {
        self.state.actual.count()
    }
    pub fn add_sample(&self) {
        self.state.add_sample();
    }
    pub fn replace_sources(&self) {
        self.state.replace_sources();
    }
    pub fn toggle_theme(&self) {
        self.state.toggle_theme();
    }
    pub fn toggle_legend(&self) {
        self.state.toggle_legend();
    }
    pub fn toggle_bars(&self) {
        self.state.toggle_bars();
    }

    /// Named controls remain available for host diagnostics. Nonvisual series
    /// are obtained from their owning Chart collections.
    pub fn find<T: ObjectType>(&self, name: &str) -> Ref<T> {
        NameScope::get_name_scope(&self.view)
            .expect("document scope")
            .get_as(name)
    }
}

impl Default for ChartsDemo {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates the sample for a browser, mobile or desktop host with a FerroUI theme.
/// Button handlers retain their state for as long as the view is alive.
pub fn create_view() -> Ref<UserControl> {
    ChartsDemo::new().view()
}

#[cfg(all(test, feature = "render-smoke"))]
mod render_smoke;
#[cfg(test)]
mod tests;
