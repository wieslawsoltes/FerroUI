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
//! application is captured, never the desktop. On Windows
//! `FERROUI_SMOKE_RENDERING=software` or `angle` asks for that rendering
//! mode alone.
//!
//! Not ported, because the platforms and options they name do not exist
//! yet: the command line switches `--wait-for-attach`, `--fbdev`, `--vnc`,
//! `--full-headless`, `--drm`, `--dxgi` (with `--scaling`,
//! `--orientation`, `--card`), and of the application builder the data
//! annotations validation, the Wayland, X11, Vulkan and composition
//! options, the Inter font, the developer tools and the native control
//! samples of `NativeControls/` (`EmbedSample.Implementation`).

#[cfg(windows)]
mod window_capture;

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
    builder.after_setup(|_| smoke_run()).log_to_trace(LogEventLevel::Warning, &[])
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
    use ferroui_win32::{Win32CompositionMode, Win32PlatformOptions, Win32RenderingMode};

    let mut options = Win32PlatformOptions::default();
    match std::env::var("FERROUI_SMOKE_RENDERING").ok().as_deref() {
        Some("software") => options.rendering_mode = vec![Win32RenderingMode::Software],
        Some("angle") => {
            options.rendering_mode = vec![Win32RenderingMode::AngleEgl];
            options.composition_mode = vec![Win32CompositionMode::RedirectionSurface];
        }
        _ => return builder,
    }
    println!("Rendering modes: {:?}", options.rendering_mode);
    builder.with(std::rc::Rc::new(options))
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
/// different colours among a grid of samples. `None` on a system where
/// the host has no capture.
#[cfg(windows)]
fn capture_window(window: &Ref<Window>) -> Option<Result<(Vec<u8>, PixelSize, usize), String>> {
    let Some(handle) = window.try_get_platform_handle() else {
        return Some(Err("the window has no platform handle".to_string()));
    };
    Some(window_capture::capture_client_area(handle.handle()).map(|capture| {
        let colors = window_capture::sampled_colors(&capture);
        let size = PixelSize::new(capture.width, capture.height);
        (capture.pixels, size, colors)
    }))
}

#[cfg(not(windows))]
fn capture_window(_window: &Ref<Window>) -> Option<Result<(Vec<u8>, PixelSize, usize), String>> {
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
        Some(Ok((pixels, size, colors))) => {
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
                    "Screenshot of {header}: {} ({} by {}, the window as the system composed it; {colors} colour(s) among its samples)",
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
