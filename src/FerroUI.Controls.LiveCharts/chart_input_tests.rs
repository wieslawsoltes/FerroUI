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



