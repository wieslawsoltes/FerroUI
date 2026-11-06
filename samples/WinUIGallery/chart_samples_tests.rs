//! The Chart page: the seven source examples on the native CartesianChart, with the data of the
//! source page, driven through their options as a user does. No WinUI visual-parity claim.

use crate::chart_samples::{
    self, ACTUALS, DAILY_VISITS, DAYS, FORECASTS, MONTHS, PROFITS, QUARTERLY_VISITORS, QUARTERS,
    REGIONS, TARGETS, UNITS_SOLD,
};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::Color;
use ferroui_base::Ref;
use ferroui_controls::{Button, CheckBox, ComboBox, Control, Grid, Slider, TextBlock, ToggleSwitch};
use ferroui_controls_livecharts::{Axis, CartesianChart, ChartSeries, LegendPosition, SeriesKind};

fn page() -> Ref<Control> {
    examples::create(examples::find("Chart").unwrap()).unwrap()
}

fn chart(root: &Ref<Control>, name: &str) -> Ref<CartesianChart> {
    root.get_control::<CartesianChart>(name)
}

fn series(chart: &CartesianChart, index: usize) -> Ref<ChartSeries> {
    chart.series().get(index)
}

fn x_axis(chart: &CartesianChart) -> Ref<Axis> {
    chart.x_axes().get(0)
}

fn y_axis(chart: &CartesianChart) -> Ref<Axis> {
    chart.y_axes().get(0)
}

fn ys(series: &ChartSeries) -> Vec<f64> {
    series.values().to_vec().iter().map(|point| point.y()).collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn chart_page_loads_seven_examples_with_the_source_data() {
    let _application = start();
    let root = page();
    for index in 1..=7 {
        assert!(root
            .find_control::<Grid>(&format!("ChartExample{index}"))
            .is_some());
    }
    let basic = chart(&root, "BasicLineChart");
    assert_eq!(basic.legend_position(), LegendPosition::Top);
    assert_eq!(x_axis(&basic).labels(), strings(&MONTHS));
    assert_eq!(y_axis(&basic).min_limit(), Some(0.0));
    assert_eq!(series(&basic, 0).name(), "2025");
    assert_eq!(ys(&series(&basic, 0)), PROFITS.to_vec());

    let bar = chart(&root, "BarChart");
    assert_eq!(series(&bar, 0).kind(), SeriesKind::Column);
    assert_eq!(series(&bar, 0).fill(), Some(Color::from_rgb(0x00, 0x78, 0xD4)));
    assert_eq!(x_axis(&bar).labels(), strings(&REGIONS));
    assert_eq!(ys(&series(&bar, 0)), UNITS_SOLD.to_vec());

    let axes = chart(&root, "AxisOptionsChart");
    assert_eq!(x_axis(&axes).labels(), strings(&QUARTERS));
    assert_eq!(ys(&series(&axes, 0)), QUARTERLY_VISITORS.to_vec());
    let visitors = y_axis(&axes);
    assert_eq!((visitors.min_limit(), visitors.max_limit()), (Some(0.0), Some(100.0)));
    assert_eq!(visitors.min_step(), 20.0);
    assert!(visitors.force_step());

    let styled = chart(&root, "StyledSeriesChart");
    assert_eq!(styled.series().count(), 3);
    assert_eq!(series(&styled, 0).kind(), SeriesKind::Area);
    assert_eq!(series(&styled, 1).kind(), SeriesKind::Column);
    assert_eq!(series(&styled, 2).kind(), SeriesKind::Line);
    assert_eq!(ys(&series(&styled, 0)), FORECASTS.to_vec());
    assert_eq!(ys(&series(&styled, 1)), ACTUALS.to_vec());
    assert_eq!(ys(&series(&styled, 2)), TARGETS.to_vec());
    assert!(series(&styled, 1).data_labels());
    // StrokeDashStyle="Dash" at a thickness of 2.
    assert_eq!(series(&styled, 2).dashes(), vec![4.0, 4.0]);

    let markers = chart(&root, "LinesAndMarkersChart");
    assert_eq!(x_axis(&markers).labels(), strings(&DAYS));
    assert_eq!(ys(&series(&markers, 0)), DAILY_VISITS.to_vec());
    assert!(series(&markers, 0).data_labels());
    assert_eq!(series(&markers, 0).geometry_size(), 8.0);

    // CreateTimeSeriesChart: the axes and the series are created by the page.
    let time = chart(&root, "TimeSeriesChart");
    assert_eq!(time.series().count(), 1);
    assert_eq!(series(&time, 0).kind(), SeriesKind::Area);
    let time_axis = x_axis(&time);
    assert_eq!(time_axis.name(), "Date");
    assert_eq!((time_axis.min_limit(), time_axis.max_limit()), (Some(0.0), Some(42.0)));
    assert_eq!(time_axis.labels()[7], "August 3");
    assert_eq!(
        time_axis.custom_separators(),
        vec![0.0, 7.0, 14.0, 21.0, 28.0, 35.0, 42.0]
    );
    let readings: Vec<(f64, f64)> = series(&time, 0)
        .values()
        .to_vec()
        .iter()
        .map(|point| (point.x(), point.y()))
        .collect();
    assert_eq!(
        readings,
        vec![(7.0, 72.0), (14.0, 76.0), (21.0, 74.0), (28.0, 79.0), (35.0, 77.0)]
    );

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
    assert_eq!(basic.legend_position(), LegendPosition::Hidden);
    toggle.set_is_checked(Some(true));
    assert_eq!(basic.legend_position(), LegendPosition::Top);
}

#[test]
fn chart_bar_orientation_and_category_sort_reorder_the_series() {
    assert_eq!(chart_samples::bar_order(false, false), vec![0, 1, 2, 3]);
    assert_eq!(chart_samples::bar_order(false, true), vec![3, 2, 1, 0]);
    assert_eq!(chart_samples::bar_order(true, false), vec![1, 0, 3, 2]);
    assert_eq!(chart_samples::bar_order(true, true), vec![2, 3, 0, 1]);

    let _application = start();
    let root = page();
    let bar = chart(&root, "BarChart");
    root.get_control::<ComboBox>("BarOrientationComboBox").set_selected_index(1);
    assert_eq!(series(&bar, 0).kind(), SeriesKind::Row);
    // The categories move to the vertical axis.
    assert_eq!(y_axis(&bar).labels(), strings(&REGIONS));
    assert_eq!(y_axis(&bar).name(), "Region");
    assert!(x_axis(&bar).labels().is_empty());
    assert_eq!(x_axis(&bar).min_limit(), Some(0.0));

    root.get_control::<ComboBox>("SortKeyComboBox").set_selected_index(1);
    assert_eq!(y_axis(&bar).labels(), strings(&["South", "North", "West", "East"]));
    assert_eq!(ys(&series(&bar, 0)), vec![38.0, 42.0, 47.0, 51.0]);
    root.get_control::<ComboBox>("SortOrderComboBox").set_selected_index(1);
    assert_eq!(y_axis(&bar).labels(), strings(&["East", "West", "North", "South"]));
    assert_eq!(ys(&series(&bar, 0)), vec![51.0, 47.0, 42.0, 38.0]);

    root.get_control::<ComboBox>("BarOrientationComboBox").set_selected_index(0);
    assert_eq!(series(&bar, 0).kind(), SeriesKind::Column);
    assert_eq!(x_axis(&bar).labels(), strings(&["East", "West", "North", "South"]));
    assert!(y_axis(&bar).labels().is_empty());
}

#[test]
fn chart_axis_options_change_grid_lines_visibility_and_tick_labels() {
    let _application = start();
    let root = page();
    let axes = chart(&root, "AxisOptionsChart");
    let (x, y) = (x_axis(&axes), y_axis(&axes));
    let grid_lines = root.get_control::<ComboBox>("GridLinesComboBox");
    grid_lines.set_selected_index(0);
    assert!(!y.show_separator_lines());
    grid_lines.set_selected_index(2);
    assert!(y.show_separator_lines());
    assert_eq!(y.custom_separators().len(), 11);
    grid_lines.set_selected_index(1);
    assert!(y.show_separator_lines());
    assert!(y.custom_separators().is_empty());

    root.get_control::<ToggleSwitch>("AxesVisibleToggle").set_is_checked(Some(false));
    assert!(!x.visible() && !y.visible());
    root.get_control::<ToggleSwitch>("AxesVisibleToggle").set_is_checked(Some(true));
    assert!(x.visible() && y.visible());

    root.get_control::<ToggleSwitch>("TickLabelsToggle").set_is_checked(Some(false));
    assert_eq!(x.labels_color().a, 0);
    assert_eq!(y.labels_color().a, 0);
    root.get_control::<ToggleSwitch>("TickLabelsToggle").set_is_checked(Some(true));
    assert_eq!(y.labels_color(), Color::from_rgb(0x76, 0x76, 0x76));
}

#[test]
fn chart_styled_series_check_boxes_hide_and_show_each_series() {
    let _application = start();
    let root = page();
    let styled = chart(&root, "StyledSeriesChart");
    for (index, name) in ["ForecastCheckBox", "ActualCheckBox", "TargetCheckBox"]
        .into_iter()
        .enumerate()
    {
        let check = root.get_control::<CheckBox>(name);
        check.set_is_checked(Some(false));
        for other in 0..3 {
            assert_eq!(series(&styled, other).visible(), other != index, "{name}");
        }
        check.set_is_checked(Some(true));
        assert!(series(&styled, index).visible());
    }
}

#[test]
fn chart_lines_and_markers_options_change_dash_thickness_markers_and_labels() {
    assert!(chart_samples::dash_pattern(0, 2.0).is_empty());
    assert_eq!(chart_samples::dash_pattern(3, 3.0), vec![6.0, 6.0, 3.0, 6.0]);

    let _application = start();
    let root = page();
    let visits = series(&chart(&root, "LinesAndMarkersChart"), 0);
    root.get_control::<ComboBox>("DashStyleComboBox").set_selected_index(1);
    assert_eq!(visits.dashes(), vec![4.0, 4.0]);
    // The dash array follows the thickness.
    root.get_control::<Slider>("StrokeThicknessSlider").set_range_value(5.0);
    assert_eq!(visits.stroke_width(), 5.0);
    assert_eq!(visits.dashes(), vec![10.0, 10.0]);
    root.get_control::<ComboBox>("DashStyleComboBox").set_selected_index(2);
    assert_eq!(visits.dashes(), vec![5.0, 10.0]);
    root.get_control::<ComboBox>("DashStyleComboBox").set_selected_index(0);
    assert!(visits.dashes().is_empty());

    root.get_control::<ToggleSwitch>("DataMarkersToggle").set_is_checked(Some(false));
    assert_eq!(visits.geometry_size(), 0.0);
    assert!(visits.data_labels());
    root.get_control::<ToggleSwitch>("DataLabelsToggle").set_is_checked(Some(false));
    assert!(!visits.data_labels());
    root.get_control::<ToggleSwitch>("DataMarkersToggle").set_is_checked(Some(true));
    assert_eq!(visits.geometry_size(), 8.0);
}

#[test]
fn chart_time_axis_follows_the_interval_and_the_label_format() {
    assert_eq!(chart_samples::time_axis_label(0, 0), "July 27");
    assert_eq!(chart_samples::time_axis_label(5, 1), "Aug 1");
    assert_eq!(chart_samples::time_axis_label(42, 2), "9/7/2026");
    assert_eq!(chart_samples::time_axis_separators(3), vec![5.0, 36.0]);
    assert_eq!(chart_samples::time_axis_separators(1).len(), 43);
    assert!(chart_samples::time_axis_separators(0).is_empty());

    let _application = start();
    let root = page();
    let axis = x_axis(&chart(&root, "TimeSeriesChart"));
    root.get_control::<ComboBox>("IntervalTypeComboBox").set_selected_index(3);
    assert_eq!(axis.custom_separators(), vec![5.0, 36.0]);
    root.get_control::<ComboBox>("LabelFormatComboBox").set_selected_index(1);
    assert_eq!(axis.labels()[36], "Sep 1");
    root.get_control::<ComboBox>("LabelFormatComboBox").set_selected_index(2);
    assert_eq!(axis.labels()[7], "8/3/2026");
    root.get_control::<ComboBox>("IntervalTypeComboBox").set_selected_index(0);
    assert!(axis.custom_separators().is_empty());
}

#[test]
fn chart_live_series_adds_samples_and_keeps_the_eight_most_recent() {
    assert_eq!(
        chart_samples::next_response_times(&[1.0, 2.0], 3.0),
        vec![1.0, 2.0, 3.0]
    );
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
    assert_eq!(
        ys(&series(&live, 0)),
        vec![184.0, 179.0, 173.0, 171.0, 168.0, 176.0]
    );
    assert_eq!(summary.text().as_deref(), Some("Latest: 176 ms (6 samples)"));
    click();
    click();
    assert_eq!(series(&live, 0).values().count(), 8);
    click();
    assert_eq!(
        ys(&series(&live, 0)),
        vec![179.0, 173.0, 171.0, 168.0, 176.0, 169.0, 181.0, 165.0]
    );
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
