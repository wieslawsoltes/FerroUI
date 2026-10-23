//! Native input and deep advanced-data regressions. Gesture conventions are
//! adapted from LiveChartsCore/CartesianChartEngine.cs at the pinned source.

use super::*;
use crate::{ChartSeries, GeoMarker, HeatLand, SankeyLink, SankeyNode, TreemapNode};
use ferroui_base::input::{
    KeyModifiers, Pointer, PointerPointProperties, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::{FerroObject, Vector};
use ferroui_controls::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use ferroui_controls::Window;
use ferroui_livecharts::SeriesKind;

fn cartesian() -> Ref<Chart> {
    let chart = Chart::new();
    chart.set_animations_speed(0.0);
    chart.set_zoom_mode(ZoomAndPanMode::BOTH);
    chart.set_draw_margin(Some(Rect::new(20.0, 20.0, 600.0, 320.0)));
    let series = ChartSeries::new();
    series.set_kind(SeriesKind::Line);
    series.set_numeric_values([0.0, 10.0]);
    chart.series().add(series);
    for axes in [chart.x_axes(), chart.y_axes()] {
        axes.add(Axis::new());
        axes.get(0).set_min_limit(Some(0.0));
        axes.get(0).set_max_limit(Some(10.0));
        axes.get(0).set_padding(0.0);
    }
    chart.scene(Size::new(640.0, 360.0)).unwrap();
    chart
}
fn mouse() -> Rc<dyn IPointer> {
    Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true)
}
fn attach(chart: &Ref<Chart>) -> Ref<Window> {
    let window = Window::with_impl(MockWindowingPlatform::create_window_mock_with_size(
        640.0, 360.0,
    ));
    window.set_content(Some(Control::boxed(chart)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    assert!(chart.presentation_source().is_some());
    window
}
fn press(chart: &Ref<Chart>, pointer: &Rc<dyn IPointer>, kind: PointerUpdateKind, position: Point) {
    let root = chart.visual_root().expect("pointer fixture is attached");
    let position = chart.translate_point(position, &root).unwrap();
    let args = PointerPressedEventArgs::new(
        chart.clone().upcast::<FerroObject>(),
        pointer.clone(),
        &root,
        position,
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, kind),
        KeyModifiers::NONE,
        1,
    );
    <Chart as InputElementImpl>::on_pointer_pressed(chart, &args);
}
fn release(chart: &Ref<Chart>, pointer: &Rc<dyn IPointer>, button: MouseButton, position: Point) {
    let root = chart.visual_root().expect("pointer fixture is attached");
    let position = chart.translate_point(position, &root).unwrap();
    let kind = if button == MouseButton::Right {
        PointerUpdateKind::RightButtonReleased
    } else {
        PointerUpdateKind::LeftButtonReleased
    };
    let args = PointerReleasedEventArgs::new(
        chart.clone().upcast::<FerroObject>(),
        pointer.clone(),
        &root,
        position,
        1,
        PointerPointProperties::new(RawInputModifiers::NONE, kind),
        KeyModifiers::NONE,
        button,
    );
    <Chart as InputElementImpl>::on_pointer_released(chart, &args);
}

#[test]
fn wheel_and_programmatic_zoom_ignore_axis_labels_and_outside_plot() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = cartesian();
    let window = attach(&chart);
    let axis = chart.x_axes().get(0);
    assert!(!chart.zoom_at(Point::new(10.0, 180.0), 0.5).unwrap());
    assert_eq!(axis.min_limit(), Some(0.0));
    assert_eq!(axis.max_limit(), Some(10.0));
    let args = PointerWheelEventArgs::new(
        chart.clone().upcast::<FerroObject>(),
        mouse(),
        &window,
        chart
            .translate_point(Point::new(10.0, 180.0), &window)
            .unwrap(),
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::NONE,
        Vector::new(0.0, 1.0),
    );
    <Chart as InputElementImpl>::on_pointer_wheel_changed(&chart, &args);
    assert!(!args.handled());
    assert_eq!(axis.max_limit(), Some(10.0));
}

#[test]
fn resize_before_pointer_input_uses_current_control_bounds() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = cartesian();
    chart.set_draw_margin(None);
    chart.measure(Size::new(400.0, 300.0));
    chart.arrange(Rect::new(0.0, 0.0, 400.0, 300.0));
    chart.scene(Size::new(400.0, 300.0)).unwrap();
    chart.measure(Size::new(800.0, 300.0));
    chart.arrange(Rect::new(0.0, 0.0, 800.0, 300.0));
    // The new plot spans [66, 788]. A stale 400px measurement puts this pivot
    // outside the plot and either rejects it or zooms around the wrong value.
    assert!(chart.zoom_at(Point::new(427.0, 150.0), 0.5).unwrap());
    assert_eq!(chart.axis_scales().0[0].pixel_end, 788.0);
    assert!((chart.x_axes().get(0).min_limit().unwrap() - 2.5).abs() < 1e-8);
    assert!((chart.x_axes().get(0).max_limit().unwrap() - 7.5).abs() < 1e-8);
}

#[test]
fn logarithmic_inverted_zoom_keeps_pointer_value_fixed() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = cartesian();
    let axis = chart.x_axes().get(0);
    axis.set_min_limit(Some(1.0));
    axis.set_max_limit(Some(1000.0));
    axis.set_logarithmic_base(Some(10.0));
    axis.set_inverted(true);
    chart.set_zoom_mode(ZoomAndPanMode::ZOOM_X);
    chart.scene(Size::new(640.0, 360.0)).unwrap();
    let pivot = Point::new(220.0, 180.0);
    let before = chart.axis_scales().0[0].to_chart_values(pivot.x);
    assert!(chart.zoom_at(pivot, 0.5).unwrap());
    chart.scene(Size::new(640.0, 360.0)).unwrap();
    let after = chart.axis_scales().0[0].to_chart_values(pivot.x);
    assert!((before - after).abs() / before < 1e-12);
    assert!(axis.min_limit().unwrap() > 0.0);
}

#[test]
fn polar_multiple_axes_convert_in_control_coordinates_with_title_layout() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = Chart::new();
    chart.set_chart_kind(ChartKind::Polar);
    chart.set_title("Independent polar axes".into());
    chart.set_initial_rotation(-90.0);
    chart.set_animations_speed(0.0);
    chart.x_axes().add(Axis::new());
    chart.y_axes().add(Axis::new());
    let angle = Axis::new();
    angle.set_min_limit(Some(0.0));
    angle.set_max_limit(Some(8.0));
    angle.set_inverted(true);
    chart.x_axes().add(angle);
    let radius = Axis::new();
    radius.set_min_limit(Some(0.0));
    radius.set_max_limit(Some(100.0));
    chart.y_axes().add(radius);
    let series = crate::PolarLineSeries::new();
    series.set_x_axis(1);
    series.set_y_axis(1);
    series.set_coordinates([
        ferroui_livecharts::kernel::Coordinate::new(1.0, 20.0),
        ferroui_livecharts::kernel::Coordinate::new(5.0, 80.0),
    ]);
    chart.series().add(series.upcast());
    let scene = chart.scene(Size::new(640.0, 360.0)).unwrap();
    let data = Point::new(2.0, 60.0);
    let pixel = chart.scale_data_to_pixels(data, 1, 1).unwrap().unwrap();
    let restored = chart.scale_pixels_to_data(pixel, 1, 1).unwrap().unwrap();
    assert!((restored.x - data.x).abs() < 1e-9);
    assert!((restored.y - data.y).abs() < 1e-9);
    assert!(scene.plot_bounds.y >= 48.0);
    assert_eq!(chart.axis_scales().0.len(), 2);
    assert_eq!(chart.axis_scales().1.len(), 2);
    assert!(chart.polar_scale(2, 0).unwrap().is_none());
    assert!(chart
        .scale_data_to_pixels(Point::new(f64::NAN, 1.0), 0, 0)
        .unwrap()
        .is_none());
}

#[test]
fn virtual_axes_support_input_without_mutating_the_declared_collections() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = Chart::new();
    chart.set_animations_speed(0.0);
    chart.set_zoom_mode(ZoomAndPanMode::BOTH);
    chart.set_draw_margin(Some(Rect::new(20.0, 20.0, 600.0, 320.0)));
    let series = crate::LineSeries::new();
    series.set_numeric_values([0.0, 10.0]);
    chart.series().add(series.upcast());
    let size = Size::new(640.0, 360.0);
    chart.scene(size).unwrap();
    let before = chart.axis_scales().0[0];
    let revision = chart.revision();
    assert!(chart.zoom_at(Point::new(320.0, 180.0), 0.5).unwrap());
    assert!(chart.revision() > revision);
    chart.scene(size).unwrap();
    let zoomed = chart.axis_scales().0[0];
    assert!(zoomed.max - zoomed.min < before.max - before.min);
    assert!(chart.x_axes().is_empty() && chart.y_axes().is_empty());
    chart.reset_zoom();
    chart.scene(size).unwrap();
    assert_eq!(chart.axis_scales().0[0], before);

    let declared = Axis::new();
    declared.set_min_limit(Some(20.0));
    declared.set_max_limit(Some(40.0));
    chart.x_axes().add(declared.clone());
    chart.scene(size).unwrap();
    assert_eq!(chart.axis_scales().0.len(), 1);
    assert_eq!(chart.axis_scales().0[0].min, 20.0);
    let revision = chart.revision();
    declared.set_max_limit(Some(50.0));
    assert!(chart.revision() > revision);
    chart.x_axes().clear();
    chart.scene(size).unwrap();
    assert_eq!(chart.axis_scales().0[0], before);
    let revision = chart.revision();
    declared.set_max_limit(Some(60.0));
    assert_eq!(chart.revision(), revision);
}

fn geo_map() -> Ref<Chart> {
    let chart = Chart::new();
    chart.set_chart_kind(ChartKind::GeoMap);
    chart.set_geo_data(Some(GeoData::default()));
    chart.set_title("Map viewport".into());
    chart.set_animations_speed(0.0);
    chart.scene(Size::new(640.0, 360.0)).unwrap();
    chart
}

#[test]
fn geographic_gestures_are_opt_in_and_zoom_keeps_local_pivot_fixed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let pivot = Point::new(270.0, 180.0);
    assert!(!chart.zoom_at(pivot, 0.5).unwrap());
    assert!(!chart.pan_by(pivot, Point::new(280.0, 190.0)).unwrap());
    chart.set_map_interaction_mode(MapInteractionMode::BOTH);
    let before = chart.unproject_coordinate(pivot).unwrap().unwrap();
    assert!(chart.zoom_at(pivot, 0.5).unwrap());
    assert_eq!(chart.geo_options().zoom, 2.0);
    let after = chart.unproject_coordinate(pivot).unwrap().unwrap();
    assert!((before.longitude - after.longitude).abs() < 1e-9);
    assert!((before.latitude - after.latitude).abs() < 1e-9);
    let projected = chart.project_coordinate(before).unwrap().unwrap();
    assert!((projected.x - pivot.x).abs() < 1e-9);
    assert!((projected.y - pivot.y).abs() < 1e-9);
    assert!(!chart.zoom_at(Point::new(1.0, 1.0), 0.5).unwrap());
    assert!(chart.zoom_at(pivot, f64::NAN).is_err());
    chart.reset_zoom();
    assert_eq!(chart.geo_options().zoom, 1.0);
    assert_eq!(chart.geo_options().pan, Point::new(0.0, 0.0));
}

#[test]
fn geographic_pointer_capture_settles_pan_and_mode_change_releases_capture() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let _window = attach(&chart);
    let pointer = mouse();
    let pivot = Point::new(320.0, 180.0);
    press(
        &chart,
        &pointer,
        PointerUpdateKind::LeftButtonPressed,
        pivot,
    );
    assert!(pointer.captured().is_none());
    chart.set_map_interaction_mode(MapInteractionMode::BOTH);
    chart.zoom_at(pivot, 0.5).unwrap();
    press(
        &chart,
        &pointer,
        PointerUpdateKind::LeftButtonPressed,
        pivot,
    );
    assert!(pointer.captured().is_some());
    assert!(chart.pan_by(pivot, Point::new(2000.0, 2000.0)).unwrap());
    release(
        &chart,
        &pointer,
        MouseButton::Left,
        Point::new(2000.0, 2000.0),
    );
    assert!(pointer.captured().is_none());
    let plot = chart.scene(chart.bounds().size()).unwrap().plot_bounds;
    assert!(chart.geo_options().pan.x.abs() <= plot.width * 0.5 + 1e-9);
    assert!(chart.geo_options().pan.y.abs() <= plot.height * 0.5 + 1e-9);
    press(
        &chart,
        &pointer,
        PointerUpdateKind::LeftButtonPressed,
        pivot,
    );
    assert!(pointer.captured().is_some());
    chart.set_map_interaction_mode(MapInteractionMode::NONE);
    assert!(pointer.captured().is_none());
}

#[test]
fn orthographic_coordinate_conversion_rejects_hidden_hemisphere() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let mut options = chart.geo_options();
    options.projection = ferroui_livecharts::geo::MapProjection::Orthographic;
    chart.set_geo_options(options);
    assert!(chart
        .project_coordinate(GeoCoordinate::new(0.0, 0.0))
        .unwrap()
        .is_some());
    assert!(chart
        .project_coordinate(GeoCoordinate::new(180.0, 0.0))
        .unwrap()
        .is_none());
    assert!(chart
        .unproject_coordinate(Point::new(-1000.0, -1000.0))
        .unwrap()
        .is_none());
}

#[test]
fn animated_hit_testing_uses_the_displayed_point_position() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = cartesian();
    let series = chart.series().get(0);
    series.set_kind(SeriesKind::Scatter);
    series.set_coordinates([ferroui_livecharts::kernel::Coordinate::new(5.0, 2.0)]);
    chart.set_tooltip_finding_strategy(ferroui_livecharts::FindingStrategy::ExactMatch);
    let _window = attach(&chart);
    let initial = chart.scene(chart.bounds().size()).unwrap();
    let before = initial.hits[0].center;
    chart.set_animations_speed(10_000.0);
    series.values().get(0).set_y(8.0);
    let target = chart.scene(chart.bounds().size()).unwrap();
    assert!(!target.hits[0].contains(before));
    let hits = chart.find_points(before);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, target.hits[0].id);
}

#[test]
fn section_zoom_normalizes_reverse_drag_and_clamps_to_plot() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = cartesian();
    chart.set_zoom_mode(ZoomAndPanMode::ZOOM_X);
    assert!(chart
        .zoom_to_section(Point::new(470.0, 100.0), Point::new(-40.0, 300.0))
        .unwrap());
    assert_eq!(chart.x_axes().get(0).min_limit(), Some(0.0));
    assert_eq!(chart.x_axes().get(0).max_limit(), Some(7.5));
    assert_eq!(chart.y_axes().get(0).max_limit(), Some(10.0));
    chart.set_zoom_mode(ZoomAndPanMode::ZOOM_X | ZoomAndPanMode::NO_ZOOM_BY_SECTION);
    assert!(!chart
        .zoom_to_section(Point::new(100.0, 100.0), Point::new(300.0, 200.0))
        .unwrap());
}

#[test]
fn middle_button_is_ignored_and_escape_releases_real_pointer_capture() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = cartesian();
    let _window = attach(&chart);
    let pointer = mouse();
    press(
        &chart,
        &pointer,
        PointerUpdateKind::MiddleButtonPressed,
        Point::new(320.0, 180.0),
    );
    assert!(pointer.captured().is_none());
    assert!(chart.pointer_down.get().is_none());
    press(
        &chart,
        &pointer,
        PointerUpdateKind::LeftButtonPressed,
        Point::new(320.0, 180.0),
    );
    assert!(pointer.captured().is_some());
    assert!(chart.pointer_down.get().is_some());
    let mut escape = KeyEventArgs::new();
    escape.key = Key::Escape;
    <Chart as InputElementImpl>::on_key_down(&chart, &escape);
    assert!(escape.handled());
    assert!(pointer.captured().is_none());
    assert!(chart.pointer_down.get().is_none());
}

#[test]
fn right_button_section_and_inverted_trigger_use_distinct_gestures() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = cartesian();
    let _window = attach(&chart);
    let pointer = mouse();
    press(
        &chart,
        &pointer,
        PointerUpdateKind::RightButtonPressed,
        Point::new(170.0, 100.0),
    );
    assert!(chart.section_start.get().is_some());
    assert!(chart.pointer_down.get().is_none());
    release(
        &chart,
        &pointer,
        MouseButton::Right,
        Point::new(470.0, 260.0),
    );
    assert_eq!(chart.x_axes().get(0).min_limit(), Some(2.5));
    assert_eq!(chart.x_axes().get(0).max_limit(), Some(7.5));
    assert!(pointer.captured().is_none());
    chart.set_zoom_mode(ZoomAndPanMode::BOTH | ZoomAndPanMode::INVERT_PANNING_POINTER_TRIGGER);
    press(
        &chart,
        &pointer,
        PointerUpdateKind::RightButtonPressed,
        Point::new(320.0, 180.0),
    );
    assert!(chart.pointer_down.get().is_some());
    assert!(chart.section_start.get().is_none());
    chart.cancel_pointer_gesture();
    assert_eq!(
        pointer_action(
            ZoomAndPanMode::BOTH,
            PointerType::Touch,
            MouseButton::Left,
            2
        ),
        PointerAction::Section
    );
    assert_eq!(
        pointer_action(
            ZoomAndPanMode::BOTH | ZoomAndPanMode::INVERT_PANNING_POINTER_TRIGGER,
            PointerType::Touch,
            MouseButton::Left,
            2
        ),
        PointerAction::Pan
    );
}

#[test]
fn deep_sankey_mutations_invalidate_and_replacing_collection_unsubscribes() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = Chart::new();
    chart.set_chart_kind(ChartKind::Sankey);
    let mut options = chart.sankey_options();
    options.show_labels = false;
    chart.set_sankey_options(options);
    let a = SankeyNode::with_name("A");
    let b = SankeyNode::with_name("B");
    chart.set_sankey_nodes(SankeyNodes::from_items([a.clone(), b.clone()]));
    let link = SankeyLink::with_nodes(a, b, 2.0);
    let links = SankeyLinks::from_items([link.clone()]);
    chart.set_sankey_links(links.clone());
    let before = chart.scene(Size::new(640.0, 360.0)).unwrap();
    let revision = chart.revision();
    link.set_weight(5.0);
    assert!(chart.revision() > revision);
    let after = chart.scene(Size::new(640.0, 360.0)).unwrap();
    assert!(!Rc::ptr_eq(&before, &after));
    assert!(after.hits.iter().any(|hit| hit.value == 5.0));
    chart.set_sankey_links(SankeyLinks::new());
    let revision = chart.revision();
    link.set_weight(9.0);
    assert_eq!(chart.revision(), revision);
    assert!(!links.has_collection_changed_subscribers());
}

#[test]
fn treemap_nested_mutations_remeasure_and_cycles_report_error() {
    let _scope = Dispatcher::unit_test_scope();
    let chart = Chart::new();
    chart.set_chart_kind(ChartKind::Treemap);
    let mut options = chart.treemap_options();
    options.show_labels = false;
    chart.set_treemap_options(options);
    let leaf = TreemapNode::with_value(2.0, "leaf");
    let children = TreemapNodes::from_items([leaf.clone()]);
    let root = TreemapNode::with_children("root", children.clone());
    chart.set_treemap_roots(TreemapNodes::from_items([root.clone()]));
    chart.scene(Size::new(640.0, 360.0)).unwrap();
    let revision = chart.revision();
    leaf.set_numeric_value(7.0);
    assert!(chart.revision() > revision);
    children.add(root);
    assert!(chart.scene(Size::new(640.0, 360.0)).is_err());
    children.clear();
}

#[test]
fn geography_overlays_are_observed_and_default_collections_are_independent() {
    let _scope = Dispatcher::unit_test_scope();
    let a = Chart::new();
    let b = Chart::new();
    let heat = HeatLand::new();
    a.heat_lands().add(heat.clone());
    let marker = GeoMarker::new();
    a.geo_markers().add(marker.clone());
    assert!(b.heat_lands().is_empty());
    assert!(b.geo_markers().is_empty());
    let revision = a.revision();
    heat.set_numeric_value(42.0);
    assert!(a.revision() > revision);
    let revision = a.revision();
    marker.set_longitude(12.0);
    assert!(a.revision() > revision);
}
