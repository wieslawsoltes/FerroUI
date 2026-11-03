//! The data and the handlers of WinUI Gallery's `ChartPage.xaml.cs` (14061eea; see NOTICE.md) in
//! Rust, on the native CartesianChart of `ferroui-controls-livecharts`. The charts own layout,
//! scaling, drawing and input; the sample owns the data of the page and its options.
//!
//! The source binds its arrays with `x:Bind` (`Months`, `Profits` and the others) into
//! `charts:Samples`; the port sets the values of the series and the labels of the category axes
//! from the same arrays. The `DateTimeAxis` of the time series is an axis over the days of the
//! range, labelled per day, whose separators are the ticks of the interval type.

use ferroui_base::media::Color;
use ferroui_base::{ObjectType, Ref, WeakRef};
use ferroui_controls::primitives::{RangeBase, SelectingItemsControl, ToggleButton};
use ferroui_controls::{Button, CheckBox, ComboBox, Control, Slider, TextBlock, ToggleSwitch};
use ferroui_controls_livecharts::{
    AreaSeries, Axis, CartesianChart, ChartPoint, ChartPoints, ChartSeries, LegendPosition,
    SeriesKind,
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
/// `ReadingDates` (2026-08-03 to 2026-08-31, weekly) as days after the axis minimum, 2026-07-27.
pub(crate) const READING_DAYS: [f64; 5] = [7.0, 14.0, 21.0, 28.0, 35.0];
pub(crate) const TEMPERATURE_READINGS: [f64; 5] = [72.0, 76.0, 74.0, 79.0, 77.0];
pub(crate) const INITIAL_RESPONSE_TIMES: [f64; 5] = [184.0, 179.0, 173.0, 171.0, 168.0];
pub(crate) const INCOMING_RESPONSE_TIMES: [f64; 5] = [176.0, 169.0, 181.0, 165.0, 172.0];

/// The days of the time axis: its minimum 2026-07-27 is day 0, its maximum 2026-09-07 day 42.
pub(crate) const TIME_AXIS_DAYS: usize = 42;

const AXIS_TICK_COLOR: Color = Color::from_rgb(0x76, 0x76, 0x76);
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

pub(crate) fn wire(root: &Ref<Control>) {
    let Some(basic) = find::<CartesianChart>(root, "BasicLineChart") else {
        return;
    };
    wire_basic_line(root, &basic);
    wire_bar(root);
    wire_axis_options(root);
    wire_styled_series(root);
    wire_lines_and_markers(root);
    wire_time_series(root);
    wire_live_series(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name)
        .and_then(|control| control.cast())
}

fn labels(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn is_checked_change(change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == ToggleButton::is_checked_property().as_property()
}

fn is_selection_change(change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) -> bool {
    change.property() == SelectingItemsControl::selected_index_property().as_property()
}

fn is_on(toggle: &WeakRef<ToggleSwitch>) -> bool {
    toggle
        .upgrade()
        .is_some_and(|toggle| toggle.is_checked() == Some(true))
}

fn is_checked(check: &WeakRef<CheckBox>) -> bool {
    check
        .upgrade()
        .is_some_and(|check| check.is_checked() == Some(true))
}

/// The series and the first axes of a chart of the page.
struct ChartParts {
    series: Vec<Ref<ChartSeries>>,
    x: Ref<Axis>,
    y: Ref<Axis>,
}

fn parts(chart: &CartesianChart) -> Option<ChartParts> {
    Some(ChartParts {
        series: chart.series().to_vec(),
        x: chart.x_axes().try_get(0)?,
        y: chart.y_axes().try_get(0)?,
    })
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

// --- Example 1: BasicLineChart --------------------------------------------------------------------

fn wire_basic_line(root: &Ref<Control>, chart: &Ref<CartesianChart>) {
    let Some(parts) = parts(chart) else {
        return;
    };
    parts.x.set_labels(labels(&MONTHS));
    if let Some(series) = parts.series.first() {
        series.set_numeric_values(PROFITS);
    }
    // LegendToggle_Toggled.
    let toggle = root.get_control::<ToggleSwitch>("LegendToggle");
    let weak_toggle = toggle.downgrade();
    let weak_chart = chart.downgrade();
    toggle.property_changed(move |change| {
        if !is_checked_change(change) {
            return;
        }
        if let Some(chart) = weak_chart.upgrade() {
            chart.set_legend_position(if is_on(&weak_toggle) {
                LegendPosition::Top
            } else {
                LegendPosition::Hidden
            });
        }
    });
}

// --- Example 2: BarChart ---------------------------------------------------------------------------

/// The order of the regions for a sort key (`Index` or `Value`) and a sort order.
pub(crate) fn bar_order(by_value: bool, descending: bool) -> Vec<usize> {
    let mut order: Vec<usize> = (0..REGIONS.len()).collect();
    if by_value {
        order.sort_by(|a, b| UNITS_SOLD[*a].total_cmp(&UNITS_SOLD[*b]));
    }
    if descending {
        order.reverse();
    }
    order
}

fn wire_bar(root: &Ref<Control>) {
    let chart = root.get_control::<CartesianChart>("BarChart");
    let orientation = root.get_control::<ComboBox>("BarOrientationComboBox");
    let sort_key = root.get_control::<ComboBox>("SortKeyComboBox");
    let sort_order = root.get_control::<ComboBox>("SortOrderComboBox");
    // BarOrientationComboBox_SelectionChanged and CategorySortComboBox_SelectionChanged: the
    // orientation is the kind of the series (a horizontal bar is a row), whose categories are on
    // the vertical axis; the sort of the category axis orders the points and their labels.
    let apply: Rc<dyn Fn()> = {
        let chart = chart.downgrade();
        let (orientation, sort_key, sort_order) =
            (orientation.downgrade(), sort_key.downgrade(), sort_order.downgrade());
        Rc::new(move || {
            let (Some(chart), Some(orientation), Some(sort_key), Some(sort_order)) =
                (chart.upgrade(), orientation.upgrade(), sort_key.upgrade(), sort_order.upgrade())
            else {
                return;
            };
            let Some(parts) = parts(&chart) else {
                return;
            };
            let Some(series) = parts.series.first() else {
                return;
            };
            let order = bar_order(sort_key.selected_index() == 1, sort_order.selected_index() == 1);
            let names: Vec<String> = order.iter().map(|index| REGIONS[*index].to_owned()).collect();
            series.set_numeric_values(order.iter().map(|index| UNITS_SOLD[*index]));
            let (category, value) = if orientation.selected_index() == 1 {
                series.set_kind(SeriesKind::Row);
                (&parts.y, &parts.x)
            } else {
                series.set_kind(SeriesKind::Column);
                (&parts.x, &parts.y)
            };
            category.set_name("Region".to_owned());
            category.set_labels(names);
            category.set_min_limit(None);
            value.set_name("Units sold".to_owned());
            value.set_labels(Vec::new());
            value.set_min_limit(Some(0.0));
        })
    };
    for combo in [&orientation, &sort_key, &sort_order] {
        on_selection(combo, &apply);
    }
    apply();
}

// --- Example 3: AxisOptions --------------------------------------------------------------------------

/// The separators of the visitors axis for a `GridLines` option: none, the major interval (the
/// forced step of 20, `Spacing`), or the minor interval of 10 as well.
fn apply_grid_lines(axis: &Axis, index: i32) {
    match index {
        0 => {
            axis.set_show_separator_lines(false);
            axis.set_custom_separators(Vec::new());
        }
        1 => {
            axis.set_show_separator_lines(true);
            axis.set_custom_separators(Vec::new());
        }
        2 => {
            axis.set_show_separator_lines(true);
            axis.set_custom_separators((0..=10).map(|step| f64::from(step) * 10.0).collect());
        }
        _ => {}
    }
}

fn wire_axis_options(root: &Ref<Control>) {
    let chart = root.get_control::<CartesianChart>("AxisOptionsChart");
    let Some(parts) = parts(&chart) else {
        return;
    };
    parts.x.set_labels(labels(&QUARTERS));
    if let Some(series) = parts.series.first() {
        series.set_numeric_values(QUARTERLY_VISITORS);
    }
    // GridLinesComboBox_SelectionChanged.
    let grid_lines = root.get_control::<ComboBox>("GridLinesComboBox");
    {
        let weak_axis = parts.y.downgrade();
        let weak_grid_lines = grid_lines.downgrade();
        grid_lines.property_changed(move |change| {
            if !is_selection_change(change) {
                return;
            }
            if let (Some(axis), Some(grid_lines)) = (weak_axis.upgrade(), weak_grid_lines.upgrade()) {
                apply_grid_lines(&axis, grid_lines.selected_index());
            }
        });
    }
    // AxisOptionToggle_Toggled: both axes follow the toggles. The axes have no tick marks of their
    // own; tick labels are hidden with a transparent label colour.
    let axes_visible = root.get_control::<ToggleSwitch>("AxesVisibleToggle");
    let tick_labels = root.get_control::<ToggleSwitch>("TickLabelsToggle");
    let apply: Rc<dyn Fn()> = {
        let axes = [parts.x.downgrade(), parts.y.downgrade()];
        let (axes_visible, tick_labels) = (axes_visible.downgrade(), tick_labels.downgrade());
        Rc::new(move || {
            let visible = is_on(&axes_visible);
            let labels_color = if is_on(&tick_labels) {
                AXIS_TICK_COLOR
            } else {
                Color::from_argb(0, 0x76, 0x76, 0x76)
            };
            for axis in axes.iter().filter_map(WeakRef::upgrade) {
                axis.set_visible(visible);
                axis.set_labels_color(labels_color);
            }
        })
    };
    for toggle in [&axes_visible, &tick_labels] {
        on_checked(toggle, &apply);
    }
}

// --- Example 4: StyledSeries ---------------------------------------------------------------------------

fn wire_styled_series(root: &Ref<Control>) {
    let chart = root.get_control::<CartesianChart>("StyledSeriesChart");
    let Some(parts) = parts(&chart) else {
        return;
    };
    parts.x.set_labels(labels(&MONTHS));
    let [forecast, actual, target] = parts.series.as_slice() else {
        return;
    };
    forecast.set_numeric_values(FORECASTS);
    actual.set_numeric_values(ACTUALS);
    target.set_numeric_values(TARGETS);
    // StrokeDashStyle="Dash".
    target.set_dashes(dash_pattern(1, target.stroke_width()));
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
                    series.set_visible(is_checked(check));
                }
            }
        })
    };
    for check in &checks {
        on_checked(check, &apply);
    }
}

// --- Example 5: LinesAndMarkers ------------------------------------------------------------------------

/// The dash array of a `StrokeDashStyle` (Solid, Dash, Dot, DashDot, DashDotDot), in multiples of
/// the stroke thickness as XAML dash styles are, converted to the pixels the chart takes. A dot is
/// one thickness long: the caps of the chart's strokes are flat.
pub(crate) fn dash_pattern(index: i32, thickness: f64) -> Vec<f64> {
    let units: &[f64] = match index {
        1 => &[2.0, 2.0],
        2 => &[1.0, 2.0],
        3 => &[2.0, 2.0, 1.0, 2.0],
        4 => &[2.0, 2.0, 1.0, 2.0, 1.0, 2.0],
        _ => &[],
    };
    units.iter().map(|unit| unit * thickness).collect()
}

/// The size of the markers of the line series while they are shown.
const MARKER_SIZE: f64 = 8.0;

fn wire_lines_and_markers(root: &Ref<Control>) {
    let chart = root.get_control::<CartesianChart>("LinesAndMarkersChart");
    let Some(parts) = parts(&chart) else {
        return;
    };
    parts.x.set_labels(labels(&DAYS));
    let Some(series) = parts.series.first() else {
        return;
    };
    series.set_numeric_values(DAILY_VISITS);
    let dash_style = root.get_control::<ComboBox>("DashStyleComboBox");
    let thickness = root.get_control::<Slider>("StrokeThicknessSlider");
    let markers = root.get_control::<ToggleSwitch>("DataMarkersToggle");
    let data_labels = root.get_control::<ToggleSwitch>("DataLabelsToggle");
    // DashStyleComboBox_SelectionChanged and StrokeThicknessSlider_ValueChanged: the dash array
    // is in pixels, so it follows the thickness.
    let apply_stroke: Rc<dyn Fn()> = {
        let series = series.downgrade();
        let (dash_style, thickness) = (dash_style.downgrade(), thickness.downgrade());
        Rc::new(move || {
            let (Some(series), Some(dash_style), Some(thickness)) =
                (series.upgrade(), dash_style.upgrade(), thickness.upgrade())
            else {
                return;
            };
            series.set_stroke_width(thickness.value());
            series.set_dashes(dash_pattern(dash_style.selected_index(), thickness.value()));
        })
    };
    on_selection(&dash_style, &apply_stroke);
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
                series.set_geometry_size(if is_on(&markers) { MARKER_SIZE } else { 0.0 });
                series.set_data_labels(is_on(&data_labels));
            }
        })
    };
    for toggle in [&markers, &data_labels] {
        on_checked(toggle, &apply_points);
    }
}

// --- Example 6: TimeSeries -------------------------------------------------------------------------------

/// The month and the day of a day of the time axis (2026-07-27 is day 0).
pub(crate) fn time_axis_date(day: usize) -> (usize, usize) {
    match day {
        0..=4 => (7, 27 + day),
        5..=35 => (8, day - 4),
        _ => (9, day - 35),
    }
}

/// The label of a day of the time axis in a `LabelFormat` of the source (`month day`,
/// `month.abbreviated day`, `shortdate`, en-US).
pub(crate) fn time_axis_label(day: usize, format: i32) -> String {
    let (month, date) = time_axis_date(day);
    let name = MONTH_NAMES[month - 1];
    match format {
        1 => format!("{} {date}", &name[..3]),
        2 => format!("{month}/{date}/2026"),
        _ => format!("{name} {date}"),
    }
}

/// The ticks of a `DateTimeIntervalType` (Auto, Day, Week, Month): none of its own (the axis
/// chooses), every day, every week from the Monday 2026-07-27, or the first day of each month.
pub(crate) fn time_axis_separators(interval: i32) -> Vec<f64> {
    match interval {
        1 => (0..=TIME_AXIS_DAYS).map(|day| day as f64).collect(),
        2 => (0..=TIME_AXIS_DAYS).step_by(7).map(|day| day as f64).collect(),
        3 => (0..=TIME_AXIS_DAYS)
            .filter(|day| time_axis_date(*day).1 == 1)
            .map(|day| day as f64)
            .collect(),
        _ => Vec::new(),
    }
}

fn wire_time_series(root: &Ref<Control>) {
    let chart = root.get_control::<CartesianChart>("TimeSeriesChart");
    let interval = root.get_control::<ComboBox>("IntervalTypeComboBox");
    let format = root.get_control::<ComboBox>("LabelFormatComboBox");
    // CreateTimeSeriesChart.
    let time_axis = Axis::new();
    time_axis.set_name("Date".to_owned());
    time_axis.set_min_limit(Some(0.0));
    time_axis.set_max_limit(Some(TIME_AXIS_DAYS as f64));
    let temperature = Axis::new();
    temperature.set_name("Temperature (\u{b0}F)".to_owned());
    temperature.set_show_separator_lines(true);
    let series = AreaSeries::new();
    series.set_name("Temperature".to_owned());
    series.set_line_smoothness(0.0);
    series.set_geometry_size(0.0);
    series.set_values(ChartPoints::from_items(
        READING_DAYS
            .iter()
            .zip(TEMPERATURE_READINGS)
            .map(|(day, reading)| ChartPoint::with_xy(*day, reading)),
    ));
    chart.x_axes().add(time_axis.clone());
    chart.y_axes().add(temperature);
    chart.series().add(series.upcast());
    // DateTimeAxisOption_SelectionChanged.
    let apply: Rc<dyn Fn()> = {
        let time_axis = time_axis.downgrade();
        let (interval, format) = (interval.downgrade(), format.downgrade());
        Rc::new(move || {
            let (Some(axis), Some(interval), Some(format)) =
                (time_axis.upgrade(), interval.upgrade(), format.upgrade())
            else {
                return;
            };
            axis.set_custom_separators(time_axis_separators(interval.selected_index()));
            axis.set_labels(
                (0..=TIME_AXIS_DAYS)
                    .map(|day| time_axis_label(day, format.selected_index()))
                    .collect(),
            );
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
    series: WeakRef<ChartSeries>,
    summary: WeakRef<TextBlock>,
    values: RefCell<Vec<f64>>,
    next: Cell<usize>,
}

impl LiveSeries {
    fn update(&self) {
        let values = self.values.borrow();
        if let Some(series) = self.series.upgrade() {
            series.set_numeric_values(values.iter().copied());
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
    let chart = root.get_control::<CartesianChart>("LiveSeriesChart");
    let Some(series) = chart.series().try_get(0) else {
        return;
    };
    let live = Rc::new(LiveSeries {
        series: series.downgrade(),
        summary: root.get_control::<TextBlock>("LiveSeriesSummary").downgrade(),
        values: RefCell::new(INITIAL_RESPONSE_TIMES.to_vec()),
        next: Cell::new(0),
    });
    live.update();
    let button = root.get_control::<Button>("AddResponseTimeSampleButton");
    button.click(move |_, _| live.add_sample());
}
