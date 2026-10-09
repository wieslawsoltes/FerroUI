//! An application on each backend: the headless platform lays out a small
//! window of the Fluent theme (a text block, a button, a text box) and
//! renders it into a bitmap, once with `use_skia` and once with
//! `use_vello`, and the two frames are compared by the measure of the
//! scenes.
//!
//! An application chooses its backend on its builder; the tests of the
//! headless platform do the same (`use_harfbuzz().use_skia().use_headless`
//! with the headless drawing turned off). Each backend has a session of its
//! own here, which is a thread with its own services.

use crate::{compare, Pixels};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::platform::PixelFormat;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, Thickness};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, Button, Control, NewApplication, StackPanel, TextBlock, TextBox, Window,
};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use ferroui_skia::SkiaApplicationExtensions;
use ferroui_themes_fluent::FluentTheme;
use ferroui_vello::VelloApplicationExtensions;
use std::rc::Rc;
use std::sync::Mutex;

#[repr(C)]
struct FluentApplication {
    base: Application,
}

ferro_class!(FluentApplication: Application);
ferro_impl_classes!(FluentApplication: FerroObjectImpl);

impl NewApplication for FluentApplication {
    fn new_application() -> Ref<Self> {
        let this = instantiate(Self { base: Application::construct() });
        this.styles().add(FluentTheme::new().as_style());
        this
    }
}

impl ApplicationImpl for FluentApplication {}

fn headless_options() -> FerroHeadlessPlatformOptions {
    FerroHeadlessPlatformOptions { use_headless_drawing: false, ..FerroHeadlessPlatformOptions::default() }
}

fn build_skia_app() -> AppBuilder {
    AppBuilder::configure::<FluentApplication>().use_harfbuzz().use_skia().use_headless(headless_options())
}

fn build_vello_app() -> AppBuilder {
    AppBuilder::configure::<FluentApplication>().use_harfbuzz().use_vello().use_headless(headless_options())
}

/// The window of the test: a text block, a button and a text box in a
/// column.
fn create_window() -> Ref<Window> {
    let text_block = TextBlock::new();
    text_block.set_text(Some("A text block of the Fluent theme"));
    text_block.set_margin(Thickness::uniform(8.0));

    let button = Button::new();
    let content: BoxedValue = Rc::new("A button".to_string());
    button.set_content(Some(content));
    button.set_margin(Thickness::uniform(8.0));
    button.set_horizontal_alignment(HorizontalAlignment::Left);

    let text_box = TextBox::new();
    text_box.set_text(Some("The text of a text box"));
    text_box.set_margin(Thickness::uniform(8.0));

    let panel = StackPanel::new();
    panel.set_orientation(Orientation::Vertical);
    panel.children().add(text_block);
    panel.children().add(button);
    panel.children().add(text_box);

    let window = Window::new();
    window.set_content(Some(Control::boxed(panel)));
    window.set_width(320.0);
    window.set_height(180.0);
    window
}

/// The messages of the panics of the process, of every thread: the render
/// loop catches the panic of a frame, so a member of a backend that fails
/// while a frame is drawn is found here and not by the test that waits.
static PANICS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn collect_panics() {
    static HOOK: std::sync::Once = std::sync::Once::new();

    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            PANICS.lock().unwrap_or_else(|e| e.into_inner()).push(info.to_string());
            previous(info);
        }));
    });
}

/// Shows the window in an application of the builder and returns the frame
/// it renders.
fn render_window(build: fn() -> AppBuilder) -> Pixels {
    let session = HeadlessUnitTestSession::start_new(build);

    let task = session.dispatch_result(
        || {
            let window = create_window();
            window.show();

            let frame = window.capture_rendered_frame().expect("the window renders a frame");
            let framebuffer = frame.lock();
            let size = framebuffer.size();
            let (width, height) = (size.width as usize, size.height as usize);
            let row_bytes = framebuffer.row_bytes() as usize;
            let format = framebuffer.format();
            assert!(format == PixelFormat::RGBA8888 || format == PixelFormat::BGRA8888, "{format:?}");

            let mut rgba = Vec::with_capacity(width * height * 4);
            framebuffer.with_data(&mut |data| {
                for y in 0..height {
                    for pixel in data[y * row_bytes..y * row_bytes + width * 4].chunks_exact(4) {
                        if format == PixelFormat::BGRA8888 {
                            rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
                        } else {
                            rgba.extend_from_slice(pixel);
                        }
                    }
                }
            });
            framebuffer.dispose();
            window.close();

            Pixels { width, height, rgba }
        },
        CancellationToken::none(),
    );

    let pixels = task.wait().expect("the test was cancelled");
    session.dispose();
    pixels
}

/// The headless platform with `use_vello` lays out and renders a window of
/// the Fluent theme with text, without a member of the backend failing, and
/// the frame is the one the Skia backend renders but for what the text
/// scenes measure.
#[test]
fn a_window_of_the_fluent_theme_renders_on_both_backends() {
    collect_panics();

    let skia = render_window(build_skia_app);
    let vello = render_window(build_vello_app);

    let panics = PANICS.lock().unwrap_or_else(|e| e.into_inner()).clone();
    assert!(panics.is_empty(), "a backend failed while the window was drawn:\n{}", panics.join("\n"));

    assert_eq!((320, 180), (skia.width, skia.height));
    assert_eq!((skia.width, skia.height), (vello.width, vello.height));

    let ink = |pixels: &Pixels| {
        let background = &pixels.rgba[..4];
        pixels.rgba.chunks_exact(4).filter(|pixel| pixel.iter().zip(background).any(|(a, b)| a.abs_diff(*b) > 48)).count()
    };
    let difference = compare(&skia, &vello);

    println!(
        "A window of the Fluent theme, 320 by 180 (a text block, a button, a text box): {:.3} % of the pixels beyond the tolerance, largest difference {}, mean {:.3}; pixels unlike the background: {} (Skia), {} (Vello)",
        difference.share,
        difference.largest,
        difference.mean,
        ink(&skia),
        ink(&vello)
    );

    // The text and the controls are drawn by both.
    assert!(ink(&skia) > 1500 && ink(&vello) > 1500, "{} {}", ink(&skia), ink(&vello));
    assert!(difference.share < WINDOW_BOUND, "{difference:?}");
}

/// The share of pixels of the window that may differ from the Skia backend
/// by more than the tolerance, in percent: the 1.811 % that were measured,
/// half as much again and 0.05 %, as the bounds of the scenes. The text is
/// in the default family of the system, so the number is this machine's.
const WINDOW_BOUND: f64 = 2.77;
