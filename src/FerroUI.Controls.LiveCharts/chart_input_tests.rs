//! Native input regressions. Gesture conventions are
//! adapted from LiveChartsCore/CartesianChartEngine.cs at the pinned source.

use super::*;
use crate::ChartSeries;
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
    let _app = UnitTestApplication::start(TestServices::styled_window());
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
    let _app = UnitTestApplication::start(TestServices::styled_window());
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
    // The title takes the top of the control; the polar plot is measured below it.
    assert!(scene.plot_bounds.y >= 48.0);
    assert_eq!(chart.axis_scales().0.len(), 2);
    assert_eq!(chart.axis_scales().1.len(), 2);
    assert!(chart.polar_scale(2, 0).unwrap().is_none());
    assert!(chart
        .scale_data_to_pixels(Point::new(f64::NAN, 1.0), 0, 0)
        .unwrap()
        .is_none());
    // A polar chart has no zoom, pan or section gestures.
    assert!(!chart.zoom_at(pixel, 0.5).unwrap());
    let cartesian = cartesian();
    assert!(cartesian.polar_scale(0, 0).unwrap().is_none());
}

#[test]
fn virtual_axes_support_input_without_mutating_the_declared_collections() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
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
    let _app = UnitTestApplication::start(TestServices::styled_window());
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




// The geographic map: the cases of the pinned GeoMapTests, GeoMapEventsTests,
// GeoMapReattachTests, GeoMapAnimationTests and ChartInteractiveApiTests that need a host.

/// The fixture of GeoMapEventsTests: a Mercator map with Russia at 42.
fn geo_map() -> Ref<Chart> {
    let chart = crate::GeoMap::new().upcast::<Chart>();
    chart.set_map_projection(ferroui_livecharts::MapProjection::Mercator);
    let series = crate::HeatLandSeries::new();
    series.lands().add(crate::HeatLand::with_value("rus", 42.0));
    chart.series().add(series.upcast());
    chart
}
fn moved(chart: &Ref<Chart>, pointer: &Rc<dyn IPointer>, position: Point) {
    let root = chart.visual_root().expect("pointer fixture is attached");
    let position = chart.translate_point(position, &root).unwrap();
    let args = PointerEventArgs::new(
        Some(&ferroui_base::input::InputElement::pointer_moved_event()),
        chart.clone().upcast::<FerroObject>(),
        pointer.clone(),
        Some(&*root),
        position,
        0,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
        KeyModifiers::NONE,
    );
    <Chart as InputElementImpl>::on_pointer_moved(chart, &args);
}
fn clicks(chart: &Ref<Chart>) -> (Rc<RefCell<Vec<ChartPointerEventArgs>>>, Subscription) {
    let captured = Rc::new(RefCell::new(Vec::new()));
    let sink = captured.clone();
    let subscription = Subscription::new(
        chart.data_pointer_down(move |args| sink.borrow_mut().push(args.clone())),
    );
    (captured, subscription)
}

/// GeoMapEventsTests.DataPointerDownFiresWhenClickingInsideALand and
/// DataPointerDownPayloadCarriesLandAndPositionInsideHoverArea, GeoMapTests.
/// GeoMap_PointerUpOnLand_FiresDataPointerDownWithLandDefinition: a click on Moscow reports
/// Russia at the release, with its value and a hover area around the click.
#[test]
fn geo_map_click_on_a_land_fires_data_pointer_down_at_the_release() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let _window = attach(&chart);
    let moscow = chart.project(37.62, 55.75).unwrap().unwrap();
    let (captured, _subscription) = clicks(&chart);
    let pointer = mouse();
    press(&chart, &pointer, PointerUpdateKind::LeftButtonPressed, moscow);
    assert!(captured.borrow().is_empty(), "a map reports a click at its release");
    release(&chart, &pointer, MouseButton::Left, moscow);
    let events = captured.borrow();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].points.len(), 1, "one land under the click");
    let hit = &events[0].points[0];
    let land = chart.map_land(hit.id).unwrap();
    assert_eq!(land.short_name, "rus");
    assert_eq!(land.value(), 42.0);
    let bounds = hit.shape.hover_bounds();
    assert!(moscow.x >= bounds.x && moscow.x <= bounds.x + bounds.width);
    assert!(moscow.y >= bounds.y && moscow.y <= bounds.y + bounds.height);
}

/// GeoMapEventsTests.DataPointerDownDoesNotFireWhenClickingTheOcean and
/// DataPointerDownDoesNotFireWhenPointerMovesPastClickThreshold, GeoMapTests.
/// GeoMap_DragGesture_DoesNotFireDataPointerDown and ChartInteractiveApiTests.
/// GeoMap_PointerEventsFlowThroughInvokers.
#[test]
fn geo_map_ocean_clicks_and_drags_fire_nothing() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let _window = attach(&chart);
    let (captured, _subscription) = clicks(&chart);
    let pointer = mouse();
    let ocean = chart.project(-30.0, 0.0).unwrap().unwrap();
    press(&chart, &pointer, PointerUpdateKind::LeftButtonPressed, ocean);
    release(&chart, &pointer, MouseButton::Left, ocean);
    assert!(captured.borrow().is_empty());
    let moscow = chart.project(37.62, 55.75).unwrap().unwrap();
    let away = Point::new(moscow.x + 20.0, moscow.y + 20.0);
    press(&chart, &pointer, PointerUpdateKind::LeftButtonPressed, moscow);
    moved(&chart, &pointer, away);
    release(&chart, &pointer, MouseButton::Left, away);
    assert!(captured.borrow().is_empty());
    // Within the 5 pixels of a click, the press is still a click.
    let near = Point::new(moscow.x + 2.0, moscow.y + 2.0);
    press(&chart, &pointer, PointerUpdateKind::LeftButtonPressed, moscow);
    moved(&chart, &pointer, near);
    release(&chart, &pointer, MouseButton::Left, near);
    assert_eq!(captured.borrow().len(), 1);
}

/// GeoMapTests.GeoMap_HoverTransitions_FireHoveredPointsChanged: entering France, moving to
/// Germany and leaving the chart.
#[test]
fn geo_map_hover_transitions_report_each_land() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let _window = attach(&chart);
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let _subscription = Subscription::new(chart.hover_changed(move |args| {
        sink.borrow_mut().push(args.points.iter().map(|p| p.id).collect::<Vec<_>>())
    }));
    let paris = chart.project(2.35, 48.85).unwrap().unwrap();
    let berlin = chart.project(13.40, 52.52).unwrap().unwrap();
    let mouse = mouse();
    moved(&chart, &mouse, paris);
    moved(&chart, &mouse, berlin);
    <Chart as InputElementImpl>::on_pointer_exited(
        &chart,
        &PointerEventArgs::new(
            Some(&ferroui_base::input::InputElement::pointer_exited_event()),
            chart.clone().upcast::<FerroObject>(),
            mouse.clone(),
            None,
            Point::new(0.0, 0.0),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ),
    );
    let events = events.borrow();
    assert_eq!(events.len(), 3, "enter, transition, exit");
    let name = |ids: &Vec<ferroui_livecharts::PointId>| chart.map_land(ids[0]).unwrap().short_name;
    assert_eq!(name(&events[0]), "fra");
    assert_eq!(name(&events[1]), "deu");
    assert!(events[2].is_empty());
}

/// GeoMapAnimationTests.GeoMap_FirstDraw_LandColorsAppearAtFinalGradient and
/// GeoMap_DataChange_LandColorsJumpToNewGradientPositions: the lands of a map take their colours
/// at once; a map has no scene transition.
#[test]
fn geo_map_land_colours_change_without_animation() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    chart.set_animations_speed(1000.0);
    let _window = attach(&chart);
    let size = chart.bounds().size();
    let first = chart.scene(size).unwrap();
    assert!(chart.transition.borrow().is_none());
    let series = chart.series().get(0).cast::<crate::HeatLandSeries>().unwrap();
    series.lands().add(crate::HeatLand::with_value("usa", 90.0));
    let second = chart.scene(size).unwrap();
    assert!(chart.transition.borrow().is_none());
    let shown = chart.presentation_scene(Dispatcher::ui_thread().now() as f64, second.clone());
    assert!(Rc::ptr_eq(&shown, &second));
    let fill = |scene: &Scene, name: &str| {
        let land = scene
            .hits
            .iter()
            .find(|hit| chart.map_land(hit.id).is_some_and(|l| l.short_name == name))
            .unwrap()
            .id;
        scene.primitives.iter().find(|p| p.id == Some(land)).unwrap().style.fill
    };
    // Russia, alone, is at the start of the ramp; with the United States at 90 it is still the
    // lower end, and the United States takes the upper end at once.
    assert_eq!(fill(&first, "rus"), fill(&second, "rus"));
    assert_eq!(
        fill(&second, "usa"),
        Some(ferroui_livecharts::geo_map_chart_engine::HEAT_LAND_MAP[1])
    );
}

/// GeoMapReattachTests: a map detached and attached again draws again, and detaching it ends
/// its hover (Unload_HidesTooltipAndClearsHoveredLand).
#[test]
fn geo_map_reattaches_and_detaching_ends_the_hover() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    // Load_BeforeAnyUnload_IsNoOpAndDoesNotThrow: a measure before any attach.
    assert!(!chart.scene(Size::new(640.0, 360.0)).unwrap().hits.is_empty());
    let window = attach(&chart);
    let moscow = chart.project(37.62, 55.75).unwrap().unwrap();
    chart.move_pointer(moscow);
    assert!(chart.hover.borrow().is_some());
    let settle = || {
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
    };
    window.set_content(None);
    settle();
    assert!(!chart.attached.get());
    assert!(chart.hover.borrow().is_none());
    // Unload_IsIdempotent.
    window.set_content(None);
    settle();
    window.set_content(Some(Control::boxed(&chart)));
    settle();
    assert!(chart.attached.get());
    assert!(!chart.scene(Size::new(640.0, 360.0)).unwrap().hits.is_empty());
    window.set_content(None);
    settle();
    assert!(!chart.attached.get());
}

/// The wheel zooms a map in steps and the primary button pans it when its interaction mode
/// allows; a release settles the viewport.
#[test]
fn geo_map_wheel_zoom_and_drag_pan_follow_the_interaction_mode() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let chart = geo_map();
    let window = attach(&chart);
    let center = Point::new(320.0, 180.0);
    let wheel = |delta: f64| {
        let args = PointerWheelEventArgs::new(
            chart.clone().upcast::<FerroObject>(),
            mouse(),
            &window,
            chart.translate_point(center, &window).unwrap(),
            0,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::Other),
            KeyModifiers::NONE,
            Vector::new(0.0, delta),
        );
        <Chart as InputElementImpl>::on_pointer_wheel_changed(&chart, &args);
        args.handled()
    };
    // MapInteractionMode.None by default.
    assert!(!wheel(1.0));
    assert_eq!(chart.zoom_level(), 1.0);
    chart.set_interaction_mode(MapInteractionMode::BOTH);
    assert!(wheel(1.0));
    assert!((chart.zoom_level() - 1.25).abs() < 1e-9);
    let pointer = mouse();
    let before = chart.pan_offset();
    press(&chart, &pointer, PointerUpdateKind::LeftButtonPressed, center);
    assert!(pointer.captured().is_some());
    moved(&chart, &pointer, Point::new(center.x + 30.0, center.y + 10.0));
    let after = chart.pan_offset();
    assert!((after.x - before.x - 30.0).abs() < 1e-9 && (after.y - before.y - 10.0).abs() < 1e-9);
    release(&chart, &pointer, MouseButton::Left, Point::new(center.x + 30.0, center.y + 10.0));
    assert!(pointer.captured().is_none());
    // A zoom out below the minimum is taken back by the bounce of the release.
    chart.reset_viewport();
    chart.zoom_map(center, ZoomDirection::ZoomOut);
    assert!((chart.zoom_level() - 0.8).abs() < 1e-9);
    chart.bounce_back();
    assert_eq!(chart.map_animation.borrow().len(), 1);
}
