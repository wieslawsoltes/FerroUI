//! Port of `RenderingTests.cs` of the upstream unit test project of the
//! headless platform.

use super::oneshot;
use super::test_application::ferro_fact;
use crate::HeadlessWindowExtensions;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{
    Brushes, CombinedGeometry, Geometry, GeometryCollection, GeometryCombineMode, GeometryGroup, RectangleGeometry,
    StreamGeometry,
};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Ref, Thickness};
use ferroui_controls::{Border, ContentControl, Control, ItemsSource, ListBox, PathIcon, SizeToContent, Window};

/// The window of the first four tests: a content control around `content`,
/// sized to it.
fn window_with_content(content: Ref<Control>, padding: f64) -> Ref<Window> {
    let content_control = ContentControl::new();
    content_control.set_horizontal_alignment(HorizontalAlignment::Stretch);
    content_control.set_vertical_alignment(VerticalAlignment::Stretch);
    content_control.set_padding(Thickness::uniform(padding));
    content_control.set_content(Some(Control::boxed(content)));

    let window = Window::new();
    window.set_content(Some(Control::boxed(content_control)));
    window.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
    window
}

fn should_render_last_frame_to_bitmap() {
    let icon = PathIcon::new();
    icon.set_data(StreamGeometry::parse("M0,9 L10,0 20,9 19,10 10,2 1,10 z").unwrap().upcast::<Geometry>());
    let window = window_with_content(icon.upcast(), 4.0);

    window.show();

    let frame = window.capture_rendered_frame();

    assert!(frame.is_some());
}
ferro_fact!(should_render_last_frame_to_bitmap, shows_a_window);

fn should_not_crash_on_geometry_group() {
    let group = GeometryGroup::new();
    group.set_children(GeometryCollection::from_items([
        RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 50.0, 50.0)).upcast::<Geometry>(),
        RectangleGeometry::with_rect(Rect::new(50.0, 50.0, 100.0, 100.0)).upcast::<Geometry>(),
    ]));
    let icon = PathIcon::new();
    icon.set_data(group.upcast::<Geometry>());
    let window = window_with_content(icon.upcast(), 4.0);

    window.show();

    let frame = window.capture_rendered_frame();

    assert!(frame.is_some());
}
ferro_fact!(should_not_crash_on_geometry_group, shows_a_window);

fn should_not_crash_on_combined_geometry() {
    let icon = PathIcon::new();
    icon.set_data(
        CombinedGeometry::with_mode(
            GeometryCombineMode::Union,
            Some(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 50.0, 50.0)).upcast::<Geometry>()),
            Some(RectangleGeometry::with_rect(Rect::new(50.0, 50.0, 100.0, 100.0)).upcast::<Geometry>()),
        )
        .upcast::<Geometry>(),
    );
    let window = window_with_content(icon.upcast(), 4.0);

    window.show();

    let frame = window.capture_rendered_frame();

    assert!(frame.is_some());
}
ferro_fact!(should_not_crash_on_combined_geometry, shows_a_window);

fn should_not_hang_with_non_trivial_layout() {
    let list_box = ListBox::new();
    list_box.set_items_source(Some(ItemsSource::from_strs(["Test 1", "Test 2"])));
    let window = window_with_content(list_box.upcast(), 1.0);

    window.show();

    let frame = window.capture_rendered_frame();
    assert!(frame.is_some());
}
ferro_fact!(should_not_hang_with_non_trivial_layout, shows_a_window);

async fn should_render_to_a_compositor_snapshot_capture() {
    let content_control = ContentControl::new();
    content_control.set_horizontal_alignment(HorizontalAlignment::Stretch);
    content_control.set_vertical_alignment(VerticalAlignment::Stretch);
    content_control.set_width(100.0);
    content_control.set_height(100.0);
    content_control.set_background(Some(Brushes::green()));
    let window = Window::new();
    window.set_content(Some(Control::boxed(content_control)));
    window.set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);

    window.show();

    Dispatcher::ui_thread().run_jobs(None);

    let composition_visual = ElementComposition::get_element_visual(&window).unwrap();
    // The task of a server job is awaited through its continuation.
    let task = composition_visual.compositor().create_composition_visual_snapshot(&composition_visual, 1.0);
    let (completed, completion) = oneshot();
    task.on_completed(move || completed.send(()));
    completion.await;
    let snapshot = task.take_result().expect("the outcome of the snapshot").expect("the snapshot");

    assert_eq!(100.0, snapshot.size().width);
    assert_eq!(100.0, snapshot.size().height);
}
ferro_fact!(async should_render_to_a_compositor_snapshot_capture, shows_a_window);

fn should_change_render_scaling() {
    let border = Border::new();
    border.set_background(Some(Brushes::red()));
    let window = Window::new();
    window.set_content(Some(Control::boxed(border)));
    window.set_width(100.0);
    window.set_height(100.0);

    window.show();

    let frame_before = window.capture_rendered_frame();
    assert!(frame_before.is_some());

    let size_before = frame_before.unwrap().pixel_size();

    window.set_render_scaling(2.0);

    assert_eq!(2.0, window.render_scaling());

    let frame_after = window.capture_rendered_frame();
    assert!(frame_after.is_some());

    let size_after = frame_after.unwrap().pixel_size();

    assert_eq!(size_before.width * 2, size_after.width);
    assert_eq!(size_before.height * 2, size_after.height);
}
ferro_fact!(should_change_render_scaling, shows_a_window);

fn should_keep_client_size_after_scaling_change() {
    let window = Window::new();
    window.set_width(200.0);
    window.set_height(150.0);

    window.show();
    window.capture_rendered_frame();

    let client_size_before = window.client_size();

    window.set_render_scaling(2.0);
    window.capture_rendered_frame();

    assert_eq!(client_size_before.width, window.client_size().width);
    assert_eq!(client_size_before.height, window.client_size().height);
}
ferro_fact!(should_keep_client_size_after_scaling_change);
