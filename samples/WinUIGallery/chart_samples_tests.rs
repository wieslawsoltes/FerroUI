//! The Chart page: the seven source examples on the native charts of `ferroui-controls-charts`,
//! with the data of the source page, driven through their options as a user does. No WinUI
//! visual-parity claim.

use crate::chart_samples::{
    self, ACTUALS, DAILY_VISITS, DAYS, FORECASTS, MONTHS, PROFITS, QUARTERLY_VISITORS, QUARTERS, REGIONS, TARGETS,
    TEMPERATURE_READINGS, UNITS_SOLD,
};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Ref, Size};
use ferroui_controls::{Button, CheckBox, ComboBox, Control, Grid, Slider, TextBlock, ToggleSwitch};
use ferroui_controls_charts::{
    AreaSeries, BarOrientation, BarSeries, CartesianAxis, CartesianSeries, CategoryAxis, CategorySortKey, Chart,
    DateTimeAxis, DateTimeIntervalType, GridLines, LineSeries, LinearAxis, MarkerShape, SampleValue, SortOrder,
    StrokeDashStyle,
};

const SIZE: Size = Size::new(640.0, 320.0);

fn page() -> Ref<Control> {
    examples::create(examples::find("Chart").unwrap()).unwrap()
}

fn chart(root: &Ref<Control>, name: &str) -> Ref<Chart> {
    root.get_control::<Chart>(name)
}

fn series(chart: &Chart, index: usize) -> Ref<CartesianSeries> {
    chart.series().get(index).unwrap()
}

fn axis<T: ferroui_base::ObjectType>(chart: &Chart, index: usize) -> Ref<T> {
    chart.axes().get(index).unwrap().cast::<T>().unwrap()
}

fn ys(series: &CartesianSeries) -> Vec<f64> {
    let samples = series.y_values().unwrap();
    samples
        .snapshot()
        .iter()
        .map(|value| match value {
            Some(SampleValue::Number(number)) => *number,
            other => panic!("not a number: {other:?}"),
        })
        .collect()
}

fn xs(series: &CartesianSeries) -> Vec<String> {
    let samples = series.x_values().unwrap();
    samples
        .snapshot()
        .iter()
        .map(|value| match value {
            Some(SampleValue::Category(text)) => text.clone(),
            other => panic!("not a text: {other:?}"),
        })
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn chart_page_loads_seven_examples_with_the_source_data() {
    let _application = start();
    let root = page();
    for index in 1..=7 {
        assert!(root.find_control::<Grid>(&format!("ChartExample{index}")).is_some());
    }
    let basic = chart(&root, "BasicLineChart");
    assert!(basic.show_legend());
    assert_eq!(basic.legend_title(), "Profit");
    assert_eq!(xs(&series(&basic, 0)), strings(&MONTHS));
    assert_eq!(axis::<LinearAxis>(&basic, 1).minimum(), Some(0.0));
    assert_eq!(series(&basic, 0).title(), "2025");
    assert_eq!(ys(&series(&basic, 0)), PROFITS.to_vec());
    assert!(series(&basic, 0).cast::<LineSeries>().is_some());

    let bar = chart(&root, "BarChart");
    assert_eq!(series(&bar, 0).cast::<BarSeries>().unwrap().orientation(), BarOrientation::Vertical);
    assert_eq!(xs(&series(&bar, 0)), strings(&REGIONS));
    assert_eq!(ys(&series(&bar, 0)), UNITS_SOLD.to_vec());

    let axes = chart(&root, "AxisOptionsChart");
    assert_eq!(xs(&series(&axes, 0)), strings(&QUARTERS));
    assert_eq!(ys(&series(&axes, 0)), QUARTERLY_VISITORS.to_vec());
    let visitors = axis::<LinearAxis>(&axes, 1);
    assert_eq!((visitors.minimum(), visitors.maximum()), (Some(0.0), Some(100.0)));
    assert_eq!(visitors.spacing(), Some(20.0));
    assert!(visitors.axis_line_brush().is_some() && visitors.tick_brush().is_some());
    assert!(visitors.grid_line_major_brush().is_some() && visitors.grid_line_minor_brush().is_some());

    let styled = chart(&root, "StyledSeriesChart");
    assert_eq!(styled.legend_title(), "Monthly results");
    assert_eq!(styled.series().count(), 3);
    assert!(series(&styled, 0).cast::<AreaSeries>().is_some());
    assert!(series(&styled, 1).cast::<BarSeries>().is_some());
    assert!(series(&styled, 2).cast::<LineSeries>().is_some());
    assert_eq!(ys(&series(&styled, 0)), FORECASTS.to_vec());
    assert_eq!(ys(&series(&styled, 1)), ACTUALS.to_vec());
    assert_eq!(ys(&series(&styled, 2)), TARGETS.to_vec());
    assert!(series(&styled, 1).show_data_labels());
    assert_eq!(series(&styled, 2).stroke_dash_style(), StrokeDashStyle::Dash);
    assert_eq!(series(&styled, 2).marker_shape(), MarkerShape::Diamond);
    assert!(series(&styled, 2).data_marker_brush().is_some());

    let markers = chart(&root, "LinesAndMarkersChart");
    assert_eq!(xs(&series(&markers, 0)), strings(&DAYS));
    assert_eq!(ys(&series(&markers, 0)), DAILY_VISITS.to_vec());
    assert!(series(&markers, 0).show_data_labels() && series(&markers, 0).show_data_markers());
    // HighlightPeakVisits: Thursday, with its own label and marker.
    assert_eq!(chart_samples::peak_index(), 3);
    assert!(series(&markers, 0).data_marker_overrides().contains_key(&3));
    let labels = markers.layout_snapshot(SIZE).data_labels;
    assert!(labels.contains(&(0, 3, "Peak".to_owned())), "{labels:?}");

    // CreateTimeSeriesChart: the axes and the series are created by the page.
    let time = chart(&root, "TimeSeriesChart");
    assert_eq!(time.series().count(), 1);
    assert!(series(&time, 0).cast::<AreaSeries>().is_some());
    let time_axis = axis::<DateTimeAxis>(&time, 0);
    assert_eq!(time_axis.label(), "Date");
    assert_eq!(time_axis.interval_type(), DateTimeIntervalType::Week);
    assert_eq!(time_axis.label_format(), "month day");
    assert_eq!(ys(&series(&time, 0)), TEMPERATURE_READINGS.to_vec());

    assert_eq!(
        root.get_control::<TextBlock>("LiveSeriesSummary").text().as_deref(),
        Some("Latest: 168 ms (5 samples)")
    );
}

#[test]
fn chart_legend_toggle_hides_and_shows_the_legend() {
    let _application = start();
    let root = page();
    let basic = chart(&root, "BasicLineChart");
    let toggle = root.get_control::<ToggleSwitch>("LegendToggle");
    toggle.set_is_checked(Some(false));
    assert!(!basic.show_legend());
    toggle.set_is_checked(Some(true));
    assert!(basic.show_legend());
}

#[test]
fn chart_bar_orientation_and_category_sort_reorder_the_series() {
    let _application = start();
    let root = page();
    let bar = chart(&root, "BarChart");
    let bars = series(&bar, 0).cast::<BarSeries>().unwrap();
    let categories = axis::<CategoryAxis>(&bar, 0);
    let order = || categories.sorted_indices_by(4, |a, b| UNITS_SOLD[a].total_cmp(&UNITS_SOLD[b]));
    assert_eq!(order(), vec![0, 1, 2, 3]);

    root.get_control::<ComboBox>("BarOrientationComboBox").set_selected_index(1);
    assert_eq!(bars.orientation(), BarOrientation::Horizontal);

    root.get_control::<ComboBox>("SortKeyComboBox").set_selected_index(1);
    assert_eq!(categories.sort_key(), CategorySortKey::Value);
    assert_eq!(order(), vec![1, 0, 3, 2]);
    root.get_control::<ComboBox>("SortOrderComboBox").set_selected_index(1);
    assert_eq!(categories.sort_order(), SortOrder::Descending);
    assert_eq!(order(), vec![2, 3, 0, 1]);

    root.get_control::<ComboBox>("BarOrientationComboBox").set_selected_index(0);
    assert_eq!(bars.orientation(), BarOrientation::Vertical);
}

#[test]
fn chart_axis_options_change_grid_lines_visibility_and_tick_labels() {
    let _application = start();
    let root = page();
    let axes = chart(&root, "AxisOptionsChart");
    let (x, y) = (axis::<CartesianAxis>(&axes, 0), axis::<CartesianAxis>(&axes, 1));
    let grid_lines = root.get_control::<ComboBox>("GridLinesComboBox");
    grid_lines.set_selected_index(0);
    assert_eq!(y.grid_lines(), GridLines::None);
    grid_lines.set_selected_index(2);
    assert_eq!(y.grid_lines(), GridLines::Minor);
    grid_lines.set_selected_index(1);
    assert_eq!(y.grid_lines(), GridLines::Major);

    let visible = root.get_control::<ToggleSwitch>("AxesVisibleToggle");
    visible.set_is_checked(Some(false));
    assert!(!x.is_visible() && !y.is_visible());
    visible.set_is_checked(Some(true));
    assert!(x.is_visible() && y.is_visible());

    let labels = root.get_control::<ToggleSwitch>("TickLabelsToggle");
    labels.set_is_checked(Some(false));
    assert!(!x.show_tick_labels() && !y.show_tick_labels());
    labels.set_is_checked(Some(true));
    assert!(x.show_tick_labels() && y.show_tick_labels());

    let marks = root.get_control::<ToggleSwitch>("TickMarksToggle");
    assert!(x.show_tick_marks() && y.show_tick_marks());
    marks.set_is_checked(Some(false));
    assert!(!x.show_tick_marks() && !y.show_tick_marks());
}

#[test]
fn chart_styled_series_check_boxes_hide_and_show_each_series() {
    let _application = start();
    let root = page();
    let styled = chart(&root, "StyledSeriesChart");
    for (index, name) in ["ForecastCheckBox", "ActualCheckBox", "TargetCheckBox"].into_iter().enumerate() {
        let check = root.get_control::<CheckBox>(name);
        check.set_is_checked(Some(false));
        for other in 0..3 {
            assert_eq!(series(&styled, other).is_visible(), other != index, "{name}");
        }
        check.set_is_checked(Some(true));
        assert!(series(&styled, index).is_visible());
    }
}

#[test]
fn chart_lines_and_markers_options_change_dash_thickness_markers_and_labels() {
    let _application = start();
    let root = page();
    let visits = series(&chart(&root, "LinesAndMarkersChart"), 0);
    let dash = root.get_control::<ComboBox>("DashStyleComboBox");
    dash.set_selected_index(1);
    assert_eq!(visits.stroke_dash_style(), StrokeDashStyle::Dash);
    root.get_control::<Slider>("StrokeThicknessSlider").set_range_value(5.0);
    assert_eq!(visits.stroke_thickness(), 5.0);
    dash.set_selected_index(2);
    assert_eq!(visits.stroke_dash_style(), StrokeDashStyle::Dot);
    dash.set_selected_index(0);
    assert_eq!(visits.stroke_dash_style(), StrokeDashStyle::Solid);

    root.get_control::<ComboBox>("MarkerShapeComboBox").set_selected_index(3);
    assert_eq!(visits.marker_shape(), MarkerShape::Triangle);

    root.get_control::<ToggleSwitch>("DataMarkersToggle").set_is_checked(Some(false));
    assert!(!visits.show_data_markers());
    assert!(visits.show_data_labels());
    root.get_control::<ToggleSwitch>("DataLabelsToggle").set_is_checked(Some(false));
    assert!(!visits.show_data_labels());
    root.get_control::<ToggleSwitch>("DataMarkersToggle").set_is_checked(Some(true));
    assert!(visits.show_data_markers());
}

#[test]
fn chart_time_axis_follows_the_interval_and_the_label_format() {
    let _application = start();
    let root = page();
    let time = axis::<DateTimeAxis>(&chart(&root, "TimeSeriesChart"), 0);
    root.get_control::<ComboBox>("IntervalTypeComboBox").set_selected_index(3);
    assert_eq!(time.interval_type(), DateTimeIntervalType::Month);
    root.get_control::<ComboBox>("LabelFormatComboBox").set_selected_index(1);
    assert_eq!(time.label_format(), "month.abbreviated day");
    root.get_control::<ComboBox>("LabelFormatComboBox").set_selected_index(2);
    assert_eq!(time.label_format(), "shortdate");
    root.get_control::<ComboBox>("IntervalTypeComboBox").set_selected_index(0);
    assert_eq!(time.interval_type(), DateTimeIntervalType::Auto);
}

#[test]
fn chart_live_series_adds_samples_and_keeps_the_eight_most_recent() {
    assert_eq!(chart_samples::next_response_times(&[1.0, 2.0], 3.0), vec![1.0, 2.0, 3.0]);
    assert_eq!(chart_samples::live_series_summary(&[184.0, 176.4]), "Latest: 176 ms (2 samples)");

    let _application = start();
    let root = page();
    let live = chart(&root, "LiveSeriesChart");
    let summary = root.get_control::<TextBlock>("LiveSeriesSummary");
    let add = root.get_control::<Button>("AddResponseTimeSampleButton");
    let click = || add.raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    let revision = live.revision();
    click();
    assert!(live.revision() > revision);
    assert_eq!(ys(&series(&live, 0)), vec![184.0, 179.0, 173.0, 171.0, 168.0, 176.0]);
    assert_eq!(summary.text().as_deref(), Some("Latest: 176 ms (6 samples)"));
    click();
    click();
    assert_eq!(ys(&series(&live, 0)).len(), 8);
    click();
    assert_eq!(ys(&series(&live, 0)), vec![179.0, 173.0, 171.0, 168.0, 176.0, 169.0, 181.0, 165.0]);
    click();
    // The incoming values wrap around.
    click();
    assert_eq!(ys(&series(&live, 0)).last().copied(), Some(176.0));
    assert_eq!(summary.text().as_deref(), Some("Latest: 176 ms (8 samples)"));
}

#[test]
fn chart_page_releases_its_charts_with_the_page() {
    let _application = start();
    let root = page();
    let weak = chart(&root, "LiveSeriesChart").downgrade();
    let weak_root = root.downgrade();
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert!(weak.upgrade().is_none());
}
