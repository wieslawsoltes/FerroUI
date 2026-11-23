//! The data and the handlers of WinUI Gallery's `ChartPage.xaml.cs` (14061eea; see NOTICE.md) in
//! Rust, on the native charts of `ferroui-controls-charts`, the same API as the source's
//! `Microsoft.UI.Xaml.Controls.Charts`. The charts own layout, scaling, drawing and input; the
//! sample owns the data of the page and its options. The `x:Bind` connections of the source (the
//! axes and samples of each series) are made here, by the position in `Chart.Axes` and `Chart.Data`.

use ferroui_base::animation::SignedDuration;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ObjectType, Ref, WeakRef};
use ferroui_controls::primitives::{RangeBase, SelectingItemsControl, ToggleButton};
use ferroui_controls::{Button, CheckBox, ComboBox, Control, ItemsSource, Slider, TextBlock, ToggleSwitch};
use ferroui_controls_charts::{
    AreaSeries, Axis, BarOrientation, BarSeries, CartesianAxis, CartesianSeries, CategoryAxis, CategorySortKey, Chart,
    DataLabelOverride, DataMarkerOverride, DateTimeAxis, DateTimeIntervalType, GridLines, LinearAxis, MarkerShape,
    Samples, SortOrder, StrokeDashStyle,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

// --- The data of ChartPage.xaml.cs ---------------------------------------------------------------

pub(crate) const MONTHS: [&str; 6] = ["January", "February", "March", "April", "May", "June"];
pub(crate) const PROFITS: [f64; 6] = [18.0, 26.0, 33.0, 39.0, 47.0, 52.0];
pub(crate) const ACTUALS: [f64; 6] = [12.0, 23.0, 37.0, 31.0, 46.0, 52.0];
pub(crate) const FORECASTS: [f64; 6] = [16.0, 25.0, 35.0, 40.0, 48.0, 55.0];
pub(crate) const TARGETS: [f64; 6] = [20.0, 25.0, 40.0, 40.0, 50.0, 50.0];
pub(crate) const REGIONS: [&str; 4] = ["North", "South", "East", "West"];
pub(crate) const UNITS_SOLD: [f64; 4] = [42.0, 38.0, 51.0, 47.0];
pub(crate) const QUARTERS: [&str; 4] = ["Q1", "Q2", "Q3", "Q4"];
pub(crate) const QUARTERLY_VISITORS: [f64; 4] = [35.0, 62.0, 48.0, 81.0];
pub(crate) const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
pub(crate) const DAILY_VISITS: [f64; 7] = [120.0, 135.0, 128.0, 172.0, 150.0, 98.0, 110.0];
/// `ReadingDates`: weekly from 2026-08-03 to 2026-08-31.
pub(crate) const READING_DATES: [(i32, i32); 5] = [(8, 3), (8, 10), (8, 17), (8, 24), (8, 31)];
pub(crate) const TEMPERATURE_READINGS: [f64; 5] = [72.0, 76.0, 74.0, 79.0, 77.0];
pub(crate) const INITIAL_RESPONSE_TIMES: [f64; 5] = [184.0, 179.0, 173.0, 171.0, 168.0];
pub(crate) const INCOMING_RESPONSE_TIMES: [f64; 5] = [176.0, 169.0, 181.0, 165.0, 172.0];

/// The options of the `LabelFormat` of the time axis.
const LABEL_FORMATS: [&str; 3] = ["month day", "month.abbreviated day", "shortdate"];
/// The options of the `IntervalType` of the time axis.
const INTERVALS: [DateTimeIntervalType; 4] =
    [DateTimeIntervalType::Auto, DateTimeIntervalType::Day, DateTimeIntervalType::Week, DateTimeIntervalType::Month];
/// The options of the dash style, in the order of the combo box.
const DASH_STYLES: [StrokeDashStyle; 5] = [
    StrokeDashStyle::Solid,
    StrokeDashStyle::Dash,
    StrokeDashStyle::Dot,
    StrokeDashStyle::DashDot,
    StrokeDashStyle::DashDotDot,
];
/// The options of the marker shape, in the order of the combo box.
const MARKER_SHAPES: [MarkerShape; 10] = [
    MarkerShape::None,
    MarkerShape::Square,
    MarkerShape::Diamond,
    MarkerShape::Triangle,
    MarkerShape::X,
    MarkerShape::Asterisk,
    MarkerShape::ShortDash,
    MarkerShape::LongDash,
    MarkerShape::Circle,
    MarkerShape::Plus,
];

pub(crate) fn wire(root: &Ref<Control>) {
    if find::<Chart>(root, "BasicLineChart").is_none() {
        return;
    }
    wire_basic_line(root);
    wire_bar(root);
    wire_axis_options(root);
    wire_styled_series(root);
    wire_lines_and_markers(root);
    wire_time_series(root);
    wire_live_series(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

fn is_checked_change(change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == ToggleButton::is_checked_property().as_property()
}

fn is_selection_change(change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == SelectingItemsControl::selected_index_property().as_property()
}

fn is_on<T: ObjectType + std::ops::Deref<Target = ToggleButton>>(toggle: &WeakRef<T>) -> bool {
    toggle.upgrade().is_some_and(|toggle| toggle.is_checked() == Some(true))
}

/// Runs `apply` when the selection of `combo` changes.
fn on_selection(combo: &ComboBox, apply: &Rc<dyn Fn()>) {
    let apply = apply.clone();
    combo.property_changed(move |change| {
        if is_selection_change(change) {
            apply();
        }
    });
}

/// Runs `apply` when the state of `toggle` (a check box or a toggle switch) changes.
fn on_checked(toggle: &ToggleButton, apply: &Rc<dyn Fn()>) {
    let apply = apply.clone();
    toggle.property_changed(move |change| {
        if is_checked_change(change) {
            apply();
        }
    });
}

fn fill_strings(chart: &Chart, index: usize, values: &[&str]) {
    if let Ok(samples) = chart.data().get(index) {
        samples.set_source(Some(ItemsSource::from_values(values.iter().map(|value| (*value).to_owned()))));
    }
}

fn fill_numbers(chart: &Chart, index: usize, values: &[f64]) {
    if let Ok(samples) = chart.data().get(index) {
        samples.set_source(Some(ItemsSource::from_values(values.iter().copied())));
    }
}

/// The `XAxis`, `YAxis`, `XValues` and `YValues` of a series, by their positions in the chart.
fn connect(chart: &Chart, series: usize, x: Option<(usize, usize)>, y: (usize, usize)) -> Option<Ref<CartesianSeries>> {
    let series = chart.series().get(series).ok()?;
    if let Some((axis, samples)) = x {
        series.set_x_axis(Some(chart.axes().get(axis).ok()?)).ok()?;
        series.set_x_values(Some(chart.data().get(samples).ok()?));
    }
    series.set_y_axis(Some(chart.axes().get(y.0).ok()?)).ok()?;
    series.set_y_values(Some(chart.data().get(y.1).ok()?));
    Some(series)
}

// --- Example 1: BasicLineChart --------------------------------------------------------------------

fn wire_basic_line(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("BasicLineChart");
    fill_strings(&chart, 0, &MONTHS);
    fill_numbers(&chart, 1, &PROFITS);
    connect(&chart, 0, Some((0, 0)), (1, 1));
    // LegendToggle_Toggled.
    let toggle = root.get_control::<ToggleSwitch>("LegendToggle");
    let weak_toggle = toggle.downgrade();
    let weak_chart = chart.downgrade();
    toggle.property_changed(move |change| {
        if is_checked_change(change) {
            if let Some(chart) = weak_chart.upgrade() {
                chart.set_show_legend(is_on(&weak_toggle));
            }
        }
    });
}

// --- Example 2: BarChart ---------------------------------------------------------------------------

fn wire_bar(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("BarChart");
    fill_strings(&chart, 0, &REGIONS);
    fill_numbers(&chart, 1, &UNITS_SOLD);
    let Some(series) = connect(&chart, 0, Some((0, 0)), (1, 1)).and_then(|series| series.cast::<BarSeries>()) else {
        return;
    };
    let Some(axis) = chart.axes().get(0).ok().and_then(|axis| axis.cast::<CategoryAxis>()) else {
        return;
    };
    let orientation = root.get_control::<ComboBox>("BarOrientationComboBox");
    let sort_key = root.get_control::<ComboBox>("SortKeyComboBox");
    let sort_order = root.get_control::<ComboBox>("SortOrderComboBox");
    // BarOrientationComboBox_SelectionChanged and CategorySortComboBox_SelectionChanged.
    let apply: Rc<dyn Fn()> = {
        let (series, axis) = (series.downgrade(), axis.downgrade());
        let (orientation, sort_key, sort_order) = (orientation.downgrade(), sort_key.downgrade(), sort_order.downgrade());
        Rc::new(move || {
            let (Some(series), Some(axis), Some(orientation), Some(sort_key), Some(sort_order)) =
                (series.upgrade(), axis.upgrade(), orientation.upgrade(), sort_key.upgrade(), sort_order.upgrade())
            else {
                return;
            };
            series.set_orientation(if orientation.selected_index() == 1 {
                BarOrientation::Horizontal
            } else {
                BarOrientation::Vertical
            });
            axis.set_sort_key(if sort_key.selected_index() == 1 { CategorySortKey::Value } else { CategorySortKey::Index });
            axis.set_sort_order(if sort_order.selected_index() == 1 { SortOrder::Descending } else { SortOrder::Ascending });
        })
    };
    for combo in [&orientation, &sort_key, &sort_order] {
        on_selection(combo, &apply);
    }
}

// --- Example 3: AxisOptions --------------------------------------------------------------------------

fn wire_axis_options(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("AxisOptionsChart");
    fill_strings(&chart, 0, &QUARTERS);
    fill_numbers(&chart, 1, &QUARTERLY_VISITORS);
    connect(&chart, 0, Some((0, 0)), (1, 1));
    let axes: Vec<WeakRef<CartesianAxis>> =
        chart.axes().snapshot().iter().filter_map(|axis| axis.clone().cast::<CartesianAxis>()).map(|axis| axis.downgrade()).collect();
    let Some(visitors) = axes.get(1).cloned() else { return };
    // GridLinesComboBox_SelectionChanged.
    let grid_lines = root.get_control::<ComboBox>("GridLinesComboBox");
    {
        let weak = grid_lines.downgrade();
        grid_lines.property_changed(move |change| {
            if !is_selection_change(change) {
                return;
            }
            if let (Some(axis), Some(grid_lines)) = (visitors.upgrade(), weak.upgrade()) {
                axis.set_grid_lines(match grid_lines.selected_index() {
                    0 => GridLines::None,
                    2 => GridLines::Minor,
                    _ => GridLines::Major,
                });
            }
        });
    }
    // AxisOptionToggle_Toggled: both axes follow the toggles.
    let axes_visible = root.get_control::<ToggleSwitch>("AxesVisibleToggle");
    let tick_labels = root.get_control::<ToggleSwitch>("TickLabelsToggle");
    let tick_marks = root.get_control::<ToggleSwitch>("TickMarksToggle");
    let apply: Rc<dyn Fn()> = {
        let (axes_visible, tick_labels, tick_marks) = (axes_visible.downgrade(), tick_labels.downgrade(), tick_marks.downgrade());
        Rc::new(move || {
            for axis in axes.iter().filter_map(WeakRef::upgrade) {
                axis.set_is_visible(is_on(&axes_visible));
                axis.set_show_tick_labels(is_on(&tick_labels));
                axis.set_show_tick_marks(is_on(&tick_marks));
            }
        })
    };
    for toggle in [&axes_visible, &tick_labels, &tick_marks] {
        on_checked(toggle, &apply);
    }
    apply();
}

// --- Example 4: StyledSeries ---------------------------------------------------------------------------

fn wire_styled_series(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("StyledSeriesChart");
    fill_strings(&chart, 0, &MONTHS);
    fill_numbers(&chart, 1, &ACTUALS);
    fill_numbers(&chart, 2, &FORECASTS);
    fill_numbers(&chart, 3, &TARGETS);
    let forecast = connect(&chart, 0, Some((0, 0)), (1, 2));
    let actual = connect(&chart, 1, Some((0, 0)), (1, 1));
    let target = connect(&chart, 2, Some((0, 0)), (1, 3));
    let (Some(forecast), Some(actual), Some(target)) = (forecast, actual, target) else {
        return;
    };
    // SeriesVisibilityCheckBox_Click.
    let checks = [
        root.get_control::<CheckBox>("ForecastCheckBox"),
        root.get_control::<CheckBox>("ActualCheckBox"),
        root.get_control::<CheckBox>("TargetCheckBox"),
    ];
    let apply: Rc<dyn Fn()> = {
        let series = [forecast.downgrade(), actual.downgrade(), target.downgrade()];
        let checks = [checks[0].downgrade(), checks[1].downgrade(), checks[2].downgrade()];
        Rc::new(move || {
            for (series, check) in series.iter().zip(&checks) {
                if let Some(series) = series.upgrade() {
                    series.set_is_visible(is_on(check));
                }
            }
        })
    };
    for check in &checks {
        on_checked(check, &apply);
    }
}

// --- Example 5: LinesAndMarkers ------------------------------------------------------------------------

/// `HighlightPeakVisits`: the label and the marker of the highest point.
pub(crate) fn peak_index() -> u32 {
    let mut peak = 0;
    for (index, value) in DAILY_VISITS.iter().enumerate() {
        if *value > DAILY_VISITS[peak] {
            peak = index;
        }
    }
    peak as u32
}

fn wire_lines_and_markers(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("LinesAndMarkersChart");
    fill_strings(&chart, 0, &DAYS);
    fill_numbers(&chart, 1, &DAILY_VISITS);
    let Some(series) = connect(&chart, 0, Some((0, 0)), (1, 1)) else {
        return;
    };
    let peak = peak_index();
    series.data_label_overrides().add(peak, DataLabelOverride::new("Peak", series.data_label_brush()));
    series.data_marker_overrides().add(peak, DataMarkerOverride::new(MarkerShape::Diamond, series.data_marker_brush()));

    let dash_style = root.get_control::<ComboBox>("DashStyleComboBox");
    let thickness = root.get_control::<Slider>("StrokeThicknessSlider");
    let shape = root.get_control::<ComboBox>("MarkerShapeComboBox");
    let markers = root.get_control::<ToggleSwitch>("DataMarkersToggle");
    let data_labels = root.get_control::<ToggleSwitch>("DataLabelsToggle");
    // DashStyleComboBox_SelectionChanged, MarkerShapeComboBox_SelectionChanged and
    // StrokeThicknessSlider_ValueChanged.
    let apply_stroke: Rc<dyn Fn()> = {
        let series = series.downgrade();
        let (dash_style, thickness, shape) = (dash_style.downgrade(), thickness.downgrade(), shape.downgrade());
        Rc::new(move || {
            let (Some(series), Some(dash_style), Some(thickness), Some(shape)) =
                (series.upgrade(), dash_style.upgrade(), thickness.upgrade(), shape.upgrade())
            else {
                return;
            };
            series.set_stroke_thickness(thickness.value());
            if let Some(style) = usize::try_from(dash_style.selected_index()).ok().and_then(|index| DASH_STYLES.get(index)) {
                series.set_stroke_dash_style(*style);
            }
            if let Some(marker) = usize::try_from(shape.selected_index()).ok().and_then(|index| MARKER_SHAPES.get(index)) {
                series.set_marker_shape(*marker);
            }
        })
    };
    on_selection(&dash_style, &apply_stroke);
    on_selection(&shape, &apply_stroke);
    {
        let apply = apply_stroke.clone();
        thickness.property_changed(move |change| {
            if change.property() == RangeBase::value_property().as_property() {
                apply();
            }
        });
    }
    // DataPointToggle_Toggled.
    let apply_points: Rc<dyn Fn()> = {
        let series = series.downgrade();
        let (markers, data_labels) = (markers.downgrade(), data_labels.downgrade());
        Rc::new(move || {
            if let Some(series) = series.upgrade() {
                series.set_show_data_markers(is_on(&markers));
                series.set_show_data_labels(is_on(&data_labels));
            }
        })
    };
    for toggle in [&markers, &data_labels] {
        on_checked(toggle, &apply_points);
    }
}

// --- Example 6: TimeSeries -------------------------------------------------------------------------------

fn date(month: i32, day: i32) -> DateTimeOffset {
    DateTimeOffset::new(2026, month, day, 0, 0, 0, SignedDuration::ZERO)
}

fn wire_time_series(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("TimeSeriesChart");
    let interval = root.get_control::<ComboBox>("IntervalTypeComboBox");
    let format = root.get_control::<ComboBox>("LabelFormatComboBox");
    // CreateTimeSeriesChart.
    let day = Samples::from_values(READING_DATES.iter().map(|(month, day)| date(*month, *day)));
    let reading = Samples::from_values(TEMPERATURE_READINGS);
    let day_x = DateTimeAxis::new();
    day_x.set_label("Date");
    let _ = day_x.set_minimum(Some(date(7, 27)));
    let _ = day_x.set_maximum(Some(date(9, 7)));
    let temperature_y = LinearAxis::new();
    temperature_y.set_label("Temperature (\u{b0}F)");
    temperature_y.set_grid_lines(GridLines::Major);
    let series = AreaSeries::new();
    series.set_title("Temperature");
    let added = chart.data().add(day.clone()).and_then(|()| chart.data().add(reading.clone()));
    let added = added
        .and_then(|()| chart.axes().add(day_x.clone().upcast::<Axis>()))
        .and_then(|()| chart.axes().add(temperature_y.clone().upcast::<Axis>()))
        .and_then(|()| chart.series().add(series.clone().upcast::<CartesianSeries>()))
        .and_then(|()| series.set_x_axis(Some(day_x.clone().upcast())))
        .and_then(|()| series.set_y_axis(Some(temperature_y.upcast())));
    if added.is_err() {
        return;
    }
    series.set_x_values(Some(day));
    series.set_y_values(Some(reading));
    // DateTimeAxisOption_SelectionChanged.
    let apply: Rc<dyn Fn()> = {
        let axis = day_x.downgrade();
        let (interval, format) = (interval.downgrade(), format.downgrade());
        Rc::new(move || {
            let (Some(axis), Some(interval), Some(format)) = (axis.upgrade(), interval.upgrade(), format.upgrade()) else {
                return;
            };
            if let Some(value) = usize::try_from(interval.selected_index()).ok().and_then(|index| INTERVALS.get(index)) {
                axis.set_interval_type(*value);
            }
            if let Some(value) = usize::try_from(format.selected_index()).ok().and_then(|index| LABEL_FORMATS.get(index)) {
                axis.set_label_format(*value);
            }
        })
    };
    for combo in [&interval, &format] {
        on_selection(combo, &apply);
    }
    // IntervalType = Week and LabelFormat = "month day", the initial options.
    apply();
}

// --- Example 7: LiveObservableSeries -----------------------------------------------------------------------

/// `AddResponseTimeSampleButton_Click`: the eight most recent values, the new one last.
pub(crate) fn next_response_times(current: &[f64], next: f64) -> Vec<f64> {
    let keep = current.len().min(7);
    let mut values = current[current.len() - keep..].to_vec();
    values.push(next);
    values
}

/// `UpdateLiveSeriesSummary`.
pub(crate) fn live_series_summary(values: &[f64]) -> String {
    let latest = values.last().copied().unwrap_or_default();
    format!("Latest: {latest:.0} ms ({} samples)", values.len())
}

/// The state of the live series: the source's `ResponseTimes` and `_nextResponseTimeIndex`.
struct LiveSeries {
    samples: WeakRef<Samples>,
    summary: WeakRef<TextBlock>,
    values: RefCell<Vec<f64>>,
    next: Cell<usize>,
}

impl LiveSeries {
    fn update(&self) {
        let values = self.values.borrow();
        // `Samples.ItemsSource` reads its values when it is set, so a new source replaces it.
        if let Some(samples) = self.samples.upgrade() {
            samples.set_source(Some(ItemsSource::from_values(values.iter().copied())));
        }
        if let Some(summary) = self.summary.upgrade() {
            summary.set_text(Some(&live_series_summary(&values)));
        }
    }

    fn add_sample(&self) {
        let next = INCOMING_RESPONSE_TIMES[self.next.get()];
        self.next.set((self.next.get() + 1) % INCOMING_RESPONSE_TIMES.len());
        let values = next_response_times(&self.values.borrow(), next);
        *self.values.borrow_mut() = values;
        self.update();
    }
}

fn wire_live_series(root: &Ref<Control>) {
    let chart = root.get_control::<Chart>("LiveSeriesChart");
    let Ok(samples) = chart.data().get(0) else {
        return;
    };
    let live = Rc::new(LiveSeries {
        samples: samples.downgrade(),
        summary: root.get_control::<TextBlock>("LiveSeriesSummary").downgrade(),
        values: RefCell::new(INITIAL_RESPONSE_TIMES.to_vec()),
        next: Cell::new(0),
    });
    live.update();
    connect(&chart, 0, None, (0, 0));
    let button = root.get_control::<Button>("AddResponseTimeSampleButton");
    button.click(move |_, _| live.add_sample());
}
