//! Port of `Program.cs` of `ControlCatalog.Desktop`: the desktop entry
//! point of the ControlCatalog sample.
//!
//! ```text
//! cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_EXIT_MS=20000 FERROUI_SMOKE_PAGES=150 cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_SCREENSHOTS=target/screenshots cargo run -p control-catalog-desktop
//! FERROUI_CATALOG_THEME=simple cargo run -p control-catalog-desktop
//! ```
//!
//! With `FERROUI_SMOKE_EXIT_MS=<n>` the main window is closed after `n`
//! milliseconds, which ends the main loop; the process exits with the exit
//! code the lifetime returns. With `FERROUI_SMOKE_PAGES=<n>` the pages of
//! the catalog are shown one after the other, `n` milliseconds each (the
//! role of the `--full-headless` run of the managed original, which
//! selects every page).
//!
//! With `FERROUI_SMOKE_SCREENSHOTS=<directory>` pictures of some pages are
//! written to the directory as PNG files and the main window is closed
//! after the last: the pages named in `FERROUI_SMOKE_SCREENSHOT_PAGES`
//! (headers of pages, separated by commas; by default the home page and a
//! handful of others) are shown one after the other, `FERROUI_SMOKE_PAGES`
//! milliseconds each (1500 by default), and each is drawn into a bitmap
//! through the render target of the framework (`NN-<header>.png`). On
//! Windows the client area of the window is captured as well, as the
//! system composed it (`NN-<header>-window.png`); only the window of the
//! application is captured, never the desktop. The system does not draw
//! what of a window lies outside the desktop, so the run first moves and
//! shrinks the window until all of it is on its screen. On Windows
//! `FERROUI_SMOKE_RENDERING=software` or `angle` asks for that rendering
//! mode alone.
//!
//! Not ported, because the platforms and options they name do not exist
//! yet: the command line switches `--wait-for-attach`, `--fbdev`, `--vnc`,
//! `--full-headless`, `--drm`, `--dxgi` (with `--scaling`,
//! `--orientation`, `--card`), and of the application builder the data
//! annotations validation, the Wayland, X11, Vulkan and composition
//! options, the Inter font, the developer tools and the native control
//! samples of `NativeControls/` for Windows and macOS
//! (`EmbedSample.Implementation`). The one for Linux is ported
//! (`NativeControls/Gtk`).

#[cfg(windows)]
mod window_capture;

/// The native control demo of Linux (`NativeControls/Gtk` of the
/// reference).
#[cfg(target_os = "linux")]
#[path = "NativeControls/Gtk/embed_sample_gtk.rs"]
mod embed_sample_gtk;
#[cfg(target_os = "linux")]
#[path = "NativeControls/Gtk/gtk_helper.rs"]
mod gtk_helper;

use control_catalog::App;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelSize, Ref, Vector, Visual};
use ferroui_controls::{AppBuilder, Application, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let builder = build_ferro_app();

    let exit_code = builder.start_with_classic_desktop_lifetime(&args);
    // With `FERROUI_VELLO_PERF` the Vello backend says where the time of its
    // frames went.
    #[cfg(feature = "vello")]
    ferroui_vello::perf::print_summary();
    std::process::ExitCode::from(exit_code as u8)
}

/// The application builder of the catalog.
pub fn build_ferro_app() -> AppBuilder {
    let builder = smoke_platform_options(AppBuilder::configure::<App>()).use_platform_detect();
    #[cfg(feature = "vello")]
    let builder = use_renderer_of_the_environment(builder);
    builder
        .after_setup(|_| {
            #[cfg(target_os = "linux")]
            control_catalog::pages::EmbedSample::set_implementation(Some(std::rc::Rc::new(
                embed_sample_gtk::EmbedSampleGtk::default(),
            )));
            smoke_run()
        })
        .log_to_trace(LogEventLevel::Warning, &[])
}

/// The render backend asked for with `FERROUI_RENDERER`: `vello` (the modes
/// of the Vello backend in their default order), `vello-hybrid`, `vello-gpu`
/// or `vello-cpu`. Anything else, and no value, leaves the backend of the
/// platform.
#[cfg(feature = "vello")]
fn use_renderer_of_the_environment(builder: AppBuilder) -> AppBuilder {
    use ferroui_vello::{VelloApplicationExtensions, VelloOptions, VelloRenderingMode};
    let options = match std::env::var("FERROUI_RENDERER").ok().as_deref() {
        Some("vello") => VelloOptions::default(),
        Some("vello-hybrid") => VelloOptions::with_rendering_mode(VelloRenderingMode::Hybrid),
        Some("vello-gpu") => VelloOptions::with_rendering_mode(VelloRenderingMode::Gpu),
        Some("vello-cpu") => VelloOptions::with_rendering_mode(VelloRenderingMode::Cpu),
        _ => return builder,
    };
    eprintln!("ControlCatalog: the Vello backend, modes {:?}", options.rendering_mode_order());
    builder.with(std::rc::Rc::new(options)).use_vello()
}

/// The options of a smoke run on Windows, from the environment:
/// `FERROUI_SMOKE_RENDERING=software|angle` asks for that rendering mode
/// alone (no fallback, so a mode that does not work fails the run).
#[cfg(windows)]
fn smoke_platform_options(builder: AppBuilder) -> AppBuilder {
    use ferroui_win32::{Win32PlatformOptions, Win32RenderingMode};

    let mut options = Win32PlatformOptions::default();
    match std::env::var("FERROUI_SMOKE_RENDERING").ok().as_deref() {
        Some("software") => options.rendering_mode = vec![Win32RenderingMode::Software],
        Some("angle") => {
            options.rendering_mode = vec![Win32RenderingMode::AngleEgl];
            options.composition_mode = vec![smoke_composition_mode()];
        }
        _ => return builder,
    }
    println!("Rendering modes: {:?}; composition modes: {:?}", options.rendering_mode, options.composition_mode);
    builder.with(std::rc::Rc::new(options))
}

/// The composition mode of a smoke run through ANGLE, without a fallback:
/// `FERROUI_SMOKE_COMPOSITION=redirection|dcomp|winui`; the redirection surface
/// of the window when the variable is not set.
#[cfg(windows)]
fn smoke_composition_mode() -> ferroui_win32::Win32CompositionMode {
    use ferroui_win32::Win32CompositionMode;

    match std::env::var("FERROUI_SMOKE_COMPOSITION").ok().as_deref() {
        None | Some("redirection") => Win32CompositionMode::RedirectionSurface,
        Some("dcomp") => Win32CompositionMode::DirectComposition,
        Some("winui") => Win32CompositionMode::WinUIComposition,
        Some(other) => panic!("FERROUI_SMOKE_COMPOSITION: unknown mode {other:?} (redirection, dcomp, winui)"),
    }
}

#[cfg(not(windows))]
fn smoke_platform_options(builder: AppBuilder) -> AppBuilder {
    builder
}

fn environment_milliseconds(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|value| value.parse::<u64>().ok())
}

/// The main window of the application, once it exists.
fn main_window() -> Option<Ref<Window>> {
    let lifetime = Application::current().and_then(|application| application.application_lifetime());
    lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()).and_then(|l| l.main_window())
}

/// The pages a screenshot run shows when `FERROUI_SMOKE_SCREENSHOT_PAGES`
/// does not name any: the home page, buttons, a text page, a list, a page
/// with images, and two more with a variety of controls.
const SCREENSHOT_PAGES: &str = "Home,Buttons,TextBlock,ListBox,Image,Calendar,Slider";

/// The file name of the picture of a page: its number in the run and its
/// header in lower case, with everything but letters and digits a hyphen.
fn screenshot_name(index: usize, header: &str, suffix: &str) -> String {
    let header: String =
        header.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("{:02}-{header}{suffix}.png", index + 1)
}

fn png() -> BitmapEncoderOptions {
    BitmapEncoderOptions::Png(PngBitmapEncoderOptions::DEFAULT)
}

/// Draws the window into a bitmap of the size of its client area through
/// the render target of the framework and writes it as a PNG file.
fn save_frame(window: &Ref<Window>, path: &Path) -> std::io::Result<PixelSize> {
    let scaling = window.render_scaling();
    let size = window.client_size();
    let pixel_size = PixelSize::new((size.width * scaling).ceil() as i32, (size.height * scaling).ceil() as i32);
    if pixel_size.width < 1 || pixel_size.height < 1 {
        return Err(std::io::Error::other(format!("the client area of the window is empty: {pixel_size:?}")));
    }
    let bitmap = RenderTargetBitmap::with_dpi(pixel_size, Vector::new(96.0 * scaling, 96.0 * scaling));
    bitmap.render(&window.clone().upcast::<Visual>());
    let result = bitmap.save_to_file(&path.to_string_lossy(), &png());
    bitmap.dispose();
    result.map(|()| pixel_size)
}

/// The client area of the window as the system composed it: the pixels
/// (four bytes each, blue first, opaque), their size, and the number of
/// what the capture holds, in words (the number of different colours among
/// a grid of samples, and how far its content reaches). `None` on a system
/// where the host has no capture.
#[cfg(windows)]
fn capture_window(window: &Ref<Window>) -> Option<Result<(Vec<u8>, PixelSize, String), String>> {
    let Some(handle) = window.try_get_platform_handle() else {
        return Some(Err("the window has no platform handle".to_string()));
    };
    Some(window_capture::capture_client_area(handle.handle()).map(|capture| {
        let colors = window_capture::sampled_colors(&capture);
        let (right, bottom) = window_capture::content_extent(&capture);
        let size = PixelSize::new(capture.width, capture.height);
        (capture.pixels, size, format!("{colors} colour(s) among its samples, content reaches {right} by {bottom}"))
    }))
}

#[cfg(not(windows))]
fn capture_window(_window: &Ref<Window>) -> Option<Result<(Vec<u8>, PixelSize, String), String>> {
    None
}

/// Writes the pictures of the page that is shown: the frame the framework
/// draws, and where the host can capture it, the window itself.
fn save_screenshots(directory: &Path, index: usize, header: &str) {
    let Some(window) = main_window() else {
        println!("Screenshot of {header}: there is no main window");
        return;
    };

    let frame = directory.join(screenshot_name(index, header, ""));
    match save_frame(&window, &frame) {
        Ok(size) => println!(
            "Screenshot of {header}: {} ({} by {}, the frame of the framework)",
            frame.display(),
            size.width,
            size.height
        ),
        Err(error) => println!("Screenshot of {header}: the frame could not be written to {}: {error}", frame.display()),
    }

    match capture_window(&window) {
        None => {}
        Some(Err(error)) => println!("Screenshot of {header}: the window could not be captured: {error}"),
        Some(Ok((pixels, size, content))) => {
            let path = directory.join(screenshot_name(index, header, "-window"));
            let bitmap = Bitmap::from_pixels(
                PixelFormat::BGRA8888,
                AlphaFormat::Opaque,
                &pixels,
                size,
                Vector::new(96.0, 96.0),
                size.width * 4,
            );
            let result = bitmap.save_to_file(&path.to_string_lossy(), &png());
            bitmap.dispose();
            match result {
                Ok(()) => println!(
                    "Screenshot of {header}: {} ({} by {}, the window as the system composed it; {content})",
                    path.display(),
                    size.width,
                    size.height
                ),
                Err(error) => {
                    println!("Screenshot of {header}: the capture could not be written to {}: {error}", path.display())
                }
            }
        }
    }
}

/// Moves and shrinks the window so that all of it is on its screen. The
/// system does not draw the part of a window that lies outside the
/// desktop, so a capture of a window that is larger than the screen, or
/// placed over its edge, is blank there (the window of the catalog asks
/// for 1100 by 800, more than a small desktop has).
fn fit_window_to_screen(window: &Ref<Window>) {
    let Some(screen) = window.screens().screen_from_window(window) else {
        println!("Screenshots: the window is on no screen");
        return;
    };
    let area = screen.working_area();
    let scaling = window.render_scaling();
    let client = window.client_size();
    let frame = window.frame_size().unwrap_or(client);
    let position = window.position();
    let (frame_width, frame_height) = ((frame.width * scaling).ceil() as i32, (frame.height * scaling).ceil() as i32);
    println!(
        "Screenshots: the window is at {}, {} with a frame of {frame_width} by {frame_height} pixels (client {} by {} at scaling {scaling}); the working area of its screen is {area:?}",
        position.x, position.y, client.width, client.height
    );
    let fits = position.x >= area.x
        && position.y >= area.y
        && position.x + frame_width <= area.right()
        && position.y + frame_height <= area.bottom();
    if fits {
        return;
    }
    let width = client.width.min(f64::from(area.width) / scaling - (frame.width - client.width));
    let height = client.height.min(f64::from(area.height) / scaling - (frame.height - client.height));
    window.set_width(width.floor());
    window.set_height(height.floor());
    window.set_position(area.position());
    println!(
        "Screenshots: the window did not fit its screen: moved to {}, {} with a client area of {} by {}",
        area.x,
        area.y,
        width.floor(),
        height.floor()
    );
}

/// With `FERROUI_SMOKE_TRANSPARENCY=mica|acrylic|blur|transparent` the
/// pictures are of a window that asks for that transparency level, as the
/// settings page of the catalog does for a person: the hint is set, and
/// when the platform took a level the background of the window becomes a
/// grey of one fifth opacity, so that what is behind the window shows
/// through. The level the platform took is printed.
fn apply_smoke_transparency(window: &Window) {
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Colors, IBrush};
    use ferroui_controls::{WindowTransparencyLevel, WindowTransparencyLevelCollection};

    let level = match std::env::var("FERROUI_SMOKE_TRANSPARENCY").ok().as_deref() {
        None | Some("") => return,
        Some("mica") => WindowTransparencyLevel::mica(),
        Some("acrylic") => WindowTransparencyLevel::acrylic_blur(),
        Some("blur") => WindowTransparencyLevel::blur(),
        Some("transparent") => WindowTransparencyLevel::transparent(),
        Some(other) => panic!("FERROUI_SMOKE_TRANSPARENCY: unknown level {other:?} (mica, acrylic, blur, transparent)"),
    };
    window.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![level.clone()]));
    let actual = window.actual_transparency_level();
    println!("Screenshots: transparency level asked for: {level:?}; the window has: {actual:?}");
    if actual != WindowTransparencyLevel::none() {
        let brush: std::rc::Rc<dyn IBrush> = std::rc::Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::GRAY, 0.2));
        window.set_background(Some(brush));
    }
}

/// The screenshot run asked for with `FERROUI_SMOKE_SCREENSHOTS`.
fn screenshot_run(directory: PathBuf) {
    if let Err(error) = std::fs::create_dir_all(&directory) {
        println!("Screenshots: the directory {} could not be created: {error}", directory.display());
        return;
    }
    let pages = std::env::var("FERROUI_SMOKE_SCREENSHOT_PAGES").unwrap_or_else(|_| SCREENSHOT_PAGES.to_string());
    let headers: Vec<String> =
        pages.split(',').map(str::trim).filter(|header| !header.is_empty()).map(str::to_string).collect();
    let interval = Duration::from_millis(environment_milliseconds("FERROUI_SMOKE_PAGES").unwrap_or(1500));
    println!("Screenshots of {headers:?} to {}, {interval:?} a page", directory.display());
    // Before the first page is shown: the window is made to fit its screen.
    // The timer stops itself after its only tick.
    let _timer = DispatcherTimer::run_once(
        || match main_window() {
            Some(window) => {
                // With `FERROUI_SMOKE_EXTEND_CLIENT_AREA` the pictures are
                // of a window whose client area extends into its frame:
                // the title bar and the caption buttons are drawn by the
                // framework and are part of what is captured. The window
                // is fitted to its screen after that: with the caption as
                // client area the window takes the size it asks for again.
                if std::env::var_os("FERROUI_SMOKE_EXTEND_CLIENT_AREA").is_some_and(|value| !value.is_empty()) {
                    window.set_extend_client_area_to_decorations_hint(true);
                    println!(
                        "Screenshots: the client area is extended into the frame: {}",
                        window.is_extended_into_window_decorations()
                    );
                }
                fit_window_to_screen(&window);
                apply_smoke_transparency(&window);
            }
            None => println!("Screenshots: there is no main window to fit to its screen"),
        },
        interval / 2,
        DispatcherPriority::NORMAL,
    );
    control_catalog::show_pages(
        headers,
        interval,
        move |index, header| save_screenshots(&directory, index, header),
        || {
            println!("Screenshots written: closing the main window");
            if let Some(main_window) = main_window() {
                main_window.close();
            }
        },
    );
}

/// The smoke run asked for with `FERROUI_SMOKE_EXIT_MS`,
/// `FERROUI_SMOKE_PAGES` and `FERROUI_SMOKE_SCREENSHOTS`.
fn smoke_run() {
    if let Some(directory) = std::env::var_os("FERROUI_SMOKE_SCREENSHOTS").filter(|directory| !directory.is_empty()) {
        screenshot_run(PathBuf::from(directory));
    } else if let Some(ms) = environment_milliseconds("FERROUI_SMOKE_PAGES") {
        control_catalog::show_every_page(Duration::from_millis(ms));
    }

    if let Some(ms) = environment_milliseconds("FERROUI_SMOKE_EXIT_MS") {
        println!("Will close the main window after {ms} ms");
        // The timer stops itself after its only tick.
        let _timer = DispatcherTimer::run_once(
            || {
                println!("Timer fired: closing the main window");
                if let Some(main_window) = main_window() {
                    main_window.close();
                }
            },
            Duration::from_millis(ms),
            DispatcherPriority::NORMAL,
        );
    }
}
