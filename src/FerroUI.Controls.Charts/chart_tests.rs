//! Chart collection, property, lifecycle and automation contracts.
use crate::*;
use ferroui_base::animation::TimeSpan;
use ferroui_base::collections::{
    FerroList, NotifyCollectionChangedAction as Action, NotifyCollectionChangedEventArgs,
};
use ferroui_base::controls::{IResourceProvider, ResourceDictionary, ResourceKey};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{
    Colors, DrawingContext, FontFamily, IBrush, PlatformDrawingContext, SolidColorBrush,
};
use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{Ref, Size};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Control, ItemsSource, TextBlock, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn line() -> Ref<LineSeries> {
    let series = LineSeries::new();
    series.set_x_values(Some(Samples::from_values(["Jan", "Feb", "Mar"])));
    series.set_y_values(Some(Samples::from_values([1.0, 2.0, 3.0])));
    series
}

fn populated_chart() -> Ref<Chart> {
    let chart = Chart::new();
    chart.set_width(400.0);
    chart.set_height(300.0);
    chart.series().add(line()).unwrap();
    chart
}

fn show(chart: &Chart) -> Ref<Window> {
    let window = Window::new();
    window.set_content(Some(Control::boxed(chart.to_ref())));
    window.show();
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    window
}

type Event = (Action, i32, i32);

fn record<T>(sink: &Rc<RefCell<Vec<Event>>>, change: &NotifyCollectionChangedEventArgs<'_, T>) {
    sink.borrow_mut().push((
        change.action,
        change.new_starting_index,
        change.old_starting_index,
    ));
}

fn date(year: i32, month: i32, day: i32) -> DateTimeOffset {
    DateTimeOffset::new(year, month, day, 0, 0, 0, TimeSpan::ZERO)
}

fn chart_with_axes(
    series: Ref<LineSeries>,
    x: Option<Ref<Axis>>,
    y: Option<Ref<Axis>>,
) -> Ref<Chart> {
    let chart = Chart::new();
    chart.set_width(400.0);
    chart.set_height(300.0);
    if let Some(x) = x {
        chart.axes().add(x.clone()).unwrap();
        series.set_x_axis(Some(x)).unwrap();
    }
    if let Some(y) = y {
        chart.axes().add(y.clone()).unwrap();
        series.set_y_axis(Some(y)).unwrap();
    }
    chart.series().add(series).unwrap();
    chart
}

const AXIS_BRUSH_KEYS: [&str; 5] = [
    "ChartsGridLineMajorBrush",
    "ChartsGridLineMinorBrush",
    "ChartsTickBrush",
    "ChartsTickLabelBrush",
    "ChartsAxisLineBrush",
];

fn axis_brush_resources() -> Ref<ResourceDictionary> {
    let resources = ResourceDictionary::new();
    for key in AXIS_BRUSH_KEYS {
        let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
        resources.add_value(key, brush);
    }
    resources
}

fn assert_axis_brushes_unset(axis: &CartesianAxis) {
    assert!(axis.grid_line_major_brush().is_none());
    assert!(axis.grid_line_minor_brush().is_none());
    assert!(axis.tick_brush().is_none());
    assert!(axis.tick_label_brush().is_none());
    assert!(axis.axis_line_brush().is_none());
}

fn assert_chart_name_in_automation_tree(window: &Window, expected: &str) {
    let mut remaining = vec![ControlAutomationPeer::create_peer_for_element(window)];
    while let Some(peer) = remaining.pop() {
        if peer.get_name() == expected {
            assert_eq!("Chart", peer.get_class_name());
            assert!(peer.is_control_element());
            return;
        }
        remaining.extend(peer.get_children().iter().cloned());
    }
    panic!("Chart named {expected:?} was not in the window automation tree");
}

fn draw_commands(chart: &Chart) -> Vec<String> {
    let log = DrawingLog::new();
    let mut platform = MockDrawingContextImpl::new(log.clone());
    {
        let mut core = PlatformDrawingContext::borrowed(&mut platform);
        let mut context = DrawingContext::new(&mut core);
        chart.render(&mut context);
    }
    log.entries()
}

#[test]
fn chart_defaults_expose_empty_collections_and_no_legend() {
    let chart = Chart::new();
    assert!(!chart.show_legend());
    assert!(chart.legend_title().is_empty());
    assert_eq!(0, chart.series().count());
    assert_eq!(0, chart.axes().count());
    assert_eq!(0, chart.data().count());
    assert!(chart.series() == chart.series());
    assert!(chart.axes() == chart.axes());
    assert!(chart.data() == chart.data());
    let other = Chart::new();
    assert!(chart.series() != other.series());
    assert!(chart.axes() != other.axes());
    assert!(chart.data() != other.data());
}

#[test]
fn chart_measure_uses_available_minimums_and_fallback() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    chart.measure(Size::new(80.0, 48.0));
    assert_eq!(Size::new(80.0, 48.0), chart.desired_size());
    chart.measure(Size::default());
    assert_eq!(Size::default(), chart.desired_size());
    chart.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(Size::new(160.0, 96.0), chart.desired_size());
    chart.set_min_width(200.0);
    chart.set_min_height(120.0);
    chart.measure(Size::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(Size::new(200.0, 120.0), chart.desired_size());
}

#[test]
fn chart_can_load_unload_and_reload() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let loaded = Rc::new(Cell::new(0));
    let unloaded = Rc::new(Cell::new(0));
    let count = loaded.clone();
    chart.loaded(move |_, _| count.set(count.get() + 1));
    let count = unloaded.clone();
    chart.unloaded(move |_, _| count.set(count.get() + 1));
    let window = show(&chart);
    assert!(chart.is_loaded());
    window.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!chart.is_loaded());
    window.set_content(Some(Control::boxed(&chart)));
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(chart.is_loaded());
    assert_eq!(2, loaded.get());
    assert_eq!(1, unloaded.get());
    assert_eq!(1, chart.series().count());
    window.close();
}

#[test]
fn chart_resize_while_loaded_updates_actual_size() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let window = show(&chart);
    chart.set_width(640.0);
    chart.set_height(360.0);
    chart.update_layout();
    assert!(chart.is_loaded());
    assert_eq!(Size::new(640.0, 360.0), chart.bounds().size());
    window.close();
}

#[test]
fn parent_requested_theme_can_change_while_chart_is_loaded() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let window = show(&chart);
    for theme in [ThemeVariant::dark(), ThemeVariant::light()] {
        window.set_requested_theme_variant(Some(theme.clone()));
        chart.update_layout();
        assert!(chart.is_loaded());
        assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
        assert_eq!(Some(theme), chart.actual_theme_variant());
    }
    window.close();
}

#[test]
fn legend_properties_round_trip_before_and_after_load() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    chart.set_show_legend(true);
    chart.set_legend_title("Quarterly revenue");
    let window = show(&chart);
    assert!(chart.is_loaded());
    assert!(chart.show_legend());
    assert_eq!("Quarterly revenue", chart.legend_title());
    for title in ["Annual revenue", ""] {
        chart.set_legend_title(title);
        chart.update_layout();
        assert_eq!(title, chart.legend_title());
        assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
    }
    chart.set_show_legend(false);
    chart.set_legend_title("Hidden title");
    chart.update_layout();
    assert!(!chart.show_legend());
    assert_eq!("Hidden title", chart.legend_title());
    window.close();
}

#[test]
fn chart_legend_dependency_properties_update_and_clear_values() {
    let chart = Chart::new();
    chart.set_value(Chart::show_legend_property(), true);
    chart.set_value(Chart::legend_title_property(), "Revenue".to_owned());
    assert!(chart.show_legend());
    assert_eq!("Revenue", chart.legend_title());
    assert!(chart.get_value(Chart::show_legend_property()));
    assert_eq!("Revenue", chart.get_value(Chart::legend_title_property()));
    chart.clear_value(Chart::show_legend_property());
    chart.clear_value(Chart::legend_title_property());
    assert!(!chart.show_legend());
    assert!(chart.legend_title().is_empty());
}

#[test]
fn chart_formatting_properties_round_trip_before_and_after_load() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let red: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let blue: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE));
    chart.set_background(Some(red.clone()));
    chart.set_foreground(Some(blue.clone()));
    chart.set_font_family(FontFamily::new("Consolas"));
    chart.set_font_size(18.0);
    let window = show(&chart);
    assert!(Rc::ptr_eq(&red, &chart.background().unwrap()));
    assert!(Rc::ptr_eq(&blue, &chart.foreground().unwrap()));
    assert_eq!(FontFamily::new("Consolas"), chart.font_family());
    assert_eq!(18.0, chart.font_size());
    chart.set_background(Some(blue.clone()));
    chart.set_foreground(Some(red.clone()));
    chart.set_font_family(FontFamily::new("Segoe UI"));
    chart.set_font_size(20.0);
    chart.update_layout();
    assert!(chart.is_loaded());
    assert!(Rc::ptr_eq(&blue, &chart.background().unwrap()));
    assert!(Rc::ptr_eq(&red, &chart.foreground().unwrap()));
    assert_eq!(FontFamily::new("Segoe UI"), chart.font_family());
    assert_eq!(20.0, chart.font_size());
    assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
    window.close();
}

#[test]
fn line_series_without_explicit_axes_keeps_axis_properties_null_after_load() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let window = show(&chart);
    let series = chart.series().get(0).unwrap();
    let _layout = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert!(chart.is_loaded());
    assert!(series.x_axis().is_none());
    assert!(series.y_axis().is_none());
    assert_eq!(0, chart.axes().count());
    window.close();
}

#[test]
fn series_collection_supports_indexing_copy_and_enumeration() {
    let chart = Chart::new();
    let first: Ref<CartesianSeries> = line().upcast();
    let second: Ref<CartesianSeries> = AreaSeries::new().upcast();
    chart.series().add(first.clone()).unwrap();
    chart.series().add(second.clone()).unwrap();
    assert_eq!(2, chart.series().count());
    assert!(chart.series().get(0).unwrap() == first);
    assert!(chart.series().get(1).unwrap() == second);
    assert!(chart.series().contains(first.clone()));
    assert_eq!(Some(1), chart.series().index_of(second.clone()));
    let copied = chart.series().snapshot().as_ref().clone();
    assert!(copied == vec![first.clone(), second.clone()]);
    let enumerated: Vec<_> = chart.series().snapshot().iter().cloned().collect();
    assert!(enumerated == vec![first, second]);
}

#[test]
fn series_collection_mutation_preserves_existing_snapshot() {
    // The Rust adaptation uses FerroList snapshots instead of fail-fast CLR iterators.
    let chart = Chart::new();
    let first: Ref<CartesianSeries> = line().upcast();
    chart.series().add(first.clone()).unwrap();
    let snapshot = chart.series().snapshot();
    chart.series().add(AreaSeries::new()).unwrap();
    assert_eq!(1, snapshot.len());
    assert!(snapshot[0] == first);
    assert_eq!(2, chart.series().count());
}

#[test]
fn series_collection_raises_exact_vector_events() {
    let chart = Chart::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    chart
        .series()
        .add_collection_changed(Rc::new(move |change| record(&sink, change)));
    let first = line();
    let second = AreaSeries::new();
    let third = BarSeries::new();
    chart.series().add(first).unwrap();
    chart.series().insert(0, second).unwrap();
    chart.series().set(1, third.clone()).unwrap();
    chart.series().set(1, third).unwrap();
    chart.series().remove_at(0).unwrap();
    chart.series().clear().unwrap();
    chart.series().clear().unwrap();
    assert_eq!(
        vec![
            (Action::Add, 0, -1),
            (Action::Add, 0, -1),
            (Action::Replace, 1, 1),
            (Action::Remove, -1, 0),
            (Action::Reset, -1, -1)
        ],
        *events.borrow()
    );
}

#[test]
fn series_collection_rejects_out_of_range_and_null() {
    // Ref<T> excludes null statically; runtime index errors remain fallible.
    let chart = Chart::new();
    assert!(chart.series().get(0).is_err());
    assert!(chart.series().set(0, line()).is_err());
    assert!(chart.series().insert(1, line()).is_err());
    assert!(chart.series().remove_at(0).is_err());
    assert_eq!(0, chart.series().count());
}

#[test]
fn series_can_be_added_again_after_removal() {
    let first = Chart::new();
    let second = Chart::new();
    let series = line();
    first.series().add(series.clone()).unwrap();
    first.series().remove_at(0).unwrap();
    second.series().add(series.clone()).unwrap();
    assert_eq!(0, first.series().count());
    assert_eq!(1, second.series().count());
    assert!(second.series().get(0).unwrap() == series);
}

#[test]
fn failed_series_mutations_do_not_change_collection_or_raise_events() {
    let chart = Chart::new();
    let other = Chart::new();
    let original = line();
    chart.series().add(original.clone()).unwrap();
    let events = Rc::new(Cell::new(0));
    let count = events.clone();
    chart
        .series()
        .add_collection_changed(Rc::new(move |_| count.set(count.get() + 1)));
    assert!(chart.series().add(original.clone()).is_err());
    assert!(chart.series().insert(0, original.clone()).is_err());
    assert!(other.series().add(original.clone()).is_err());
    chart.series().set(0, original.clone()).unwrap();
    assert_eq!(0, events.get());
    assert_eq!(1, chart.series().count());
    assert_eq!(0, other.series().count());
    assert!(chart.series().get(0).unwrap() == original);
}

#[test]
fn axis_collection_supports_add_remove_and_vector_events() {
    let chart = Chart::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    chart
        .axes()
        .add_collection_changed(Rc::new(move |change| record(&sink, change)));
    let first = LinearAxis::new();
    let second = CategoryAxis::new();
    chart.axes().add(first.clone()).unwrap();
    chart.axes().insert(0, second.clone()).unwrap();
    assert_eq!(2, chart.axes().count());
    assert!(chart.axes().get(0).unwrap() == second);
    assert!(chart.axes().get(1).unwrap() == first);
    chart.axes().remove_at(1).unwrap();
    chart.axes().clear().unwrap();
    assert_eq!(0, chart.axes().count());
    assert_eq!(
        vec![
            (Action::Add, 0, -1),
            (Action::Add, 0, -1),
            (Action::Remove, -1, 1),
            (Action::Reset, -1, -1)
        ],
        *events.borrow()
    );
}

#[test]
fn axis_collection_rejects_duplicate_cross_chart_null_and_out_of_range() {
    let first = Chart::new();
    let second = Chart::new();
    let axis = LinearAxis::new();
    first.axes().add(axis.clone()).unwrap();
    assert!(first.axes().add(axis.clone()).is_err());
    assert!(second.axes().add(axis).is_err());
    // A nullable axis cannot be passed to this typed collection.
    assert!(first.axes().set(1, LinearAxis::new()).is_err());
    assert!(first.axes().insert(3, LinearAxis::new()).is_err());
    assert!(second.axes().remove_at(0).is_err());
    assert_eq!(1, first.axes().count());
    assert_eq!(0, second.axes().count());
}

#[test]
fn explicit_axes_can_be_assigned_to_series_in_same_chart() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let x: Ref<Axis> = CategoryAxis::new().upcast();
    let y: Ref<Axis> = LinearAxis::new().upcast();
    chart.axes().add(x.clone()).unwrap();
    chart.axes().add(y.clone()).unwrap();
    let series = line();
    series.set_x_axis(Some(x.clone())).unwrap();
    series.set_y_axis(Some(y.clone())).unwrap();
    chart.series().add(series.clone()).unwrap();
    let window = show(&chart);
    assert!(series.x_axis() == Some(x));
    assert!(series.y_axis() == Some(y));
    assert_eq!(2, chart.axes().count());
    window.close();
}

#[test]
fn series_rejects_axis_not_present_in_same_chart() {
    let chart = Chart::new();
    let series = line();
    series.set_y_axis(Some(LinearAxis::new().upcast())).unwrap();
    assert!(chart.series().add(series).is_err());
    assert_eq!(0, chart.series().count());
}

#[test]
fn series_rejects_axis_from_another_chart() {
    let first = Chart::new();
    let second = Chart::new();
    let axis: Ref<Axis> = LinearAxis::new().upcast();
    first.axes().add(axis.clone()).unwrap();
    let series = line();
    second.series().add(series.clone()).unwrap();
    assert!(series.set_y_axis(Some(axis)).is_err());
    assert!(series.y_axis().is_none());
}

#[test]
fn data_collection_supports_basic_list_and_vector_semantics() {
    let chart = Chart::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    chart
        .data()
        .add_collection_changed(Rc::new(move |change| record(&sink, change)));
    let first = Samples::from_values([1.0, 2.0]);
    let second = Samples::from_values([3.0, 4.0]);
    let replacement = Samples::from_values([5.0, 6.0]);
    chart.data().add(first.clone()).unwrap();
    chart.data().insert(0, second.clone()).unwrap();
    assert!(chart.data().get(0).unwrap() == second);
    assert!(chart.data().get(1).unwrap() == first);
    chart.data().set(1, replacement.clone()).unwrap();
    assert!(chart.data().get(1).unwrap() == replacement);
    chart.data().remove_at(0).unwrap();
    chart.data().clear().unwrap();
    assert_eq!(
        vec![
            (Action::Add, 0, -1),
            (Action::Add, 0, -1),
            (Action::Replace, 1, 1),
            (Action::Remove, -1, 0),
            (Action::Reset, -1, -1)
        ],
        *events.borrow()
    );
}

#[test]
fn labeled_by_supplies_automation_name() {
    let chart = Chart::new();
    let label = TextBlock::new();
    label.set_text(Some("CPU history"));
    AutomationProperties::set_labeled_by(&chart, label.clone());
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    assert_eq!("CPU history", peer.get_name());
}

#[test]
fn explicit_automation_name_wins_over_labeled_by() {
    for name_first in [true, false] {
        let chart = Chart::new();
        let label = TextBlock::new();
        label.set_text(Some("CPU history"));
        if name_first {
            AutomationProperties::set_name(&chart, Some("Processor utilization"));
            AutomationProperties::set_labeled_by(&chart, label.clone());
        } else {
            AutomationProperties::set_labeled_by(&chart, label.clone());
            AutomationProperties::set_name(&chart, Some("Processor utilization"));
        }
        let peer = ControlAutomationPeer::create_peer_for_element(&chart);
        assert_eq!("Processor utilization", peer.get_name());
    }
}

#[test]
fn app_full_description_wins() {
    // WinRT FullDescription is represented by the framework's existing HelpText.
    let chart = Chart::new();
    AutomationProperties::set_name(&chart, Some("CPU history"));
    AutomationProperties::set_help_text(
        &chart,
        Some("Processor utilization over the last minute."),
    );
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    assert_eq!(
        "Processor utilization over the last minute.",
        peer.get_help_text()
    );
}

#[test]
fn peer_queries_do_not_lay_out_the_chart() {
    let chart = populated_chart();
    AutomationProperties::set_name(&chart, Some("Quarterly revenue"));
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    assert_eq!("Quarterly revenue", peer.get_name());
    assert_eq!(
        AutomationControlType::Custom,
        peer.get_automation_control_type()
    );
    assert_eq!("Chart", peer.get_class_name());
    assert_eq!("chart", peer.get_localized_control_type());
    assert_eq!(0.0, chart.bounds().width);
    assert_eq!(0.0, chart.desired_size().width);
    assert_eq!(0, chart.scene_build_count());
}

#[test]
fn dropping_chart_releases_series_and_axes_even_when_collections_are_retained() {
    let series = line();
    let axis = LinearAxis::new();
    let chart = Chart::new();
    chart.series().add(series.clone()).unwrap();
    chart.axes().add(axis.clone()).unwrap();
    let retained = chart.series();
    let weak = chart.downgrade();
    drop(chart);
    assert!(weak.upgrade().is_none());
    assert_eq!(1, retained.count());
    let replacement = Chart::new();
    replacement.series().add(series).unwrap();
    replacement.axes().add(axis).unwrap();
}

#[test]
fn collection_notifications_commit_ownership_before_reentrant_edits() {
    let first = Chart::new();
    let second = Chart::new();
    let series = line();
    let other = second.clone();
    let moved = series.clone();
    first.series().add_collection_changed(Rc::new(move |event| {
        if event.action == Action::Remove {
            other.series().add(moved.clone()).unwrap();
        }
    }));
    first.series().add(series.clone()).unwrap();
    first.series().remove_at(0).unwrap();
    assert!(first.series().is_empty());
    assert!(second.series().get(0).unwrap() == series);
}

#[test]
fn shared_samples_invalidate_every_chart_and_old_sources_disconnect() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let values = Rc::new(FerroList::from_items([1.0, 2.0, 3.0]));
    let samples = Samples::from_source(ItemsSource::new(values.clone()));
    let first = Chart::new();
    let second = Chart::new();
    for chart in [&first, &second] {
        let series = line();
        series.set_y_values(Some(samples.clone()));
        chart.series().add(series).unwrap();
    }
    let size = Size::new(400.0, 300.0);
    let before = first.layout_snapshot(size);
    assert_eq!(3, before.points.len());
    let first_revision = first.revision();
    let second_revision = second.revision();
    values.set(1, 9.0);
    assert!(first.revision() > first_revision);
    assert!(second.revision() > second_revision);
    let after = first.layout_snapshot(size);
    assert_eq!(9.0, after.points[1].value);
    assert_ne!(before.points[1].position, after.points[1].position);
    assert_eq!(after.points, second.layout_snapshot(size).points);
    samples.set_source(Some(ItemsSource::from_values([4.0, 5.0, 6.0])));
    let first_revision = first.revision();
    let second_revision = second.revision();
    values.set(1, 20.0);
    assert_eq!(first_revision, first.revision());
    assert_eq!(second_revision, second.revision());
    assert_eq!(5.0, first.layout_snapshot(size).points[1].value);
}

#[test]
fn linear_axis_properties_can_change_while_loaded() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let axis = LinearAxis::new();
    let chart = chart_with_axes(line(), None, Some(axis.clone().upcast()));
    let window = show(&chart);
    let before = chart.layout_snapshot(Size::new(400.0, 300.0));
    axis.set_minimum(Some(0.0)).unwrap();
    axis.set_maximum(Some(50.0)).unwrap();
    axis.set_spacing(Some(10.0)).unwrap();
    chart.update_layout();
    assert_eq!(
        (Some(0.0), Some(50.0), Some(10.0)),
        (axis.minimum(), axis.maximum(), axis.spacing())
    );
    assert!(chart.is_loaded());
    assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
    let after = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert_ne!(before.points[2].position, after.points[2].position);
    assert!(after.axis_labels.iter().any(|label| label == "50"));
    window.close();
}

#[test]
fn axis_visibility_can_change_while_loaded() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let axis = LinearAxis::new();
    axis.set_label("Revenue");
    let chart = chart_with_axes(line(), None, Some(axis.clone().upcast()));
    let window = show(&chart);
    let before = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert!(before.axis_labels.iter().any(|label| label == "Revenue"));
    axis.set_is_visible(false);
    chart.update_layout();
    let hidden = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert!(chart.is_loaded());
    assert!(!axis.is_visible());
    assert!(!hidden.axis_labels.iter().any(|label| label == "Revenue"));
    assert!(hidden.plot_area.width > before.plot_area.width);
    axis.set_is_visible(true);
    chart.update_layout();
    assert!(chart.is_loaded());
    assert!(axis.is_visible());
    assert_eq!(before, chart.layout_snapshot(Size::new(400.0, 300.0)));
    window.close();
}

#[test]
fn axis_resource_overrides_and_theme_changes_do_not_change_local_brush_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let resources = XamlChartsResources::new();
    for theme in [
        ThemeVariant::light(),
        ThemeVariant::dark(),
        ThemeVariant::default(),
    ] {
        for key in AXIS_BRUSH_KEYS {
            assert!(resources
                .try_get_resource(&ResourceKey::from(key), Some(&theme))
                .is_some());
        }
    }
    let axis = LinearAxis::new();
    axis.set_show_tick_marks(true);
    axis.set_grid_lines(GridLines::Major);
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE));
    axis.set_tick_label_brush(Some(brush.clone()));
    let chart = chart_with_axes(line(), None, Some(axis.clone().upcast()));
    chart
        .resources()
        .merged_dictionaries()
        .add(axis_brush_resources().into());
    let window = show(&chart);
    for theme in [ThemeVariant::light(), ThemeVariant::dark()] {
        window.set_requested_theme_variant(Some(theme));
        chart.update_layout();
        assert_eq!(
            3,
            chart.layout_snapshot(Size::new(400.0, 300.0)).points.len()
        );
        assert!(Rc::ptr_eq(&brush, &axis.tick_label_brush().unwrap()));
    }
    axis.set_tick_label_brush(None);
    window.set_requested_theme_variant(None);
    chart.update_layout();
    let _ = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert_axis_brushes_unset(&axis);
    assert!(chart.is_loaded());
    window.close();
}

#[test]
fn app_level_axis_resource_overrides_keep_local_brush_properties_unset() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let app = UnitTestApplication::current().unwrap();
    let overrides = ResourceDictionary::new();
    for theme in [
        ThemeVariant::light(),
        ThemeVariant::dark(),
        ThemeVariant::default(),
    ] {
        overrides.add_theme_dictionary(theme, axis_brush_resources());
    }
    let provider: Rc<dyn IResourceProvider> = overrides.into();
    let dictionaries = app.resources().merged_dictionaries();
    let index = dictionaries.count();
    dictionaries.add(provider);
    for theme in [
        Some(ThemeVariant::light()),
        Some(ThemeVariant::dark()),
        None,
    ] {
        let axis = LinearAxis::new();
        let chart = chart_with_axes(line(), None, Some(axis.clone().upcast()));
        let window = show(&chart);
        window.set_requested_theme_variant(theme);
        window.update_layout();
        assert_eq!(
            3,
            chart.layout_snapshot(Size::new(400.0, 300.0)).points.len()
        );
        assert_axis_brushes_unset(&axis);
        assert!(chart.is_loaded());
        window.close();
    }
    dictionaries.remove_at(index);
}

#[test]
fn line_series_accepts_compatible_explicit_axes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let x = CategoryAxis::new();
    let y = LinearAxis::new();
    let series = line();
    let chart = chart_with_axes(
        series.clone(),
        Some(x.clone().upcast()),
        Some(y.clone().upcast()),
    );
    let window = show(&chart);
    assert!(series.x_axis().unwrap() == x);
    assert!(series.y_axis().unwrap() == y);
    assert_eq!(
        3,
        chart.layout_snapshot(Size::new(400.0, 300.0)).points.len()
    );
    assert!(chart.is_loaded());
    window.close();

    let x = DateTimeAxis::new();
    let series = line();
    series.set_x_values(Some(Samples::from_values([
        date(2020, 1, 1),
        date(2021, 1, 1),
        date(2022, 1, 1),
    ])));
    let chart = chart_with_axes(
        series.clone(),
        Some(x.clone().upcast()),
        Some(LinearAxis::new().upcast()),
    );
    let window = show(&chart);
    assert!(series.x_axis().unwrap() == x);
    assert_eq!(
        3,
        chart.layout_snapshot(Size::new(400.0, 300.0)).points.len()
    );
    assert!(chart.is_loaded());
    window.close();
}

#[test]
fn line_series_rejects_incompatible_explicit_axes_before_load() {
    let series = line();
    assert!(series.set_x_axis(Some(LinearAxis::new().upcast())).is_err());
    assert!(series.x_axis().is_none());
    assert!(series
        .set_y_axis(Some(CategoryAxis::new().upcast()))
        .is_err());
    assert!(series.y_axis().is_none());
    assert!(series
        .set_y_axis(Some(DateTimeAxis::new().upcast()))
        .is_err());
    assert!(series.y_axis().is_none());
}

#[test]
fn line_series_preserves_axis_after_rejected_replacement() {
    let series = line();
    let x: Ref<Axis> = CategoryAxis::new().upcast();
    let y: Ref<Axis> = LinearAxis::new().upcast();
    series.set_x_axis(Some(x.clone())).unwrap();
    series.set_y_axis(Some(y.clone())).unwrap();
    assert!(series.set_x_axis(Some(LinearAxis::new().upcast())).is_err());
    assert!(series
        .set_y_axis(Some(CategoryAxis::new().upcast()))
        .is_err());
    assert!(series.x_axis() == Some(x));
    assert!(series.y_axis() == Some(y));
}

#[test]
fn line_series_category_axis_handles_x_value_type_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let x = CategoryAxis::new();
    let y = LinearAxis::new();
    let samples = Samples::from_values(["A", "B"]);
    let series = line();
    series.set_x_values(Some(samples.clone()));
    let chart = chart_with_axes(
        series.clone(),
        Some(x.clone().upcast()),
        Some(y.clone().upcast()),
    );
    let window = show(&chart);
    for source in [
        ItemsSource::from_values([10.0, 20.0]),
        ItemsSource::from_values([date(2020, 1, 1), date(2021, 1, 1)]),
    ] {
        samples.set_source(Some(source));
        chart.update_layout();
        assert_eq!(
            2,
            chart.layout_snapshot(Size::new(400.0, 300.0)).points.len()
        );
        assert!(series.x_axis().unwrap() == x);
        assert!(series.y_axis().unwrap() == y);
        assert!(chart.is_loaded());
    }
    window.close();
}

#[test]
fn date_time_axis_loads_with_date_values() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let axis = DateTimeAxis::new();
    axis.set_minimum(Some(date(2019, 1, 1))).unwrap();
    axis.set_maximum(Some(date(2022, 1, 1))).unwrap();
    let series = line();
    series.set_x_values(Some(Samples::from_values([
        date(2020, 1, 1),
        date(2021, 1, 1),
    ])));
    let chart = chart_with_axes(
        series.clone(),
        Some(axis.clone().upcast()),
        Some(LinearAxis::new().upcast()),
    );
    let window = show(&chart);
    assert_eq!(Some(date(2019, 1, 1)), axis.minimum());
    assert_eq!(Some(date(2022, 1, 1)), axis.maximum());
    assert!(series.x_axis().unwrap() == axis);
    let layout = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert_eq!(2, layout.points.len());
    assert!(layout.points[0].position.x < layout.points[1].position.x);
    assert!(layout.points[0].position.x > layout.plot_area.x);
    assert!(layout.points[1].position.x < layout.plot_area.right());
    assert!(chart.is_loaded());
    window.close();
}

#[test]
fn date_time_axis_interval_types_change_while_loaded() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let axis = DateTimeAxis::new();
    let series = line();
    series.set_x_values(Some(Samples::from_values([
        date(2020, 1, 1),
        date(2021, 1, 1),
    ])));
    let chart = chart_with_axes(
        series,
        Some(axis.clone().upcast()),
        Some(LinearAxis::new().upcast()),
    );
    let window = show(&chart);
    for interval in [
        DateTimeIntervalType::Day,
        DateTimeIntervalType::Week,
        DateTimeIntervalType::Month,
        DateTimeIntervalType::Year,
        DateTimeIntervalType::Auto,
    ] {
        axis.set_interval_type(interval);
        chart.update_layout();
        let layout = chart.layout_snapshot(Size::new(400.0, 300.0));
        assert_eq!(interval, axis.interval_type());
        assert_eq!(2, layout.points.len());
        assert!(!layout.axis_labels.is_empty());
        assert!(chart.is_loaded());
    }
    window.close();
}

#[test]
fn chart_is_exposed_to_ui_automation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    AutomationProperties::set_name(&chart, Some("TestChart"));
    let window = show(&chart);
    assert_chart_name_in_automation_tree(&window, "TestChart");
    window.close();
}

#[test]
fn chart_keeps_working_through_data_series_legend_and_theme_changes() {
    // Execute the source interaction sequence against the native control and
    // automation tree; platform button injection belongs to backend E2E suites.
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let values = Rc::new(FerroList::from_items([1.0, 2.0, 3.0, 4.0]));
    let samples = Samples::from_source(ItemsSource::new(values.clone()));
    let chart = Chart::new();
    chart.set_width(400.0);
    chart.set_height(300.0);
    chart.set_show_legend(true);
    AutomationProperties::set_name(&chart, Some("TestChart"));
    chart.data().add(samples.clone()).unwrap();
    let series: [Ref<CartesianSeries>; 3] = [
        LineSeries::new().upcast(),
        AreaSeries::new().upcast(),
        BarSeries::new().upcast(),
    ];
    for (index, series) in series.into_iter().enumerate() {
        series.set_title(format!("Series {index}"));
        series.set_y_values(Some(samples.clone()));
        chart.series().add(series).unwrap();
    }
    let window = show(&chart);
    window.set_requested_theme_variant(Some(ThemeVariant::light()));
    let size = Size::new(400.0, 300.0);
    assert_eq!(12, chart.layout_snapshot(size).points.len());
    for value in [5.0, 6.0] {
        values.add(value);
        chart.update_layout();
        assert_eq!(values.count() * 3, chart.layout_snapshot(size).points.len());
        assert_chart_name_in_automation_tree(&window, "TestChart");
    }
    chart.set_show_legend(false);
    assert!(chart.layout_snapshot(size).legend_items.is_empty());
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    assert_eq!(Some(ThemeVariant::dark()), chart.actual_theme_variant());
    for remaining in (0..3).rev() {
        chart.series().remove_at(0).unwrap();
        chart.update_layout();
        assert_eq!(remaining, chart.series().count());
        assert_eq!(
            remaining * values.count(),
            chart.layout_snapshot(size).points.len()
        );
        assert_chart_name_in_automation_tree(&window, "TestChart");
    }
    values.add(7.0);
    assert_eq!(7, samples.count());
    assert!(chart.layout_snapshot(size).points.is_empty());
    assert!(chart.is_loaded());
    assert_chart_name_in_automation_tree(&window, "TestChart");
    window.close();
}

#[test]
fn scene_cache_survives_changes_that_do_not_affect_the_output() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let size = Size::new(400.0, 300.0);
    let first = chart.layout_snapshot(size);
    let builds = chart.scene_build_count();
    chart.set_tag(Some(Rc::new(42_i32) as ferroui_base::BoxedValue));
    chart.set_data_context(Some(Rc::new("context".to_owned()) as ferroui_base::BoxedValue));
    chart.set_name(Some("Renamed".to_owned()));
    chart.set_is_enabled(false);
    assert_eq!(first, chart.layout_snapshot(size));
    assert_eq!(builds, chart.scene_build_count());
    chart.set_show_legend(true);
    chart.layout_snapshot(size);
    assert_eq!(builds + 1, chart.scene_build_count());
    chart.set_font_size(chart.font_size() + 4.0);
    chart.layout_snapshot(size);
    assert_eq!(builds + 2, chart.scene_build_count());
}

#[test]
fn scene_cache_rebuilds_only_after_size_data_axis_or_presentation_changes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let axis = LinearAxis::new();
    let series = line();
    let chart = chart_with_axes(series.clone(), None, Some(axis.clone().upcast()));
    let size = Size::new(400.0, 300.0);
    let first = chart.layout_snapshot(size);
    let builds = chart.scene_build_count();
    assert_eq!(first, chart.layout_snapshot(size));
    assert_eq!(builds, chart.scene_build_count());
    let resized = chart.layout_snapshot(Size::new(800.0, 600.0));
    assert_eq!(builds + 1, chart.scene_build_count());
    assert_ne!(first.plot_area, resized.plot_area);
    axis.set_maximum(Some(20.0)).unwrap();
    let rescaled = chart.layout_snapshot(size);
    assert_eq!(builds + 2, chart.scene_build_count());
    assert_ne!(first.points[2].position, rescaled.points[2].position);
    series.set_show_data_labels(true);
    let labeled = chart.layout_snapshot(size);
    assert_eq!(3, labeled.data_labels.len());
    series
        .data_label_overrides()
        .add(1, DataLabelOverride::new("Peak", None));
    let overridden = chart.layout_snapshot(size);
    assert!(overridden.data_labels.contains(&(0, 1, "Peak".to_owned())));
    let builds = chart.scene_build_count();
    chart.set_font_size(chart.font_size() + 5.0);
    let _ = chart.layout_snapshot(size);
    assert_eq!(builds + 1, chart.scene_build_count());
    let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    chart.resources().add_value("ChartsTickLabelBrush", brush);
    let _ = chart.layout_snapshot(size);
    assert_eq!(builds + 2, chart.scene_build_count());
}

#[test]
fn observable_insert_replace_remove_and_reset_rebuild_source_geometry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let values = Rc::new(FerroList::from_items([1.0, 2.0, 3.0]));
    let samples = Samples::from_source(ItemsSource::new(values.clone()));
    let series = LineSeries::new();
    series.set_y_values(Some(samples));
    let chart = Chart::new();
    chart.series().add(series).unwrap();
    let size = Size::new(400.0, 300.0);
    assert_eq!(3, chart.layout_snapshot(size).points.len());
    values.insert(1, 5.0);
    let inserted = chart.layout_snapshot(size);
    assert_eq!(
        vec![1.0, 5.0, 2.0, 3.0],
        inserted
            .points
            .iter()
            .map(|point| point.value)
            .collect::<Vec<_>>()
    );
    values.set(1, -5.0);
    let replaced = chart.layout_snapshot(size);
    assert_eq!(-5.0, replaced.points[1].value);
    assert_ne!(inserted.points[1].position, replaced.points[1].position);
    values.remove_at(0);
    let removed = chart.layout_snapshot(size);
    assert_eq!(3, removed.points.len());
    assert_eq!(0, removed.points[0].sample_index);
    assert_eq!(-5.0, removed.points[0].value);
    values.clear();
    assert!(chart.layout_snapshot(size).points.is_empty());
}

#[test]
fn missing_samples_preserve_source_indices_and_category_sort_changes_geometry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let axis = CategoryAxis::new();
    axis.set_sort_key(CategorySortKey::Value);
    let series = line();
    series.set_x_values(Some(Samples::from_values(["B", "C", "A", "D"])));
    series.set_y_values(Some(Samples::from_values([
        1.0,
        f64::NAN,
        2.0,
        f64::INFINITY,
    ])));
    let chart = chart_with_axes(series, Some(axis.clone().upcast()), None);
    let layout = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert_eq!(
        vec![0, 2],
        layout
            .points
            .iter()
            .map(|point| point.sample_index)
            .collect::<Vec<_>>()
    );
    assert!(layout
        .points
        .iter()
        .all(|point| point.position.x.is_finite() && point.position.y.is_finite()));
    assert!(layout.points[1].position.x < layout.points[0].position.x);
    axis.set_sort_order(SortOrder::Descending);
    let reversed = chart.layout_snapshot(Size::new(400.0, 300.0));
    assert!(reversed.points[1].position.x > reversed.points[0].position.x);
    assert_eq!(
        vec![0, 2],
        reversed
            .points
            .iter()
            .map(|point| point.sample_index)
            .collect::<Vec<_>>()
    );
}

#[test]
fn orphan_collection_mutations_cannot_release_a_new_charts_ownership() {
    for mutation in 0..3 {
        let original = Chart::new();
        let series = line();
        let axis = LinearAxis::new();
        original.series().add(series.clone()).unwrap();
        original.axes().add(axis.clone()).unwrap();
        let orphan_series = original.series();
        let orphan_axes = original.axes();
        let weak = original.downgrade();
        drop(original);
        assert!(weak.upgrade().is_none());

        let current = Chart::new();
        current.axes().add(axis.clone()).unwrap();
        series.set_y_axis(Some(axis.clone().upcast())).unwrap();
        current.series().add(series.clone()).unwrap();
        match mutation {
            0 => {
                orphan_series.remove_at(0).unwrap();
                orphan_axes.remove_at(0).unwrap();
            }
            1 => {
                orphan_series.set(0, line()).unwrap();
                orphan_axes.set(0, LinearAxis::new()).unwrap();
            }
            _ => {
                orphan_series.clear().unwrap();
                orphan_axes.clear().unwrap();
            }
        }
        let third = Chart::new();
        assert_eq!(
            Err(ChartError::AlreadyOwned),
            third.series().add(series.clone())
        );
        assert_eq!(
            Err(ChartError::AlreadyOwned),
            third.axes().add(axis.clone())
        );
        assert!(current.series().get(0).unwrap() == series);
        assert!(current.axes().get(0).unwrap() == axis);
        assert!(series.y_axis().unwrap() == axis);
    }
}

#[test]
fn referenced_axis_removal_replacement_and_clear_fail_transactionally() {
    let chart = Chart::new();
    let y = LinearAxis::new();
    let spare = CategoryAxis::new();
    chart.axes().add(y.clone()).unwrap();
    chart.axes().add(spare.clone()).unwrap();
    let series = line();
    series.set_y_axis(Some(y.clone().upcast())).unwrap();
    chart.series().add(series).unwrap();
    let events = Rc::new(Cell::new(0));
    let sink = events.clone();
    chart
        .axes()
        .add_collection_changed(Rc::new(move |_| sink.set(sink.get() + 1)));
    let replacement = LinearAxis::new();
    assert!(matches!(
        chart.axes().remove_at(0),
        Err(ChartError::AxisInUse)
    ));
    assert_eq!(
        Err(ChartError::AxisInUse),
        chart.axes().set(0, replacement.clone())
    );
    assert_eq!(Err(ChartError::AxisInUse), chart.axes().clear());
    assert!(!chart.axes().remove(y.clone()));
    chart.axes().set(0, y.clone()).unwrap();
    assert_eq!(0, events.get());
    assert_eq!(2, chart.axes().count());
    assert!(chart.axes().get(0).unwrap() == y);
    assert!(chart.axes().get(1).unwrap() == spare);

    let other = Chart::new();
    other.axes().add(replacement).unwrap();
    assert_eq!(
        Err(ChartError::AlreadyOwned),
        other.axes().add(spare.clone())
    );
    chart.series().clear().unwrap();
    chart.axes().clear().unwrap();
    assert_eq!(1, events.get());
    other.axes().add(y).unwrap();
    other.axes().add(spare).unwrap();
}

#[test]
fn grouped_bar_rectangles_do_not_overlap_and_negative_values_start_at_zero() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    for orientation in [BarOrientation::Vertical, BarOrientation::Horizontal] {
        let chart = Chart::new();
        let x = CategoryAxis::new();
        let y = LinearAxis::new();
        y.set_minimum(Some(-10.0)).unwrap();
        y.set_maximum(Some(10.0)).unwrap();
        chart.axes().add(x.clone()).unwrap();
        chart.axes().add(y.clone()).unwrap();
        for values in [[-4.0, 6.0], [-2.0, 8.0]] {
            let series = BarSeries::new();
            series.set_orientation(orientation);
            series.set_x_values(Some(Samples::from_values(["Negative", "Positive"])));
            series.set_y_values(Some(Samples::from_values(values)));
            series.set_x_axis(Some(x.clone().upcast())).unwrap();
            series.set_y_axis(Some(y.clone().upcast())).unwrap();
            chart.series().add(series).unwrap();
        }
        let layout = chart.layout_snapshot(Size::new(600.0, 400.0));
        assert_eq!(4, layout.points.len());
        for category in 0..2 {
            let first = layout
                .points
                .iter()
                .find(|p| p.series_index == 0 && p.sample_index == category)
                .unwrap();
            let second = layout
                .points
                .iter()
                .find(|p| p.series_index == 1 && p.sample_index == category)
                .unwrap();
            let a = first.bar_bounds.unwrap();
            let b = second.bar_bounds.unwrap();
            if orientation == BarOrientation::Vertical {
                assert!(a.right() <= b.x || b.right() <= a.x);
            } else {
                assert!(a.bottom() <= b.y || b.bottom() <= a.y);
            }
        }
        let plot = layout.plot_area;
        for point in &layout.points {
            let rect = point.bar_bounds.unwrap();
            assert!(rect.width > 0.0 && rect.height > 0.0);
            assert!(rect.x >= plot.x && rect.right() <= plot.right());
            assert!(rect.y >= plot.y && rect.bottom() <= plot.bottom());
            if orientation == BarOrientation::Vertical {
                let zero = plot.y + plot.height * 0.5;
                assert!((point.position.x - (rect.x + rect.width * 0.5)).abs() < 1e-8);
                if point.value < 0.0 {
                    assert!((rect.y - zero).abs() < 1e-8);
                    assert!(rect.bottom() > zero);
                } else {
                    assert!((rect.bottom() - zero).abs() < 1e-8);
                    assert!(rect.y < zero);
                }
            } else {
                let zero = plot.x + plot.width * 0.5;
                assert!((point.position.y - (rect.y + rect.height * 0.5)).abs() < 1e-8);
                if point.value < 0.0 {
                    assert!((rect.right() - zero).abs() < 1e-8);
                    assert!(rect.x < zero);
                } else {
                    assert!((rect.x - zero).abs() < 1e-8);
                    assert!(rect.right() > zero);
                }
            }
        }
    }
}

#[test]
fn date_bar_widths_follow_nearest_dates_without_overlapping_neighbors() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    let x = DateTimeAxis::new();
    x.set_minimum(Some(date(2019, 12, 31))).unwrap();
    x.set_maximum(Some(date(2021, 1, 2))).unwrap();
    chart.axes().add(x.clone()).unwrap();
    let series = BarSeries::new();
    series.set_orientation(BarOrientation::Vertical);
    series.set_x_axis(Some(x.upcast())).unwrap();
    series.set_x_values(Some(Samples::from_values([
        date(2020, 1, 1),
        date(2020, 1, 2),
        date(2021, 1, 1),
    ])));
    series.set_y_values(Some(Samples::from_values([1.0, 2.0, 3.0])));
    chart.series().add(series).unwrap();
    let layout = chart.layout_snapshot(Size::new(800.0, 400.0));
    assert_eq!(3, layout.points.len());
    let rects: Vec<_> = layout
        .points
        .iter()
        .map(|point| point.bar_bounds.unwrap())
        .collect();
    assert!(rects
        .iter()
        .all(|rect| rect.width > 0.0 && rect.width.is_finite()));
    assert!(rects.windows(2).all(|pair| pair[0].right() <= pair[1].x));
}

#[test]
fn mutable_stroke_axis_override_and_resource_brushes_invalidate_cached_scenes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let stroke = SolidColorBrush::with_color(Colors::RED);
    let axis_brush = SolidColorBrush::with_color(Colors::RED);
    let override_brush = SolidColorBrush::with_color(Colors::RED);
    let resource_brush = SolidColorBrush::with_color(Colors::RED);
    let axis = LinearAxis::new();
    axis.set_axis_line_brush(Some(axis_brush.clone().into()));
    let series = line();
    series.set_stroke(Some(stroke.clone().into()));
    series.set_show_data_labels(true);
    series.data_label_overrides().add(
        1,
        DataLabelOverride::new("Peak", Some(override_brush.clone().into())),
    );
    let chart = chart_with_axes(series.clone(), None, Some(axis.clone().upcast()));
    let resource: Rc<dyn IBrush> = resource_brush.clone().into();
    chart
        .resources()
        .add_value("ChartsTickLabelBrush", resource);
    let size = Size::new(400.0, 300.0);
    let _ = chart.layout_snapshot(size);
    for brush in [&stroke, &axis_brush, &override_brush, &resource_brush] {
        let revision = chart.revision();
        let builds = chart.scene_build_count();
        brush.set_color(Colors::BLUE);
        assert_eq!(revision + 1, chart.revision());
        let _ = chart.layout_snapshot(size);
        assert_eq!(builds + 1, chart.scene_build_count());
    }

    // One mutable brush referenced by multiple roles is observed once.
    axis.set_tick_brush(Some(stroke.clone().into()));
    series.set_data_marker_brush(Some(stroke.clone().into()));
    let revision = chart.revision();
    stroke.set_color(Colors::GREEN);
    assert_eq!(revision + 1, chart.revision());

    series.set_stroke(None);
    series.set_data_marker_brush(None);
    axis.set_tick_brush(None);
    axis.set_axis_line_brush(None);
    series.data_label_overrides().clear();
    let replacement: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLACK));
    chart
        .resources()
        .set("ChartsTickLabelBrush", Some(Rc::new(replacement)));
    let revision = chart.revision();
    for brush in [&stroke, &axis_brush, &override_brush, &resource_brush] {
        brush.set_opacity(0.5);
    }
    assert_eq!(revision, chart.revision());
}

#[test]
fn explicit_foreground_wins_over_theme_resource_in_rendered_text() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let resource: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
    let local: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::BLUE));
    chart
        .resources()
        .add_value("ChartsControlForegroundBrush", resource);
    chart.set_foreground(Some(local));
    let window = show(&chart);
    let commands = draw_commands(&chart);
    let glyphs: Vec<_> = commands
        .iter()
        .filter(|command| command.starts_with("DrawGlyphRun "))
        .collect();
    assert!(!glyphs.is_empty());
    assert!(glyphs
        .iter()
        .all(|command| command.starts_with(&format!("DrawGlyphRun {} ", Colors::BLUE))));

    chart.clear_value(ferroui_controls::primitives::TemplatedControl::foreground_property());
    let commands = draw_commands(&chart);
    let glyphs: Vec<_> = commands
        .iter()
        .filter(|command| command.starts_with("DrawGlyphRun "))
        .collect();
    assert!(!glyphs.is_empty());
    assert!(glyphs
        .iter()
        .all(|command| command.starts_with(&format!("DrawGlyphRun {} ", Colors::RED))));
    window.close();
}

#[test]
fn duplicate_samples_references_receive_one_subscription_and_disconnect_after_removal() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let values = Rc::new(FerroList::from_items([1.0, 2.0, 3.0]));
    let samples = Samples::from_source(ItemsSource::new(values.clone()));
    let chart = Chart::new();
    chart.data().add(samples.clone()).unwrap();
    chart.data().add(samples.clone()).unwrap();
    for _ in 0..2 {
        let series = LineSeries::new();
        series.set_x_values(Some(samples.clone()));
        series.set_y_values(Some(samples.clone()));
        chart.series().add(series).unwrap();
    }
    let size = Size::new(400.0, 300.0);
    let _ = chart.layout_snapshot(size);
    let revision = chart.revision();
    values.set(1, 9.0);
    assert_eq!(revision + 1, chart.revision());
    assert_eq!(6, chart.layout_snapshot(size).points.len());
    chart.series().remove_at(0).unwrap();
    let revision = chart.revision();
    values.set(1, 8.0);
    assert_eq!(revision + 1, chart.revision());
    assert_eq!(3, chart.layout_snapshot(size).points.len());
    chart.series().clear().unwrap();
    chart.data().clear().unwrap();
    let revision = chart.revision();
    values.set(1, 7.0);
    assert_eq!(revision, chart.revision());
    assert!(chart.layout_snapshot(size).points.is_empty());
}

#[test]
fn reentrant_deferred_resource_resolution_does_not_duplicate_or_leak_subscriptions() {
    let chart = Chart::new();
    let series = line();
    chart.series().add(series.clone()).unwrap();
    assert_eq!(1, series.property_changed_subscriber_count());
    let weak = chart.downgrade();
    chart
        .resources()
        .add_deferred_fn("ChartsTickLabelBrush", move |_| {
            // Resource factories execute application code. This raises a nested
            // collection notification while the chart rebuilds its subscriptions.
            weak.upgrade()
                .unwrap()
                .data()
                .add(Samples::from_values([10.0]))
                .unwrap();
            let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::RED));
            Some(Rc::new(brush))
        });
    assert_eq!(1, chart.data().count());
    assert_eq!(1, series.property_changed_subscriber_count());
    chart.series().clear().unwrap();
    assert_eq!(0, series.property_changed_subscriber_count());
    let revision = chart.revision();
    series.set_title("Detached series");
    assert_eq!(revision, chart.revision());
}

#[test]
fn native_template_children_are_measured_and_arranged_without_changing_chart_sizing() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = populated_chart();
    let child = ferroui_controls::Border::new();
    child.set_width(24.0);
    child.set_height(12.0);
    let template_child = child.clone();
    chart.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
        move |_, _| template_child.clone().upcast(),
    )));
    let window = show(&chart);
    assert!(child.is_measure_valid());
    assert!(child.is_arrange_valid());
    assert_eq!(Size::new(24.0, 12.0), child.desired_size());
    assert_eq!(Size::new(24.0, 12.0), child.bounds().size());
    assert_eq!(Size::new(400.0, 300.0), chart.desired_size());
    assert_eq!(Size::new(400.0, 300.0), chart.bounds().size());
    chart.set_width(500.0);
    chart.update_layout();
    assert!(child.is_arrange_valid());
    assert_eq!(Size::new(24.0, 12.0), child.bounds().size());
    assert_eq!(Size::new(500.0, 300.0), chart.bounds().size());
    window.close();
}

#[test]
fn item_conversion_finishes_before_collection_bounds_and_ownership_checks() {
    struct ClearingConversion {
        collection: ChartSeriesCollection,
        item: Ref<LineSeries>,
    }
    impl ferroui_base::IntoRef<CartesianSeries> for ClearingConversion {
        fn into_ref(self) -> Ref<CartesianSeries> {
            self.collection.clear().unwrap();
            self.item.upcast()
        }
    }
    for operation in 0..3 {
        let chart = Chart::new();
        chart.series().add(line()).unwrap();
        let item = line();
        let conversion = ClearingConversion {
            collection: chart.series(),
            item: item.clone(),
        };
        let result = match operation {
            0 => chart.series().insert(1, conversion),
            1 => chart.series().set(0, conversion),
            _ => chart.series().add(conversion),
        };
        if operation < 2 {
            assert_eq!(Err(ChartError::IndexOutOfBounds), result);
            assert!(chart.series().is_empty());
            Chart::new().series().add(item).unwrap();
        } else {
            result.unwrap();
            assert_eq!(1, chart.series().count());
            assert!(chart.series().get(0).unwrap() == item);
        }
    }
}

#[test]
fn an_unnamed_chart_is_named_by_its_legend_title_or_its_series() {
    let chart = populated_chart();
    let peer = ControlAutomationPeer::create_peer_for_element(&chart);
    let titled: Vec<String> = chart
        .series()
        .snapshot()
        .iter()
        .map(|series| series.title())
        .filter(|title| !title.is_empty())
        .collect();
    if !titled.is_empty() {
        assert_eq!(titled.join(", "), peer.get_name());
    }
    chart.set_legend_title("Orders");
    assert_eq!("Orders", peer.get_name());
    AutomationProperties::set_name(&chart, Some("Weekly orders"));
    assert_eq!("Weekly orders", peer.get_name());
    assert_eq!(0, chart.scene_build_count());
}
