//! Tests of the headless platform.
//!
//! The upstream tests (the unit test project of the headless platform) run
//! on the session of their test assembly, with the Simple theme and the Skia
//! backend: they are ported in `unit_tests`. The four tests of this file
//! that carry upstream's names are the upstream tests that need neither a
//! theme nor rendered pixels, run a second time with the headless drawing,
//! which the application of the upstream tests does not use; each one sets
//! up an application of its own with the headless platform, on the test
//! thread. Tests that are not from upstream say so.

use crate::headless_platform_render_interface::HeadlessPlatformRenderInterface;
use crate::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessWindowExtensions};
use ferroui_base::media::IntersectionResult;
use ferroui_base::platform::{IPlatformRenderInterface, PixelFormats};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, UnitTestDispatcherScope};
use ferroui_base::{FerroLocator, LocatorExtensions, PixelPoint, Point, Rect};
use ferroui_controls::{AppBuilder, Application, Window};
use std::rc::Rc;

/// An application set up with the headless platform for one test: isolates
/// the dispatcher, the service locator and the setup flag of the builder.
struct HeadlessApplication {
    locator: Rc<dyn IDisposable>,
    _dispatcher: UnitTestDispatcherScope,
}

impl HeadlessApplication {
    fn start(options: FerroHeadlessPlatformOptions) -> HeadlessApplication {
        let dispatcher = Dispatcher::unit_test_scope();
        AppBuilder::reset_setup_for_unit_tests();
        let locator = FerroLocator::enter_scope();
        AppBuilder::configure::<Application>().use_headless(options).setup_without_starting();
        HeadlessApplication { locator, _dispatcher: dispatcher }
    }
}

impl Drop for HeadlessApplication {
    fn drop(&mut self) {
        self.locator.dispose();
        AppBuilder::reset_setup_for_unit_tests();
    }
}

// --- RenderingTests ------------------------------------------------------------------------------

#[test]
fn should_keep_client_size_after_scaling_change() {
    let _app = HeadlessApplication::start(FerroHeadlessPlatformOptions::default());
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

// The first half of `Should_Change_Render_Scaling`; the second half compares the sizes of
// rendered frames, which the headless drawing does not produce.
#[test]
fn should_change_render_scaling() {
    let _app = HeadlessApplication::start(FerroHeadlessPlatformOptions::default());
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);

    window.show();

    window.set_render_scaling(2.0);

    assert_eq!(2.0, window.render_scaling());
}

// --- PopupTests ----------------------------------------------------------------------------------

#[test]
fn point_to_screen_respects_window_position() {
    let _app = HeadlessApplication::start(FerroHeadlessPlatformOptions::default());
    let window = Window::new();
    window.set_width(100.0);
    window.set_height(100.0);
    window.set_position(PixelPoint::new(100, 200));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    assert_eq!(PixelPoint::new(110, 220), window.point_to_screen(Point::new(10.0, 20.0)));
    assert_eq!(Point::new(10.0, 20.0), window.point_to_client(PixelPoint::new(110, 220)));

    window.close();
}

// --- ServicesTests -------------------------------------------------------------------------------

#[test]
fn can_access_screens() {
    let _app = HeadlessApplication::start(FerroHeadlessPlatformOptions::default());
    let window = Window::new();
    let screens = window.screens();

    let current_screen_from_window = screens.screen_from_window(&window);
    let current_screen_from_visual = screens.screen_from_visual(&window);

    match (current_screen_from_window, current_screen_from_visual) {
        (Some(from_window), Some(from_visual)) => assert!(Rc::ptr_eq(&from_window, &from_visual)),
        (None, None) => {}
        _ => panic!("the window and the visual are on different screens"),
    }
}

// --- tests of this port --------------------------------------------------------------------------

// Not an upstream test: the defaults of the options.
#[test]
fn the_options_have_the_defaults_of_the_original() {
    let options = FerroHeadlessPlatformOptions::default();

    assert_eq!(60, options.fps);
    assert!(options.should_render_on_ui_thread);
    assert!(options.use_headless_drawing);
    assert_eq!(PixelFormats::RGBA8888, options.frame_buffer_format);
    assert!(options.overlay_popups);
    assert_eq!(None, options.use_shared_mouse_device);
}

// Not an upstream test: the guard of `GetLastRenderedFrame`.
#[test]
#[should_panic(expected = "To capture a rendered frame")]
fn get_last_rendered_frame_is_not_supported_with_the_headless_drawing() {
    let _app = HeadlessApplication::start(FerroHeadlessPlatformOptions::default());
    let window = Window::new();
    window.show();

    window.get_last_rendered_frame();
}

// Not an upstream test: the fill intersection of the headless geometries with edges.
#[test]
fn rectangle_geometries_intersect_by_their_edges() {
    let scope = FerroLocator::enter_scope();
    HeadlessPlatformRenderInterface::initialize();
    let render_interface = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();

    let outer = render_interface.create_rectangle_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
    let inner = render_interface.create_rectangle_geometry(Rect::new(10.0, 10.0, 20.0, 20.0));
    let apart = render_interface.create_rectangle_geometry(Rect::new(200.0, 200.0, 10.0, 10.0));

    assert_eq!(IntersectionResult::FullyInside, outer.get_fill_intersection_result(&*inner));
    assert_eq!(IntersectionResult::FullyContains, inner.get_fill_intersection_result(&*outer));
    assert_eq!(IntersectionResult::Empty, outer.get_fill_intersection_result(&*apart));
    assert!(HeadlessPlatformRenderInterface::is_current());

    scope.dispose();
}

// Not an upstream test: a stream geometry takes its bounds from the points written to it.
#[test]
fn a_stream_geometry_has_the_bounds_of_its_points() {
    let scope = FerroLocator::enter_scope();
    HeadlessPlatformRenderInterface::initialize();
    let render_interface = FerroLocator::current().get_required_service::<dyn IPlatformRenderInterface>();

    let geometry = render_interface.create_stream_geometry();
    let mut context = geometry.open();
    context.begin_figure(Point::new(10.0, 20.0), true);
    context.line_to(Point::new(50.0, 20.0), true);
    context.line_to(Point::new(50.0, 80.0), true);
    context.end_figure(true);
    context.dispose();

    assert_eq!(Rect::new(10.0, 20.0, 40.0, 60.0), geometry.bounds());
    assert!(geometry.fill_contains(Point::new(45.0, 30.0)));
    assert!(!geometry.fill_contains(Point::new(500.0, 500.0)));

    scope.dispose();
}
