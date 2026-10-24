// Behavioral counterparts of Microsoft WinUI Charts/APITests/ChartSeriesTests.cs.
// Copyright (c) Microsoft Corporation. Licensed under the MIT License.
// See NOTICE.md for the pinned source and the native collection adaptation.

use crate::{
    AreaSeries, Axis, BarOrientation, BarSeries, CartesianSeries, CategoryAxis, Chart, ChartError,
    DataLabelOverride, DataMarkerOverride, LineSeries, LinearAxis, MarkerShape, Samples,
    StrokeDashStyle,
};
use ferroui_base::collections::{FerroList, NotifyCollectionChangedAction};
use ferroui_base::media::immutable::{ImmutableLinearGradientBrush, ImmutableSolidColorBrush};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::IBrush;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Control, Window};
use std::cell::RefCell;
use std::rc::Rc;

fn scope() -> (TextTestScope, UnitTestApplicationScope) {
    let text = TextTestScope::new();
    let app = UnitTestApplication::start(TestServices::styled_window());
    (text, app)
}

fn all_series() -> [Ref<CartesianSeries>; 3] {
    [
        LineSeries::new().upcast(),
        AreaSeries::new().upcast(),
        BarSeries::new().upcast(),
    ]
}

fn brush(color: u32) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::from_uint32(color))
}

fn same_brush(actual: Option<Rc<dyn IBrush>>, expected: &Rc<dyn IBrush>) {
    assert!(Rc::ptr_eq(&actual.expect("Expected a brush"), expected));
}

fn ready(series: &CartesianSeries) {
    series.set_x_values(Some(Samples::from_values(["A", "B", "C"])));
    series.set_y_values(Some(Samples::from_values([2.0, -1.0, 4.0])));
}

fn load_chart(chart: &Ref<Chart>) -> Ref<Window> {
    let window = Window::new();
    window.set_width(400.0);
    window.set_height(300.0);
    window.set_content(Some(Control::boxed(chart)));
    window.show();
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(chart.is_loaded());
    window
}

fn load(series: &Ref<CartesianSeries>) -> (Ref<Chart>, Ref<Window>) {
    ready(series);
    let chart = Chart::new();
    chart.series().add(series).unwrap();
    let window = load_chart(&chart);
    (chart, window)
}

fn assert_defaults(series: &CartesianSeries) {
    assert_eq!(series.title(), "");
    assert!(series.is_visible());
    assert!(series.x_values().is_none());
    assert!(series.y_values().is_none());
    assert!(series.x_axis().is_none());
    assert!(series.y_axis().is_none());
    assert!(series.stroke().is_none());
    assert_eq!(series.stroke_thickness(), 1.0);
    assert_eq!(series.stroke_dash_style(), StrokeDashStyle::Solid);
    assert!(!series.show_data_labels());
    assert!(!series.show_data_markers());
    assert_eq!(series.marker_shape(), MarkerShape::Circle);
    assert!(series.data_label_brush().is_none());
    assert!(series.data_marker_brush().is_none());
}

#[test]
fn line_series_defaults_are_public_properties() {
    let _scope = scope();
    assert_defaults(&LineSeries::new());
}

#[test]
fn area_series_defaults_are_public_properties() {
    let _scope = scope();
    let series = AreaSeries::new();
    assert_defaults(&series);
    assert!(series.fill().is_none());
}

#[test]
fn bar_series_defaults_are_public_properties() {
    let _scope = scope();
    let series = BarSeries::new();
    assert_defaults(&series);
    assert!(series.fill().is_none());
    assert_eq!(series.orientation(), BarOrientation::Horizontal);
}

#[test]
fn cartesian_series_properties_round_trip_through_clr_properties() {
    let _scope = scope();
    for series in all_series() {
        let x = Samples::from_values([1.0, 2.0]);
        let y = Samples::from_values([3.0, 4.0]);
        let stroke = brush(0xffff0000);
        let label = brush(0xff0000ff);
        let marker = brush(0xff008000);
        let x_axis: Ref<Axis> = CategoryAxis::new().upcast();
        let y_axis: Ref<Axis> = LinearAxis::new().upcast();
        series.set_title("Revenue");
        series.set_is_visible(false);
        series.set_x_values(Some(x.clone()));
        series.set_y_values(Some(y.clone()));
        series.set_stroke(Some(stroke.clone()));
        series.set_stroke_thickness(2.5);
        series.set_stroke_dash_style(StrokeDashStyle::DashDot);
        series.set_show_data_labels(true);
        series.set_show_data_markers(true);
        series.set_marker_shape(MarkerShape::Diamond);
        series.set_data_label_brush(Some(label.clone()));
        series.set_data_marker_brush(Some(marker.clone()));
        series.set_x_axis(Some(x_axis.clone())).unwrap();
        series.set_y_axis(Some(y_axis.clone())).unwrap();
        assert_eq!(series.title(), "Revenue");
        assert!(!series.is_visible());
        assert_eq!(series.x_values(), Some(x));
        assert_eq!(series.y_values(), Some(y));
        same_brush(series.stroke(), &stroke);
        assert_eq!(series.stroke_thickness(), 2.5);
        assert_eq!(series.stroke_dash_style(), StrokeDashStyle::DashDot);
        assert!(series.show_data_labels());
        assert!(series.show_data_markers());
        assert_eq!(series.marker_shape(), MarkerShape::Diamond);
        same_brush(series.data_label_brush(), &label);
        same_brush(series.data_marker_brush(), &marker);
        assert_eq!(series.x_axis(), Some(x_axis));
        assert_eq!(series.y_axis(), Some(y_axis));
    }
}

#[test]
fn cartesian_series_properties_round_trip_through_dependency_properties() {
    let _scope = scope();
    for series in all_series() {
        let x = Samples::from_values([1.0]);
        let y = Samples::from_values([2.0]);
        let stroke = brush(0xffffa500);
        let label = brush(0xff800080);
        let marker = brush(0xffa52a2a);
        series.set_value(CartesianSeries::title_property(), "Expenses".to_owned());
        series.set_value(CartesianSeries::is_visible_property(), false);
        series.set_value(CartesianSeries::x_values_property(), Some(x.clone()));
        series.set_value(CartesianSeries::y_values_property(), Some(y.clone()));
        series.set_value(CartesianSeries::stroke_property(), Some(stroke.clone()));
        series.set_value(CartesianSeries::stroke_thickness_property(), 3.25);
        series.set_value(
            CartesianSeries::stroke_dash_style_property(),
            StrokeDashStyle::Dot,
        );
        series.set_value(CartesianSeries::show_data_labels_property(), true);
        series.set_value(CartesianSeries::show_data_markers_property(), true);
        series.set_value(CartesianSeries::marker_shape_property(), MarkerShape::Plus);
        series.set_value(
            CartesianSeries::data_label_brush_property(),
            Some(label.clone()),
        );
        series.set_value(
            CartesianSeries::data_marker_brush_property(),
            Some(marker.clone()),
        );
        assert_eq!(series.title(), "Expenses");
        assert!(!series.is_visible());
        assert_eq!(series.x_values(), Some(x));
        assert_eq!(series.y_values(), Some(y));
        same_brush(series.stroke(), &stroke);
        assert_eq!(series.stroke_thickness(), 3.25);
        assert_eq!(series.stroke_dash_style(), StrokeDashStyle::Dot);
        assert!(series.show_data_labels());
        assert!(series.show_data_markers());
        assert_eq!(series.marker_shape(), MarkerShape::Plus);
        same_brush(series.data_label_brush(), &label);
        same_brush(series.data_marker_brush(), &marker);
    }
}

#[test]
fn clear_value_restores_cartesian_series_defaults() {
    let _scope = scope();
    for series in all_series() {
        ready(&series);
        series.set_title("Temporary");
        series.set_is_visible(false);
        series.set_stroke(Some(brush(0xff123456)));
        series.set_stroke_thickness(5.0);
        series.set_stroke_dash_style(StrokeDashStyle::DashDotDot);
        series.set_show_data_labels(true);
        series.set_show_data_markers(true);
        series.set_marker_shape(MarkerShape::Square);
        series.set_data_label_brush(Some(brush(0xff123456)));
        series.set_data_marker_brush(Some(brush(0xff123456)));
        series.clear_value(CartesianSeries::title_property());
        series.clear_value(CartesianSeries::is_visible_property());
        series.clear_value(CartesianSeries::x_values_property());
        series.clear_value(CartesianSeries::y_values_property());
        series.clear_value(CartesianSeries::stroke_property());
        series.clear_value(CartesianSeries::stroke_thickness_property());
        series.clear_value(CartesianSeries::stroke_dash_style_property());
        series.clear_value(CartesianSeries::show_data_labels_property());
        series.clear_value(CartesianSeries::show_data_markers_property());
        series.clear_value(CartesianSeries::marker_shape_property());
        series.clear_value(CartesianSeries::data_label_brush_property());
        series.clear_value(CartesianSeries::data_marker_brush_property());
        assert_defaults(&series);
    }
}

#[test]
fn area_series_fill_round_trips_and_clear_value_restores_default() {
    let _scope = scope();
    let area = AreaSeries::new();
    let first = brush(0xffff0000);
    let next = brush(0xff0000ff);
    area.set_fill(Some(first.clone()));
    same_brush(area.fill(), &first);
    area.set_value(AreaSeries::fill_property(), Some(next.clone()));
    same_brush(area.fill(), &next);
    area.clear_value(AreaSeries::fill_property());
    assert!(area.fill().is_none());
}

#[test]
fn bar_series_fill_and_orientation_round_trip_and_clear_value_restores_defaults() {
    let _scope = scope();
    let bar = BarSeries::new();
    let first = brush(0xffff0000);
    let next = brush(0xff0000ff);
    bar.set_fill(Some(first.clone()));
    bar.set_orientation(BarOrientation::Vertical);
    same_brush(bar.fill(), &first);
    assert_eq!(bar.orientation(), BarOrientation::Vertical);
    bar.set_value(BarSeries::fill_property(), Some(next.clone()));
    bar.set_value(
        BarSeries::orientation_property(),
        BarOrientation::Horizontal,
    );
    same_brush(bar.fill(), &next);
    assert_eq!(bar.orientation(), BarOrientation::Horizontal);
    bar.clear_value(BarSeries::fill_property());
    bar.set_orientation(BarOrientation::Vertical);
    bar.clear_value(BarSeries::orientation_property());
    assert!(bar.fill().is_none());
    assert_eq!(bar.orientation(), BarOrientation::Horizontal);
}

#[test]
fn stroke_thickness_values_are_retained_by_public_property() {
    let _scope = scope();
    for series in all_series() {
        for value in [
            0.0,
            1.0,
            -1.0,
            2.5,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            f64::MAX,
        ] {
            series.set_stroke_thickness(value);
            assert_eq!(series.stroke_thickness().to_bits(), value.to_bits());
            assert!(series.effective_stroke_thickness().is_finite());
            assert!(series.effective_stroke_thickness() >= 0.0);
        }
    }
}

#[test]
fn stroke_thickness_invalid_values_do_not_throw_while_loaded() {
    let _scope = scope();
    for series in all_series() {
        let (chart, window) = load(&series);
        for value in [-1.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            series.set_stroke_thickness(value);
            window.update_layout();
            assert_eq!(series.stroke_thickness().to_bits(), value.to_bits());
            assert!(chart.is_loaded());
        }
        window.close();
    }
}

#[test]
fn stroke_dash_style_accepts_every_value() {
    let _scope = scope();
    for series in all_series() {
        let (chart, window) = load(&series);
        for value in [
            StrokeDashStyle::Solid,
            StrokeDashStyle::Dash,
            StrokeDashStyle::Dot,
            StrokeDashStyle::DashDot,
            StrokeDashStyle::DashDotDot,
        ] {
            series.set_stroke_dash_style(value);
            window.update_layout();
            assert_eq!(series.stroke_dash_style(), value);
            assert!(chart.is_loaded());
        }
        window.close();
    }
}

#[test]
fn unsupported_stroke_brush_is_accepted_and_retained() {
    let _scope = scope();
    // The upstream test uses a gradient unsupported by its renderer. FerroUI
    // supports gradients; it still preserves the exact supplied brush object.
    let gradient: Rc<dyn IBrush> = Rc::new(ImmutableLinearGradientBrush::from_stops(&[]));
    for series in all_series() {
        series.set_stroke(Some(gradient.clone()));
        let (chart, window) = load(&series);
        same_brush(series.stroke(), &gradient);
        assert!(chart.is_loaded());
        window.close();
    }
}

#[test]
fn automatic_stroke_is_not_written_back_while_loaded() {
    let _scope = scope();
    for series in all_series() {
        let (chart, window) = load(&series);
        assert!(series.stroke().is_none());
        assert!(series.x_axis().is_none());
        assert!(series.y_axis().is_none());
        assert!(chart.is_loaded());
        window.close();
    }
}

#[test]
fn stroke_can_change_and_clear_while_loaded() {
    let _scope = scope();
    for series in all_series() {
        let (_chart, window) = load(&series);
        for color in [0xffff0000, 0xff0000ff] {
            let next = brush(color);
            series.set_stroke(Some(next.clone()));
            window.update_layout();
            same_brush(series.stroke(), &next);
        }
        series.clear_value(CartesianSeries::stroke_property());
        window.update_layout();
        assert!(series.stroke().is_none());
        window.close();
    }
}

#[test]
fn title_and_visibility_can_change_before_and_after_series_is_added_to_loaded_chart() {
    let _scope = scope();
    let chart = Chart::new();
    chart.set_show_legend(true);
    let window = load_chart(&chart);
    for series in all_series() {
        ready(&series);
        series.set_title("Before");
        series.set_is_visible(false);
        chart.series().add(&series).unwrap();
        window.update_layout();
        assert_eq!(series.title(), "Before");
        assert!(!series.is_visible());
        series.set_title("After");
        series.set_is_visible(true);
        window.update_layout();
        assert_eq!(series.title(), "After");
        assert!(series.is_visible());
        series.set_optional_title(None);
        assert_eq!(series.title(), "");
    }
    assert!(chart.is_loaded());
}

#[test]
fn series_can_be_removed_and_added_again() {
    let _scope = scope();
    for series in all_series() {
        let (chart, window) = load(&series);
        assert!(chart.series().remove(&series));
        assert_eq!(series.owner_id(), None);
        chart.series().add(&series).unwrap();
        window.update_layout();
        assert_eq!(chart.series().get(0).unwrap(), series);
        window.close();
    }
}

#[test]
fn area_series_can_share_explicit_axes_with_line_series() {
    let _scope = scope();
    let chart = Chart::new();
    let x: Ref<Axis> = CategoryAxis::new().upcast();
    let y: Ref<Axis> = LinearAxis::new().upcast();
    chart.axes().add(&x).unwrap();
    chart.axes().add(&y).unwrap();
    let line = LineSeries::new();
    let area = AreaSeries::new();
    for series in [
        line.clone().upcast::<CartesianSeries>(),
        area.clone().upcast::<CartesianSeries>(),
    ] {
        ready(&series);
        series.set_x_axis(Some(x.clone())).unwrap();
        series.set_y_axis(Some(y.clone())).unwrap();
        chart.series().add(&series).unwrap();
    }
    let _window = load_chart(&chart);
    assert_eq!(line.x_axis(), area.x_axis());
    assert_eq!(line.y_axis(), area.y_axis());
    assert!(chart.is_loaded());
}

#[test]
fn bar_orientation_changes_while_loaded_with_automatic_axes() {
    let _scope = scope();
    let bar = BarSeries::new();
    let (chart, window) = load(&bar.clone().upcast());
    for value in [
        BarOrientation::Vertical,
        BarOrientation::Horizontal,
        BarOrientation::Vertical,
    ] {
        bar.set_orientation(value);
        window.update_layout();
        assert_eq!(bar.orientation(), value);
        assert!(bar.x_axis().is_none());
        assert!(bar.y_axis().is_none());
    }
    assert!(chart.is_loaded());
}

#[test]
fn bar_orientation_incompatible_with_explicit_axes_is_retained_without_throwing() {
    let _scope = scope();
    let chart = Chart::new();
    let bar = BarSeries::new();
    let x: Ref<Axis> = CategoryAxis::new().upcast();
    let y: Ref<Axis> = LinearAxis::new().upcast();
    chart.axes().add(&x).unwrap();
    chart.axes().add(&y).unwrap();
    ready(&bar);
    bar.set_x_axis(Some(x.clone())).unwrap();
    bar.set_y_axis(Some(y.clone())).unwrap();
    bar.set_orientation(BarOrientation::Vertical);
    chart.series().add(&bar).unwrap();
    let window = load_chart(&chart);
    bar.set_orientation(BarOrientation::Horizontal);
    window.update_layout();
    assert_eq!(bar.orientation(), BarOrientation::Horizontal);
    assert_eq!(bar.x_axis(), Some(x));
    assert_eq!(bar.y_axis(), Some(y));
    assert!(chart.is_loaded());
}

#[test]
fn marker_and_label_properties_can_change_while_loaded() {
    let _scope = scope();
    for series in all_series() {
        let (chart, window) = load(&series);
        for visible in [true, false, true] {
            series.set_show_data_labels(visible);
            series.set_show_data_markers(visible);
            series.set_marker_shape(MarkerShape::Triangle);
            window.update_layout();
            assert_eq!(series.show_data_labels(), visible);
            assert_eq!(series.show_data_markers(), visible);
            assert_eq!(series.marker_shape(), MarkerShape::Triangle);
        }
        assert!(chart.is_loaded());
        window.close();
    }
}

#[test]
fn marker_shape_accepts_every_value() {
    let _scope = scope();
    for series in all_series() {
        let (chart, window) = load(&series);
        series.set_show_data_markers(true);
        for shape in [
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
        ] {
            series.set_marker_shape(shape);
            window.update_layout();
            assert_eq!(series.marker_shape(), shape);
        }
        assert!(chart.is_loaded());
        window.close();
    }
}

#[test]
fn data_brushes_set_and_clear_while_loaded() {
    let _scope = scope();
    for series in all_series() {
        let (_chart, window) = load(&series);
        series.set_show_data_labels(true);
        series.set_show_data_markers(true);
        let label = brush(0xff0000ff);
        let marker = brush(0xffff0000);
        series.set_data_label_brush(Some(label.clone()));
        series.set_data_marker_brush(Some(marker.clone()));
        window.update_layout();
        same_brush(series.data_label_brush(), &label);
        same_brush(series.data_marker_brush(), &marker);
        series.set_data_label_brush(None);
        series.clear_value(CartesianSeries::data_marker_brush_property());
        window.update_layout();
        assert!(series.data_label_brush().is_none());
        assert!(series.data_marker_brush().is_none());
        window.close();
    }
}

#[test]
fn fill_set_and_clear_while_loaded() {
    let _scope = scope();
    let area = AreaSeries::new();
    let bar = BarSeries::new();
    for series in [
        area.upcast::<CartesianSeries>(),
        bar.upcast::<CartesianSeries>(),
    ] {
        let (chart, window) = load(&series);
        let fill = brush(0xffaabbcc);
        if let Some(area) = series.downcast_ref::<AreaSeries>() {
            area.set_fill(Some(fill.clone()));
            window.update_layout();
            same_brush(area.fill(), &fill);
            area.clear_value(AreaSeries::fill_property());
            assert!(area.fill().is_none());
        } else {
            let bar = series.downcast_ref::<BarSeries>().unwrap();
            bar.set_fill(Some(fill.clone()));
            window.update_layout();
            same_brush(bar.fill(), &fill);
            bar.clear_value(BarSeries::fill_property());
            assert!(bar.fill().is_none());
        }
        window.update_layout();
        assert!(chart.is_loaded());
        window.close();
    }
}

#[test]
fn data_label_overrides_start_empty_and_share_contents() {
    let _scope = scope();
    let series = LineSeries::new();
    let map = series.data_label_overrides();
    assert!(map.is_empty());
    let item = DataLabelOverride::new("Peak", None);
    map.add(2, item.clone());
    assert!(map.ptr_eq(&series.data_label_overrides()));
    assert!(Rc::ptr_eq(&series.data_label_overrides().get(&2), &item));
    assert!(LineSeries::new().data_label_overrides().is_empty());
}

#[test]
fn data_marker_overrides_start_empty_and_share_contents() {
    let _scope = scope();
    let series = LineSeries::new();
    let map = series.data_marker_overrides();
    assert!(map.is_empty());
    let item = DataMarkerOverride::new(MarkerShape::Diamond, None);
    map.add(2, item.clone());
    assert!(map.ptr_eq(&series.data_marker_overrides()));
    assert!(Rc::ptr_eq(&series.data_marker_overrides().get(&2), &item));
    assert!(LineSeries::new().data_marker_overrides().is_empty());
}

#[test]
fn data_label_override_map_supports_add_update_remove_and_clear() {
    let _scope = scope();
    let map = LineSeries::new().data_label_overrides();
    map.add(2, DataLabelOverride::new("First", None));
    assert_eq!(map.get(&2).text(), "First");
    map.set(2, DataLabelOverride::new("Second", None));
    assert_eq!(map.count(), 1);
    assert_eq!(map.get(&2).text(), "Second");
    assert!(map.remove(&2));
    assert!(!map.remove(&2));
    map.add(3, DataLabelOverride::new("Third", None));
    map.clear();
    assert!(map.is_empty());
}

#[test]
fn data_marker_override_map_supports_add_update_remove_and_clear() {
    let _scope = scope();
    let map = LineSeries::new().data_marker_overrides();
    map.add(2, DataMarkerOverride::new(MarkerShape::Circle, None));
    assert_eq!(map.get(&2).shape(), MarkerShape::Circle);
    map.set(2, DataMarkerOverride::new(MarkerShape::Diamond, None));
    assert_eq!(map.count(), 1);
    assert_eq!(map.get(&2).shape(), MarkerShape::Diamond);
    assert!(map.remove(&2));
    assert!(!map.remove(&2));
    map.add(3, DataMarkerOverride::new(MarkerShape::Plus, None));
    map.clear();
    assert!(map.is_empty());
}

#[test]
fn data_label_override_map_raises_events_for_add_update_remove_and_clear() {
    let _scope = scope();
    let map = LineSeries::new().data_label_overrides();
    let records = Rc::new(RefCell::new(Vec::new()));
    let output = records.clone();
    map.add_collection_changed(Rc::new(move |args| {
        output.borrow_mut().push((
            args.action,
            args.new_items
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            args.old_items
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
        ));
    }));
    map.add(4, DataLabelOverride::new("First", None));
    map.set(4, DataLabelOverride::new("Second", None));
    map.remove(&4);
    map.add(5, DataLabelOverride::new("Third", None));
    map.clear();
    use NotifyCollectionChangedAction::{Add, Remove, Replace};
    // Native FerroDictionary clear describes all removed entries, rather than
    // WinRT IObservableMap's keyless Reset. Consumers use this existing protocol.
    assert_eq!(
        *records.borrow(),
        vec![
            (Add, vec![4], vec![]),
            (Replace, vec![4], vec![4]),
            (Remove, vec![], vec![4]),
            (Add, vec![5], vec![]),
            (Remove, vec![], vec![5])
        ]
    );
}

#[test]
fn data_marker_override_map_raises_events_for_add_update_remove_and_clear() {
    let _scope = scope();
    let map = LineSeries::new().data_marker_overrides();
    let records = Rc::new(RefCell::new(Vec::new()));
    let output = records.clone();
    map.add_collection_changed(Rc::new(move |args| {
        output.borrow_mut().push((
            args.action,
            args.new_items
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
            args.old_items
                .iter()
                .map(|(key, _)| *key)
                .collect::<Vec<_>>(),
        ));
    }));
    map.add(4, DataMarkerOverride::new(MarkerShape::Circle, None));
    map.set(4, DataMarkerOverride::new(MarkerShape::Diamond, None));
    map.remove(&4);
    map.add(5, DataMarkerOverride::new(MarkerShape::Plus, None));
    map.clear();
    use NotifyCollectionChangedAction::{Add, Remove, Replace};
    assert_eq!(
        *records.borrow(),
        vec![
            (Add, vec![4], vec![]),
            (Replace, vec![4], vec![4]),
            (Remove, vec![], vec![4]),
            (Add, vec![5], vec![]),
            (Remove, vec![], vec![5])
        ]
    );
}

#[test]
fn data_label_override_descriptors_retain_text_and_brush() {
    let _scope = scope();
    let shared = brush(0xffff0000);
    let label = DataLabelOverride::new("Peak", Some(shared.clone()));
    let empty = DataLabelOverride::new("", None);
    assert_eq!(label.text(), "Peak");
    same_brush(label.brush(), &shared);
    assert_eq!(empty.text(), "");
    assert!(empty.brush().is_none());
}

#[test]
fn data_marker_override_descriptors_retain_shape_and_brush() {
    let _scope = scope();
    let shared = brush(0xff0000ff);
    let marker = DataMarkerOverride::new(MarkerShape::Diamond, Some(shared.clone()));
    let no_brush = DataMarkerOverride::new(MarkerShape::Plus, None);
    assert_eq!(marker.shape(), MarkerShape::Diamond);
    same_brush(marker.brush(), &shared);
    assert_eq!(no_brush.shape(), MarkerShape::Plus);
    assert!(no_brush.brush().is_none());
}

#[test]
fn overrides_beyond_current_samples_are_accepted_and_survive_growth() {
    let _scope = scope();
    let source = Rc::new(FerroList::from_items([1.0]));
    let series = LineSeries::new();
    series.set_y_values(Some(Samples::from_source(source.clone().into())));
    let label = DataLabelOverride::new("Future", None);
    let marker = DataMarkerOverride::new(MarkerShape::Diamond, None);
    series.data_label_overrides().add(3, label.clone());
    series.data_marker_overrides().add(3, marker.clone());
    let chart = Chart::new();
    chart.series().add(&series).unwrap();
    let window = load_chart(&chart);
    for value in [2.0, 3.0, 4.0] {
        source.add(value);
    }
    window.update_layout();
    assert!(Rc::ptr_eq(&series.data_label_overrides().get(&3), &label));
    assert!(Rc::ptr_eq(&series.data_marker_overrides().get(&3), &marker));
    assert_eq!(series.y_values().unwrap().count(), 4);
    assert!(chart.is_loaded());
}

#[test]
fn overrides_survive_sample_changes_and_series_removal() {
    let _scope = scope();
    let source = Rc::new(FerroList::from_items([1.0, 2.0, 3.0]));
    let series = LineSeries::new();
    series.set_y_values(Some(Samples::from_source(source.clone().into())));
    let label = DataLabelOverride::new("Index 1", None);
    let marker = DataMarkerOverride::new(MarkerShape::Square, None);
    series.data_label_overrides().add(1, label.clone());
    series.data_marker_overrides().add(1, marker.clone());
    let chart = Chart::new();
    chart.series().add(&series).unwrap();
    let window = load_chart(&chart);
    source.move_item(0, 2);
    source.remove_at(0);
    series.set_y_values(Some(Samples::from_values([99.0])));
    window.update_layout();
    assert!(chart.series().remove(&series));
    assert!(Rc::ptr_eq(&series.data_label_overrides().get(&1), &label));
    assert!(Rc::ptr_eq(&series.data_marker_overrides().get(&1), &marker));
    chart.series().add(&series).unwrap();
    window.update_layout();
    assert_eq!(series.data_label_overrides().count(), 1);
    assert_eq!(series.data_marker_overrides().count(), 1);
}

#[test]
fn same_brush_instance_can_be_shared_by_series_and_overrides() {
    let _scope = scope();
    let shared = brush(0xff2277cc);
    let series = AreaSeries::new();
    series.set_stroke(Some(shared.clone()));
    series.set_fill(Some(shared.clone()));
    series.set_data_label_brush(Some(shared.clone()));
    series.set_data_marker_brush(Some(shared.clone()));
    let label = DataLabelOverride::new("Peak", Some(shared.clone()));
    let marker = DataMarkerOverride::new(MarkerShape::Circle, Some(shared.clone()));
    series.data_label_overrides().add(0, label.clone());
    series.data_marker_overrides().add(0, marker.clone());
    let (_chart, _window) = load(&series.clone().upcast());
    for actual in [
        series.stroke(),
        series.fill(),
        series.data_label_brush(),
        series.data_marker_brush(),
        label.brush(),
        marker.brush(),
    ] {
        same_brush(actual, &shared);
    }
}

#[test]
fn raw_direct_axis_writes_are_rejected_without_mutating_or_panicking() {
    let _scope = scope();
    let series = LineSeries::new();
    let x: Ref<Axis> = CategoryAxis::new().upcast();
    let y: Ref<Axis> = LinearAxis::new().upcast();
    series.set_direct_value(CartesianSeries::x_axis_property(), Some(x.clone()));
    series.set_direct_value(CartesianSeries::y_axis_property(), Some(y.clone()));
    // A rejected write through the property system keeps the old axis and does not panic.
    series.set_direct_value(CartesianSeries::x_axis_property(), Some(y.clone()));
    assert_eq!(series.x_axis(), Some(x.clone()));
    assert_eq!(series.y_axis(), Some(y.clone()));
    let chart = Chart::new();
    chart.axes().add(&x).unwrap();
    chart.axes().add(&y).unwrap();
    chart.series().add(&series).unwrap();
    let outside: Ref<Axis> = LinearAxis::new().upcast();
    assert_eq!(
        series.set_y_axis(Some(outside.clone())),
        Err(ChartError::AxisNotInChart)
    );
    assert_eq!(series.y_axis(), Some(y.clone()));
    series.set_direct_value(CartesianSeries::y_axis_property(), Some(outside));
    assert_eq!(series.y_axis(), Some(y));
}

#[test]
fn enum_discriminants_match_the_published_api() {
    use crate::{CategorySortKey, DateTimeIntervalType, GridLines, SortOrder};
    assert_eq!(
        [
            BarOrientation::Horizontal as i32,
            BarOrientation::Vertical as i32
        ],
        [0, 1]
    );
    assert_eq!(
        [CategorySortKey::Index as i32, CategorySortKey::Value as i32],
        [0, 1]
    );
    assert_eq!(
        [SortOrder::Ascending as i32, SortOrder::Descending as i32],
        [0, 1]
    );
    assert_eq!(
        [
            GridLines::None as i32,
            GridLines::Major as i32,
            GridLines::Minor as i32
        ],
        [0, 1, 2]
    );
    assert_eq!(
        [
            DateTimeIntervalType::Auto as i32,
            DateTimeIntervalType::Day as i32,
            DateTimeIntervalType::Week as i32,
            DateTimeIntervalType::Month as i32,
            DateTimeIntervalType::Year as i32
        ],
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        [
            StrokeDashStyle::Solid as i32,
            StrokeDashStyle::Dash as i32,
            StrokeDashStyle::Dot as i32,
            StrokeDashStyle::DashDot as i32,
            StrokeDashStyle::DashDotDot as i32
        ],
        [0, 1, 2, 3, 4]
    );
    assert_eq!(
        [
            MarkerShape::None as i32,
            MarkerShape::Square as i32,
            MarkerShape::Diamond as i32,
            MarkerShape::Triangle as i32,
            MarkerShape::X as i32,
            MarkerShape::Asterisk as i32,
            MarkerShape::ShortDash as i32,
            MarkerShape::LongDash as i32,
            MarkerShape::Circle as i32,
            MarkerShape::Plus as i32
        ],
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]
    );
}
