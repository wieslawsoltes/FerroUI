//! Not from upstream: a capture of a window that is rendered on a render thread holds the change
//! made before it.
//!
//! `capture_rendered_frame` ticks the render timer and reads the last frame. On a render thread a
//! batch is processed before its frame is drawn, and a tick that waited for the processed signal
//! could return with the frame from before the change (`DEVIATIONS.md`, Headless platform). The
//! test has a process of its own: an application is set up once for a process.

use ferroui_base::media::Brushes;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, PixelRect, Ref};
use ferroui_controls::{AppBuilder, Application, ApplicationImpl, Border, Control, NewApplication, Window};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessWindowExtensions};
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_simple::SimpleTheme;

#[repr(C)]
struct ThemedApplication {
    base: Application,
}

ferro_class!(ThemedApplication: Application);
ferro_impl_classes!(ThemedApplication: FerroObjectImpl);

impl NewApplication for ThemedApplication {
    fn new_application() -> Ref<Self> {
        let this = instantiate(Self { base: Application::construct() });
        this.styles().add(SimpleTheme::new().as_style());
        this
    }
}

impl ApplicationImpl for ThemedApplication {}

/// The pixel in the middle of the frame a capture gives.
fn middle_pixel(window: &Ref<Window>) -> [u8; 4] {
    let frame = window.capture_rendered_frame().expect("a rendered frame");
    let size = frame.pixel_size();
    let mut pixel = [0u8; 4];
    frame.copy_pixels(PixelRect::new(size.width / 2, size.height / 2, 1, 1), &mut pixel, 4);
    pixel
}

#[test]
fn a_capture_holds_the_change_before_it_when_rendering_on_a_render_thread() {
    AppBuilder::configure::<ThemedApplication>()
        .use_skia()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            should_render_on_ui_thread: false,
            ..Default::default()
        })
        .setup_without_starting();

    let border = Border::new();
    border.set_background(Some(Brushes::blue()));
    let window = Window::new();
    window.set_width(64.0);
    window.set_height(64.0);
    window.set_content(Some(Control::boxed(border.clone())));
    window.show();
    let mut previous = middle_pixel(&window);

    // Each change is followed by one capture, which has to show it: the pixel differs from the
    // one before the change. With a tick that returns before the frame is drawn the capture
    // repeats the frame before, in most of the rounds.
    for round in 0..40 {
        border.set_background(Some(if round % 2 == 0 { Brushes::red() } else { Brushes::blue() }));
        let pixel = middle_pixel(&window);
        assert_ne!(pixel, previous, "capture {round} holds the frame from before the change");
        previous = pixel;
    }
}
