//! Port of `Program.cs` of `ControlCatalog.Desktop`: the desktop entry
//! point of the ControlCatalog sample.
//!
//! ```text
//! cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_EXIT_MS=3000 cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_EXIT_MS=20000 FERROUI_SMOKE_PAGES=150 cargo run -p control-catalog-desktop
//! FERROUI_SMOKE_SCREENSHOTS=target/screenshots cargo run -p control-catalog-desktop
//! FERROUI_CATALOG_THEME=fluent cargo run -p control-catalog-desktop
//! FERROUI_CATALOG_THEME=simple cargo run -p control-catalog-desktop
//! FERROUI_CATALOG_WAYLAND=0 cargo run -p control-catalog-desktop
//! ```
//!
//! `FERROUI_CATALOG_THEME` selects the theme the catalog starts with: `winui`, the WinUI theme
//! of the port (docs/porting/winui-theme.md) and the default of the port, `fluent` (the default
//! of the original) or `simple`; the settings page switches between them as before. Not in the
//! original.
//!
//! On Linux the catalog uses the Wayland backend when a compositor can be
//! reached, with the backend of the platform detection (X11) as its
//! fallback, as the original does (`UseWaylandWithFallback`).
//! `FERROUI_CATALOG_WAYLAND=0` (or `x11`) leaves the backend of the platform
//! detection alone; `software` renders through shared memory buffers alone;
//! `dmabuf` asks for the dmabuf swapchain of the backend.
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
//! through the render target of the framework (`NN-<header>.png`, drawn by
//! the immediate renderer into a render target bitmap) and drawn by the
//! compositor of the window as it draws the frames it presents
//! (`NN-<header>-composed.png`, a snapshot of the composition visual of the
//! window at its render scaling). On
//! a screenshot run `FERROUI_SMOKE_OPEN_POPUP=combobox` (or `menu`) opens
//! the first combo box (or the first menu item) of each page that is shown,
//! so that a picture taken from outside (the compositor's, on Wayland) has a
//! popup in it; the frame the framework draws of the window does not;
//! `FERROUI_SMOKE_OPEN_POPUP=textbox` gives the first text box of each page
//! the focus instead, and prints its text whenever it changed, so that an
//! input method of the session can type into it. With
//! `FERROUI_SMOKE_DRAWN_DECORATIONS` the window asks once for less than
//! full decorations before the first page, after which a Wayland compositor
//! leaves the decorations to the framework. On
//! Windows the client area of the window is captured as well, as the
//! system composed it (`NN-<header>-window.png`); only the window of the
//! application is captured, never the desktop. The system does not draw
//! what of a window lies outside the desktop, so the run first moves and
//! shrinks the window until all of it is on its screen. On Windows
//! `FERROUI_SMOKE_RENDERING=software` or `angle` asks for that rendering
//! mode alone (`vulkan` too, in a build with the feature `vulkan`; on macOS `opengl`, `metal` or `software`; on Linux `glx`, `egl` or `software` of the X11 backend), `FERROUI_SMOKE_COMPOSITION` for a composition mode, and
//! `FERROUI_SMOKE_TRANSPARENCY=mica|acrylic|blur|transparent` makes the
//! window ask for that transparency level. With
//! `FERROUI_SMOKE_BACKDROP=<directory>` and a transparency level the run
//! is the backdrop run instead (`backdrop_run`): a second window with a
//! transparent background over the main window, and the screen under it
//! captured with the level `transparent` and with the level asked for.
//! That run reads the screen under its own window: it is for a machine
//! whose screen holds nothing but the run.
//!
//! With the command line switch `--vnc` the catalog runs on the headless
//! platform, without a window on the screen, and a VNC server on port 5901
//! of the loopback address shows its main window to a VNC viewer and passes
//! the keyboard and the mouse of the viewer to it; the application ends
//! when the main window closes.
//!
//! ```text
//! cargo run -p control-catalog-desktop -- --vnc
//! ```
//!
//! Not ported, because the platforms and options they name do not exist
//! yet: the command line switches `--wait-for-attach`,
//! `--full-headless`, `--dxgi`, and of the application builder the data
//! annotations validation, the X11, Vulkan and composition
//! options, the Inter font, the developer tools and the native control
//! samples of `NativeControls/` for Windows and macOS
//! (`EmbedSample.Implementation`). The one for Linux is ported
//! (`NativeControls/Gtk`).

#[cfg(feature = "mcp")]
mod mcp;

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

#[cfg(target_os = "linux")]
mod linux_framebuffer;

use control_catalog::App;
use ferroui_base::logging::LogEventLevel;
use ferroui_base::media::imaging::{Bitmap, BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{PixelPoint, PixelRect, PixelSize, Ref, Size, Vector, Visual};
use ferroui_controls::{AppBuilder, Application, ComboBox, MenuItem, ShutdownMode, TextBox, Window, WindowDecorations};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_headless_vnc::HeadlessVncPlatformExtensions;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // Reserve the protocol writer before builder configuration can print platform diagnostics.
    #[cfg(feature = "mcp")]
    let mcp_output = args.iter().any(|arg| arg == "--mcp-stdio")
        .then(|| ferroui_mcp::reserve_stdout().expect("reserve stdout for MCP"));
    let builder = build_ferro_app();
    #[cfg(feature = "mcp")]
    let builder = mcp::configure(builder, &args, mcp_output);

    // As the desktop entry point of the reference: `--fbdev` and `--drm`
    // run the catalog on the Linux framebuffer platform, on a framebuffer
    // device and on a KMS output.
    #[cfg(target_os = "linux")]
    if args.iter().any(|arg| arg == "--fbdev") {
        let exit_code = linux_framebuffer::start(builder, &args);
        return std::process::ExitCode::from(exit_code as u8);
    }
    #[cfg(target_os = "linux")]
    if args.iter().any(|arg| arg == "--drm") {
        let exit_code = linux_framebuffer::start_drm(builder, &args);
        return std::process::ExitCode::from(exit_code as u8);
    }
    if args.iter().any(|arg| arg == "--vnc") {
        let exit_code = builder.start_with_headless_vnc_platform(None, 5901, &args, ShutdownMode::OnMainWindowClose);
        return std::process::ExitCode::from(exit_code as u8);
    }

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
    #[cfg(target_os = "linux")]
    let builder = use_wayland_of_the_environment(builder);
    #[cfg(feature = "vello")]
    let builder = use_renderer_of_the_environment(builder);
    builder
        .after_setup(|_| {
            #[cfg(target_os = "linux")]
            control_catalog::pages::EmbedSample::set_implementation(Some(std::rc::Rc::new(
                embed_sample_gtk::EmbedSampleGtk::default(),
            )));
            select_theme_of_the_environment();
            smoke_run();
            #[cfg(feature = "cdp")]
            start_cdp();
        })
        .after_setup(attach_dev_tools)
        .log_to_trace(LogEventLevel::Warning, &[])
}

/// DevTools on F12 in debug builds, as the reference (`AttachDevTools` with the startup screen 1),
/// but on the framebuffer platform, whose lifetime has no windows.
fn attach_dev_tools(builder: &AppBuilder) {
    #[cfg(debug_assertions)]
    {
        use ferroui_diagnostics::diagnostics::DevToolsOptions;
        use ferroui_diagnostics::DevToolsExtensions;

        if std::env::args().any(|arg| arg == "--fbdev" || arg == "--drm") {
            return;
        }
        if let Some(application) = builder.instance() {
            let options = DevToolsOptions::new().with_startup_screen_index(Some(1));
            // The pictures of a smoke run: a window large enough for every column of the grid.
            let options = match std::env::var_os("FERROUI_SMOKE_DEVTOOLS") {
                Some(_) => options.with_startup_screen_index(None).with_size(ferroui_base::Size::new(1600.0, 900.0)),
                None => options,
            };
            application.attach_dev_tools_with_options(options);
        }
    }
    let _ = builder;
}

/// The theme of the catalog asked for with `FERROUI_CATALOG_THEME` (`winui`, the default, `fluent`
/// or `simple`), selected after the application is set up and before its main window is created.
fn select_theme_of_the_environment() {
    use control_catalog::models::CatalogTheme;

    let theme = match std::env::var("FERROUI_CATALOG_THEME").ok().as_deref().map(str::to_ascii_lowercase).as_deref() {
        None | Some("") | Some("winui") => return,
        Some("fluent") => CatalogTheme::Fluent,
        Some("simple") => CatalogTheme::Simple,
        Some(other) => panic!("FERROUI_CATALOG_THEME: unknown theme {other:?} (fluent, simple, winui)"),
    };
    println!("ControlCatalog: the theme {theme:?}");
    App::select_initial_catalog_theme(theme);
}

/// The Wayland backend with the backend of the platform detection as its fallback, as the
/// original (`UseWaylandWithFallback()` after `UsePlatformDetect()`): with a compositor that
/// cannot be reached or used the fallback takes over.
///
/// `FERROUI_CATALOG_WAYLAND` is the port's own, for the runs that need one backend: `0` (or
/// `x11`) leaves the backend of the platform detection alone, which is how the catalog is run
/// through XWayland on a Wayland session; `software` renders through shared memory buffers
/// alone; `dmabuf` asks for the dmabuf swapchain (`use_dmabuf_swapchain`), which the backend
/// uses when the compositor has `zwp_linux_dmabuf_v1`; anything else, and no value, renders as
/// the options of the backend say (EGL when the compositor and the driver have it).
#[cfg(target_os = "linux")]
fn use_wayland_of_the_environment(builder: AppBuilder) -> AppBuilder {
    use ferroui_wayland::{FerroWaylandPlatformExtensions, WaylandPlatformOptions};
    let mut options = WaylandPlatformOptions::new();
    match std::env::var("FERROUI_CATALOG_WAYLAND").ok().as_deref() {
        Some("0") | Some("x11") => return builder,
        // No profile to make a context with: no display of EGL is created.
        Some("software") => options.gl_profiles = Vec::new(),
        Some("dmabuf") => options.use_dmabuf_swapchain = Some(true),
        _ => {}
    }
    println!("ControlCatalog: the Wayland backend, with the backend of the platform detection as its fallback");
    builder.with(std::rc::Rc::new(options)).use_wayland_with_fallback()
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
/// `FERROUI_SMOKE_RENDERING=software|angle|wgl|vulkan` asks for that rendering
/// mode alone (no fallback, so a mode that does not work fails the run;
/// `vulkan` needs the feature `vulkan`, which builds Skia from source).
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
        // The OpenGL of the system, with software rendering behind it: a system without a
        // driver for it (the generic implementation is OpenGL 1.1) passes the mode over.
        Some("wgl") => options.rendering_mode = vec![Win32RenderingMode::Wgl, Win32RenderingMode::Software],
        Some("vulkan") => options.rendering_mode = vec![Win32RenderingMode::Vulkan],
        _ => return builder,
    }
    println!("Rendering modes: {:?}; composition modes: {:?}", options.rendering_mode, options.composition_mode);
    builder.with(std::rc::Rc::new(options))
}

/// The composition mode of a smoke run through ANGLE, without a fallback:
/// `FERROUI_SMOKE_COMPOSITION=redirection|dcomp|winui|dxgi`; the redirection surface
/// of the window when the variable is not set.
#[cfg(windows)]
fn smoke_composition_mode() -> ferroui_win32::Win32CompositionMode {
    use ferroui_win32::Win32CompositionMode;

    match std::env::var("FERROUI_SMOKE_COMPOSITION").ok().as_deref() {
        None | Some("redirection") => Win32CompositionMode::RedirectionSurface,
        Some("dcomp") => Win32CompositionMode::DirectComposition,
        Some("winui") => Win32CompositionMode::WinUIComposition,
        Some("dxgi") => Win32CompositionMode::LowLatencyDxgiSwapChain,
        Some(other) => panic!("FERROUI_SMOKE_COMPOSITION: unknown mode {other:?} (redirection, dcomp, winui, dxgi)"),
    }
}

/// The options of a smoke run on macOS, from the environment:
/// `FERROUI_SMOKE_RENDERING=opengl|metal|software` asks for that rendering
/// mode alone (no fallback, so a mode that does not work fails the run).
/// `opengl` needs the Skia backend built with Ganesh on OpenGL
/// (`RUSTFLAGS="--cfg ferro_skia_apple_ganesh"`, docs/porting/apple-opengl.md).
#[cfg(target_os = "macos")]
fn smoke_platform_options(builder: AppBuilder) -> AppBuilder {
    use ferroui_native::{FerroNativePlatformOptions, FerroNativeRenderingMode};

    let mut options = FerroNativePlatformOptions::default();
    match std::env::var("FERROUI_SMOKE_RENDERING").ok().as_deref() {
        Some("opengl") => options.rendering_mode = vec![FerroNativeRenderingMode::OpenGl],
        Some("metal") => options.rendering_mode = vec![FerroNativeRenderingMode::Metal],
        Some("software") => options.rendering_mode = vec![FerroNativeRenderingMode::Software],
        _ => return builder,
    }
    println!("Rendering modes: {:?}", options.rendering_mode);
    builder.with(std::rc::Rc::new(options)).after_setup(|_| {
        // Which graphics the platform applied: the OpenGL graphics are the
        // ones with a shared context, the Metal ones have none.
        use ferroui_base::platform::IPlatformGraphics;
        use ferroui_base::{FerroLocator, LocatorExtensions};
        let graphics = FerroLocator::current().get_service::<std::sync::Arc<dyn IPlatformGraphics>>();
        match graphics.as_deref() {
            Some(graphics) if graphics.uses_shared_context() => println!("Platform graphics: OpenGL (CGL)"),
            Some(_) => println!("Platform graphics: Metal"),
            None => println!("Platform graphics: none (software)"),
        }
    })
}

/// The options of a smoke run on the X11 backend, from the environment:
/// `FERROUI_SMOKE_RENDERING=glx|egl|software` asks for that rendering mode
/// alone (no fallback, so a mode that does not work fails the run).
#[cfg(target_os = "linux")]
fn smoke_platform_options(builder: AppBuilder) -> AppBuilder {
    use ferroui_x11::{X11PlatformOptions, X11RenderingMode};

    let mut options = X11PlatformOptions::new();
    match std::env::var("FERROUI_SMOKE_RENDERING").ok().as_deref() {
        Some("glx") => options.rendering_mode = vec![X11RenderingMode::Glx],
        Some("egl") => options.rendering_mode = vec![X11RenderingMode::Egl],
        Some("software") => options.rendering_mode = vec![X11RenderingMode::Software],
        _ => return builder,
    }
    println!("Rendering modes: {:?}", options.rendering_mode);
    builder.with(std::rc::Rc::new(options))
}

#[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
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

/// Writes the window as its compositor draws it: a snapshot of the
/// composition visual of the window at its render scaling, which the render
/// thread draws from the same server visuals, through the same drawing code,
/// as the frames it presents in the window (the bitmap of `save_frame` is
/// drawn by the immediate renderer instead). The picture is written when
/// the render thread has drawn it, after this returns.
fn save_composed(window: &Ref<Window>, path: PathBuf, header: String) {
    let Some(visual) = ElementComposition::get_element_visual(window) else {
        println!("Screenshot of {header}: the window has no composition visual");
        return;
    };
    let compositor = visual.compositor();
    let task = compositor.create_composition_visual_snapshot(&visual, window.render_scaling());
    // The job goes with the next batch: committed now, it draws the page
    // that is shown, before the run selects the next one.
    compositor.commit();
    let pending = task.clone();
    task.on_completed(move || match pending.take_result() {
        Some(Ok(bitmap)) => match bitmap.save_to_file(&path.to_string_lossy(), &png()) {
            Ok(()) => {
                let size = bitmap.pixel_size();
                println!(
                    "Screenshot of {header}: {} ({} by {}, the frame of the compositor)",
                    path.display(),
                    size.width,
                    size.height
                );
            }
            Err(error) => {
                println!("Screenshot of {header}: the snapshot could not be written to {}: {error}", path.display())
            }
        },
        Some(Err(error)) => println!("Screenshot of {header}: the compositor made no snapshot: {error}"),
        None => {}
    });
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

    save_composed(&window, directory.join(screenshot_name(index, header, "-composed")), header.to_string());

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
    let scaling = window.desktop_scaling();
    let client = window.client_size();
    let frame = window.frame_size().unwrap_or(client);
    let position = window.position();
    println!(
        "Screenshots: the window is at {}, {} with a frame of {} by {} and a client area of {} by {} (render scaling {}); the working area of its screen is {area:?}, in units of which a unit of the window is {scaling}",
        position.x,
        position.y,
        frame.width,
        frame.height,
        client.width,
        client.height,
        window.render_scaling()
    );
    let Some(fitted) = client_size_that_fits(area, position, client, frame, scaling) else {
        return;
    };
    window.set_width(fitted.width);
    window.set_height(fitted.height);
    window.set_position(area.position());
    println!(
        "Screenshots: the window did not fit its screen: moved to {}, {} with a client area of {} by {}",
        area.x, area.y, fitted.width, fitted.height
    );
}

/// The client size with which a window fits a working area once it is
/// moved to the corner of the area, or `None` when the window lies within
/// the area already.
///
/// `area` and `position` are in the units of the desktop (what the
/// platform places windows and reports screens in), `client` and `frame`
/// in the units of the window, and `desktop_scaling` is how many units of
/// the desktop one unit of the window is (`Window::desktop_scaling`). It
/// is not the scaling the window renders with: where the system works in
/// physical pixels the two are the same (Windows, X11), and where the
/// system works in logical units it is 1 whatever the window renders with
/// (Wayland, macOS).
fn client_size_that_fits(
    area: PixelRect,
    position: PixelPoint,
    client: Size,
    frame: Size,
    desktop_scaling: f64,
) -> Option<Size> {
    let (frame_width, frame_height) =
        ((frame.width * desktop_scaling).ceil() as i32, (frame.height * desktop_scaling).ceil() as i32);
    let fits = position.x >= area.x
        && position.y >= area.y
        && position.x + frame_width <= area.right()
        && position.y + frame_height <= area.bottom();
    if fits {
        return None;
    }
    let width = client.width.min(f64::from(area.width) / desktop_scaling - (frame.width - client.width));
    let height = client.height.min(f64::from(area.height) / desktop_scaling - (frame.height - client.height));
    Some(Size::new(width.floor(), height.floor()))
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

    let Some(level) = smoke_transparency_level() else {
        return;
    };
    window.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![level.clone()]));
    let actual = window.actual_transparency_level();
    println!("Screenshots: transparency level asked for: {level:?}; the window has: {actual:?}");
    if actual != WindowTransparencyLevel::none() {
        let brush: std::rc::Rc<dyn IBrush> = std::rc::Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::GRAY, 0.2));
        window.set_background(Some(brush));
    }
}

/// The transparency level `FERROUI_SMOKE_TRANSPARENCY` names; `None` when
/// the variable is not set.
fn smoke_transparency_level() -> Option<ferroui_controls::WindowTransparencyLevel> {
    use ferroui_controls::WindowTransparencyLevel;

    match std::env::var("FERROUI_SMOKE_TRANSPARENCY").ok().as_deref() {
        None | Some("") => None,
        Some("mica") => Some(WindowTransparencyLevel::mica()),
        Some("acrylic") => Some(WindowTransparencyLevel::acrylic_blur()),
        Some("blur") => Some(WindowTransparencyLevel::blur()),
        Some("transparent") => Some(WindowTransparencyLevel::transparent()),
        Some(other) => panic!("FERROUI_SMOKE_TRANSPARENCY: unknown level {other:?} (mica, acrylic, blur, transparent)"),
    }
}

/// The share of the pixels of two captures of one size that differ by more
/// than a few steps in a channel, in percent.
#[cfg_attr(not(windows), allow(dead_code))]
fn changed_percent(first: &[u8], second: &[u8]) -> f64 {
    let pixels = first.len().min(second.len()) / 4;
    if pixels == 0 {
        return 0.0;
    }
    let changed = first
        .chunks_exact(4)
        .zip(second.chunks_exact(4))
        .filter(|(a, b)| (0..3).any(|channel| (i32::from(a[channel]) - i32::from(b[channel])).abs() > 8))
        .count();
    changed as f64 * 100.0 / pixels as f64
}

/// The backdrop run asked for with `FERROUI_SMOKE_BACKDROP=<directory>` and
/// `FERROUI_SMOKE_TRANSPARENCY=<level>`: whether the backdrop effect of a
/// transparency level shows on the screen.
///
/// A second window with a transparent background and one line of text is
/// shown over the main window of the catalog (whose page is what lies
/// behind it), topmost. It first asks for the level `transparent`, and
/// the rectangle of the screen under its client area is captured: the
/// page behind shows through as it is. Then it asks for the level of the
/// run and the rectangle is captured again: with a backdrop effect what
/// is behind the window is blurred (acrylic) or replaced by the tinted
/// wallpaper (mica), so the two captures differ. Both captures and the
/// window as it prints itself are written as pictures, and the run says
/// how much of the rectangle changed. It reads the screen, under the
/// rectangle of its own window only: it is for a machine whose screen
/// holds nothing but the run.
#[cfg(windows)]
fn backdrop_run(directory: PathBuf) {
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Colors, IBrush};
    use ferroui_base::PixelPoint;
    use ferroui_controls::{Control, TextBlock, WindowTransparencyLevel, WindowTransparencyLevelCollection};
    use std::rc::Rc;

    fn save(path: &Path, capture: &window_capture::Capture) {
        let size = PixelSize::new(capture.width, capture.height);
        let bitmap = Bitmap::from_pixels(
            PixelFormat::BGRA8888,
            AlphaFormat::Opaque,
            &capture.pixels,
            size,
            Vector::new(96.0, 96.0),
            size.width * 4,
        );
        match bitmap.save_to_file(&path.to_string_lossy(), &png()) {
            Ok(()) => println!(
                "Backdrop: {} ({} by {}, {} colour(s) among its samples)",
                path.display(),
                size.width,
                size.height,
                window_capture::sampled_colors(capture)
            ),
            Err(error) => println!("Backdrop: {} could not be written: {error}", path.display()),
        }
        bitmap.dispose();
    }

    fn finish() {
        println!("Backdrop: closing the main window");
        if let Some(main_window) = main_window() {
            main_window.close();
        }
    }

    if let Err(error) = std::fs::create_dir_all(&directory) {
        println!("Backdrop: the directory {} could not be created: {error}", directory.display());
        return;
    }
    let Some(level) = smoke_transparency_level() else {
        println!("Backdrop: FERROUI_SMOKE_TRANSPARENCY names no level");
        return;
    };
    let name = std::env::var("FERROUI_SMOKE_TRANSPARENCY").unwrap_or_default();
    let step = Duration::from_millis(environment_milliseconds("FERROUI_SMOKE_PAGES").unwrap_or(2500));
    println!("Backdrop run for the level {level:?} to {}, {step:?} a step", directory.display());

    let _timer = DispatcherTimer::run_once(
        move || {
            // The steps are closures a timer may call again: each hands
            // copies of what the next one needs on.
            let (directory, name) = (directory.clone(), name.clone());
            let Some(main_window) = main_window() else {
                println!("Backdrop: there is no main window");
                return;
            };
            fit_window_to_screen(&main_window);

            let probe = Window::new();
            probe.set_title(Some("Backdrop".to_string()));
            probe.set_width(420.0);
            probe.set_height(260.0);
            probe.set_topmost(true);
            let transparent: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::new(Colors::TRANSPARENT));
            probe.set_background(Some(transparent));
            let text = TextBlock::new();
            text.set_text(Some("A window with a transparent background"));
            probe.set_content(Some(Control::boxed(text)));
            probe.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![WindowTransparencyLevel::transparent()]));
            probe.show();
            probe.set_position(PixelPoint::new(160, 140));
            println!("Backdrop: the window is shown with the level {:?}", probe.actual_transparency_level());

            let _timer = DispatcherTimer::run_once(
                move || {
                    let Some(handle) = probe.try_get_platform_handle() else {
                        println!("Backdrop: the window has no platform handle");
                        return finish();
                    };
                    let hwnd = handle.handle();
                    let before = window_capture::capture_client_area_from_screen(hwnd);
                    match &before {
                        Ok(capture) => save(&directory.join("backdrop-transparent-screen.png"), capture),
                        Err(error) => println!("Backdrop: the screen under the window could not be captured: {error}"),
                    }

                    probe.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![level]));
                    let actual = probe.actual_transparency_level();
                    println!("Backdrop: transparency level asked for: {level:?}; the window has: {actual:?}");
                    let (directory, name, probe) = (directory.clone(), name.clone(), probe.clone());

                    let _timer = DispatcherTimer::run_once(
                        move || {
                            let after = window_capture::capture_client_area_from_screen(hwnd);
                            match &after {
                                Ok(capture) => save(&directory.join(format!("backdrop-{name}-screen.png")), capture),
                                Err(error) => println!("Backdrop: the screen under the window could not be captured: {error}"),
                            }
                            match window_capture::capture_client_area(hwnd) {
                                Ok(capture) => save(&directory.join(format!("backdrop-{name}-window.png")), &capture),
                                Err(error) => println!("Backdrop: the window could not be captured: {error}"),
                            }
                            if let (Ok(before), Ok(after)) = (&before, &after) {
                                let changed = changed_percent(&before.pixels, &after.pixels);
                                let shows = actual == level && changed >= 20.0;
                                println!(
                                    "Backdrop check: {} {changed:.1} % of the screen under the window changed between the level transparent and the level {name} (the window has {actual:?}){}",
                                    if shows { "[ ok ]" } else { "[info]" },
                                    if shows { ": the backdrop effect shows" } else { ": no backdrop effect shows on this screen" }
                                );
                            }
                            probe.close();
                            finish();
                        },
                        step,
                        DispatcherPriority::NORMAL,
                    );
                },
                step,
                DispatcherPriority::NORMAL,
            );
        },
        step,
        DispatcherPriority::NORMAL,
    );
}

#[cfg(not(windows))]
fn backdrop_run(_directory: PathBuf) {
    println!("Backdrop: the host has no capture of the screen on this system");
}

/// With `FERROUI_SMOKE_OPEN_POPUP` in a screenshot run: looks at the window four times a
/// page and opens the first combo box (`combobox`, or `1`) or the first menu item (`menu`)
/// that was not there at the look before, which is the first of the page that was just
/// shown. The first look only remembers what the window has by itself. The control is opened
/// once it is at the same place at two looks: a page comes in with a transition that slides
/// its content, and a popup takes its place from where its control is when it opens (the
/// first pictures had the popups at the edge of the output the page came in from).
///
/// With `textbox` the first text box of the page is given the focus instead, and every look
/// after that prints its text when it changed: what an input method committed to it. A page
/// that is a gallery of samples has no text box of its own: there the first card is pressed
/// once, and the first text box of the sample it opens takes the focus. The looks are twelve
/// a page in this mode, so that both steps fit before the picture.
fn open_popups_of_pages(interval: Duration) {
    let (menu, text_box) = match std::env::var("FERROUI_SMOKE_OPEN_POPUP").ok().as_deref() {
        Some("menu") => (true, false),
        Some("combobox") | Some("1") => (false, false),
        Some("textbox") => (false, true),
        _ => return,
    };
    let focused: std::cell::RefCell<Option<(Ref<TextBox>, Option<String>)>> = std::cell::RefCell::new(None);
    let card_pressed = std::cell::Cell::new(false);
    let looks = if text_box { 12 } else { 4 };
    let seen: std::cell::RefCell<Option<Vec<Ref<Visual>>>> = std::cell::RefCell::new(None);
    let opened: std::cell::RefCell<Option<Ref<Visual>>> = std::cell::RefCell::new(None);
    let candidate: std::cell::RefCell<Option<(Ref<Visual>, Option<ferroui_base::Point>)>> = std::cell::RefCell::new(None);
    let timer = DispatcherTimer::run(
        move || {
            let Some(window) = main_window() else { return true };
            // One look after a popup was opened: where the platform put it.
            if let Some(control) = opened.borrow_mut().take() {
                if !text_box {
                    report_popup_placement(&window, &control);
                }
            }
            // The text of the text box that has the focus, when it changed.
            if let Some((focused, last)) = focused.borrow_mut().as_mut() {
                let text = focused.text();
                if text != *last {
                    println!("Text input: the focused text box has the text {:?}", text.clone().unwrap_or_default());
                    *last = text;
                }
            }
            let candidates: Vec<Ref<Visual>> = window
                .clone()
                .upcast::<Visual>()
                .get_visual_descendants()
                .filter(|visual| {
                    if text_box {
                        visual.cast::<TextBox>().is_some() || visual.cast::<ferroui_controls::Button>().is_some()
                    } else if menu {
                        visual.cast::<MenuItem>().is_some()
                    } else {
                        visual.cast::<ComboBox>().is_some()
                    }
                })
                .collect();
            let mut seen = seen.borrow_mut();
            let first_look = seen.is_none();
            let known = seen.get_or_insert_with(Vec::new);
            let new: Vec<Ref<Visual>> =
                candidates.into_iter().filter(|visual| !known.iter().any(|seen| seen.ptr_eq(visual))).collect();
            known.extend(new.iter().cloned());
            if first_look {
                return true;
            }
            // A text box before a card; a card only once, and only while no text box has the focus.
            let pick = if text_box {
                let text_boxes = new.iter().find(|visual| visual.cast::<TextBox>().is_some() && visual.is_effectively_visible());
                let cards = || {
                    new.iter().find(|visual| {
                        !card_pressed.get()
                            && focused.borrow().is_none()
                            && visual.cast::<TextBox>().is_none()
                            && visual.is_effectively_visible()
                            // A card of a gallery, not a button of the navigation.
                            && visual.get_visual_ancestors().any(|ancestor| ancestor.cast::<control_catalog::controls::CardGrid>().is_some())
                    })
                };
                if focused.borrow().is_some() { None } else { text_boxes.or_else(cards).cloned() }
            } else {
                new.into_iter().find(|visual| visual.is_effectively_visible())
            };
            if let Some(first) = pick {
                let window_visual = window.clone().upcast::<Visual>();
                println!(
                    "Screenshots: a control to open appeared at {:?} of the window",
                    first.translate_point(ferroui_base::Point::new(0.0, 0.0), &window_visual)
                );
                *candidate.borrow_mut() = Some((first, None));
                return true;
            }
            let window_visual = window.clone().upcast::<Visual>();
            let settled = {
                let mut candidate = candidate.borrow_mut();
                match candidate.as_mut() {
                    Some((control, last)) => {
                        let now = control.translate_point(ferroui_base::Point::new(0.0, 0.0), &window_visual);
                        if now.is_some() && now == *last {
                            println!("Screenshots: the control stays at {now:?} of the window");
                            true
                        } else {
                            *last = now;
                            false
                        }
                    }
                    None => false,
                }
            };
            let first = if settled { candidate.borrow_mut().take().map(|(control, _)| control) } else { None };
            if let Some(first) = first {
                if let Some(combo_box) = first.cast::<ComboBox>() {
                    println!("Screenshots: opening the drop-down of a combo box");
                    combo_box.set_is_drop_down_open(true);
                } else if let Some(menu_item) = first.cast::<MenuItem>() {
                    println!("Screenshots: opening a menu item");
                    menu_item.open();
                } else if let Some(text_box) = first.cast::<TextBox>() {
                    println!("Screenshots: giving a text box the focus: {}", text_box.focus());
                    *focused.borrow_mut() = Some((text_box.clone(), text_box.text()));
                } else if let Some(card) = first.cast::<ferroui_controls::Button>() {
                    println!("Screenshots: pressing the first card of a gallery");
                    card_pressed.set(true);
                    <ferroui_controls::Button as ferroui_controls::ButtonImpl>::on_click(&card);
                }
                *opened.borrow_mut() = Some(first);
            }
            true
        },
        interval / looks,
        DispatcherPriority::BACKGROUND,
    );
    // The timer lives as long as the run.
    std::mem::forget(timer);
}

/// Says where the popup of a control that was just opened is, on a platform that can tell: on
/// the Wayland backend the compositor places a popup, and the line compares the place it
/// reported (relative to the window geometry of the window) with the control the popup
/// belongs to. A popup that is not below its control, at its left edge or centred under it,
/// is a line with `[FAILED]`.
#[cfg(target_os = "linux")]
fn report_popup_placement(window: &Ref<Window>, control: &Ref<Visual>) {
    use ferroui_base::{FerroLocator, LocatorExtensions, Point};
    use ferroui_wayland::server::wayland_worker_client::WaylandWorkerClient;
    let Some(client) = FerroLocator::current().get_service::<WaylandWorkerClient>() else {
        return;
    };
    // The place of the control on the surface of the window: against the root of its visual
    // tree, which is the window itself or, where the framework draws the decorations (a
    // compositor without a decoration manager: Mutter), what holds the title bar above it. The
    // first run under Mutter compared with the place in the window and was off by the title bar.
    let window_visual = window.clone().upcast::<Visual>();
    let root = control.get_visual_ancestors().last().unwrap_or(window_visual);
    let Some(origin) = control.translate_point(Point::new(0.0, 0.0), &root) else {
        println!("Popup placement: [FAILED] the control is not in the window");
        return;
    };
    let size = control.bounds().size();
    let placements = client
        .invoke_oob(|worker| {
            let state = &worker.state;
            // The popups whose parent is a window: a popup of a popup is placed against its parent.
            state
                .popups
                .values()
                .filter(|popup| state.top_levels.contains_key(&popup.parent()))
                .map(|popup| popup.placement())
                .collect::<Vec<_>>()
        })
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_default();
    if placements.is_empty() {
        println!("Popup placement: [FAILED] the worker has no popup of the window; the control is at {origin:?} with {size:?}");
        return;
    }
    for placement in placements {
        let (left, top) = placement.parent_geometry.map_or((0, 0), |(left, top, _, _)| (left, top));
        // The popup's window geometry of the last xdg_popup.configure (none for a bare
        // xdg_surface.configure).
        let verdict = match placement.configure.as_ref().and_then(|configure| configure.geometry) {
            Some(geometry) => {
                let x = f64::from(geometry.x + left);
                let y = f64::from(geometry.y + top);
                let width = f64::from(geometry.width);
                let below = (y - (origin.y + size.height)).abs() <= 12.0;
                let centred = (x + width / 2.0 - (origin.x + size.width / 2.0)).abs() <= 12.0;
                let left_aligned = (x - origin.x).abs() <= 12.0;
                below && (centred || left_aligned)
            }
            None => false,
        };
        println!(
            "Popup placement: [{}] the control is at ({}, {}) with {} by {} on the surface of the window; the positioner was {:?} against the window geometry {:?}; the compositor configured {:?}",
            if verdict { "ok" } else { "FAILED" },
            origin.x,
            origin.y,
            size.width,
            size.height,
            (placement.positioner, placement.requested_anchor_rect),
            placement.parent_geometry,
            placement.configure
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn report_popup_placement(_window: &Ref<Window>, _control: &Ref<Visual>) {}

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
    open_popups_of_pages(interval);
    if std::env::var_os("FERROUI_SMOKE_WINUI_FLIGHT").is_some() {
        // The first page is selected one interval after the start.
        winui_flight_run(&directory, interval);
    }
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
                // With `FERROUI_SMOKE_DRAWN_DECORATIONS` the window asks once for less
                // than full decorations and then for full ones again. On Wayland that
                // ends the decorations of the compositor for the window for good, and
                // the framework draws the title bar, the border and the shadow.
                if std::env::var_os("FERROUI_SMOKE_DRAWN_DECORATIONS").is_some_and(|value| !value.is_empty()) {
                    window.set_window_decorations(WindowDecorations::BorderOnly);
                    window.set_window_decorations(WindowDecorations::Full);
                    println!("Screenshots: the window asked for less than full decorations once");
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

/// With `FERROUI_SMOKE_WINUI_FLIGHT` on a screenshot run whose first page is "Composition
/// (WinUI)": the first card is opened, which starts its connected animation, and the frames of
/// the compositor are saved during the flight to the detail view and during the flight back
/// (`flight-NN-<ms>.png`, the time from the selection of the page). Not in the original. The
/// page should be shown for 5 s at least (`FERROUI_SMOKE_PAGES=5000`). The box of the implicit
/// animations is then hidden and shown and the parallax scroller scrolled (`steps-<ms>.png`).
fn winui_flight_run(directory: &std::path::Path, selected_at: Duration) {
    let open_at = selected_at + Duration::from_millis(400);
    let back_at = selected_at + Duration::from_millis(2000);
    let _ = DispatcherTimer::run_once(
        || {
            // `FERROUI_SMOKE_WINUI_FLIGHT=1` (direct) or `2` (basic) chooses the configuration;
            // any other value keeps the gravity one.
            let configuration = std::env::var("FERROUI_SMOKE_WINUI_FLIGHT").ok().and_then(|value| value.parse().ok());
            if let Some(configuration @ 1..=2) = configuration {
                control_catalog::pages::smoke_set_configuration(configuration);
            }
            println!("Flight: open the first card: {}", control_catalog::pages::smoke_open_card(0));
        },
        open_at,
        DispatcherPriority::NORMAL,
    );
    let _ = DispatcherTimer::run_once(
        || {
            println!("Flight: back to the list: {}", control_catalog::pages::smoke_close_card());
        },
        back_at,
        DispatcherPriority::NORMAL,
    );
    // Then the implicit hide and show animations of the box, and the parallax header.
    let steps: [(u64, fn() -> bool, &str); 3] = [
        (2900, control_catalog::pages::smoke_toggle_box, "hide the box"),
        (3600, control_catalog::pages::smoke_toggle_box, "show the box"),
        (4300, || control_catalog::pages::smoke_scroll_parallax(160.0), "scroll the parallax scroller"),
    ];
    for (at, step, name) in steps {
        let _ = DispatcherTimer::run_once(
            move || println!("Flight: {name}: {}", step()),
            selected_at + Duration::from_millis(at),
            DispatcherPriority::NORMAL,
        );
    }
    for at in [2960u64, 3150, 3660, 3800, 4700] {
        let at = selected_at + Duration::from_millis(at);
        let path = directory.join(format!("steps-{}.png", (at - selected_at).as_millis()));
        let _ = DispatcherTimer::run_once(
            move || {
                if let Some(window) = main_window() {
                    save_composed(&window, path.clone(), format!("step at {} ms", (at - selected_at).as_millis()));
                }
            },
            at,
            DispatcherPriority::NORMAL,
        );
    }
    let mut index = 0;
    // `FERROUI_SMOKE_WINUI_FLIGHT_FEW`: two frames of each flight, for a build whose encoding of
    // a picture takes long (the debug build with Vello).
    let few = std::env::var_os("FERROUI_SMOKE_WINUI_FLIGHT_FEW").is_some();
    let plan: [(Duration, &[u64]); 2] = if few {
        [(open_at, &[130, 600]), (back_at, &[70, 500])]
    } else {
        [(open_at, &[30, 80, 130, 180, 230, 280, 340, 600]), (back_at, &[30, 70, 110, 150, 190, 240, 300, 500])]
    };
    for (start, offsets) in plan {
        for &offset in offsets {
            index += 1;
            let at = start + Duration::from_millis(offset);
            let path = directory.join(format!("flight-{index:02}-{}.png", (at - selected_at).as_millis()));
            let _ = DispatcherTimer::run_once(
                move || {
                    if let Some(window) = main_window() {
                        save_composed(&window, path.clone(), format!("flight at {} ms", (at - selected_at).as_millis()));
                    }
                },
                at,
                DispatcherPriority::NORMAL,
            );
        }
    }
}

/// `FERROUI_SMOKE_A11Y_TREE=<file>` (on Windows): once the window is up, a
/// client of UI Automation on a second thread walks the tree of the main
/// window through the client interface of the system, writes every element
/// (control type, name, class, states, bounding rectangle) to the file and
/// prints how many elements of each control type it found; the window is
/// closed when the walk is done. The process ends with code 1 when the walk
/// fails or finds nothing of the framework.
#[cfg(windows)]
fn accessibility_tree_run(path: PathBuf) {
    use ferroui_base::threading::Dispatcher;
    use ferroui_win32::automation::testing::{count_by_control_type, uninitialize_com, UiaClient};

    fn walk(handle: isize, path: &std::path::Path) -> Result<String, String> {
        let client = UiaClient::new()?;
        let root = client.element_from_handle(handle)?;
        let started = std::time::Instant::now();
        let elements = client.descendants(&root)?;
        let mut lines = vec![format!("{:?}", root.info(0)?)];
        let mut infos = Vec::with_capacity(elements.len());
        for (depth, element) in &elements {
            let info = element.info(*depth)?;
            let [left, top, right, bottom] = info.bounding_rectangle;
            lines.push(format!(
                "{}{} {:?} class {:?} framework {:?} enabled {} focusable {} at ({left},{top})-({right},{bottom})",
                "  ".repeat(info.depth),
                info.control_type_name(),
                info.name,
                info.class_name,
                info.framework_id,
                info.is_enabled,
                info.is_keyboard_focusable
            ));
            infos.push(info);
        }
        let seconds = started.elapsed().as_secs_f64();
        let of_the_framework = infos.iter().filter(|info| info.framework_id == "FerroUI").count();
        let counts = count_by_control_type(&infos);
        let summary = format!(
            "{} elements below the window ({of_the_framework} of the framework), walked and read in {seconds:.1} s: {}",
            infos.len(),
            counts.iter().map(|(name, count)| format!("{count} {name}")).collect::<Vec<_>>().join(", ")
        );
        lines.push(summary.clone());
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        std::fs::write(path, lines.join("\n") + "\n").map_err(|error| format!("{}: {error}", path.display()))?;
        drop(elements);
        drop(root);
        drop(client);
        if of_the_framework == 0 {
            return Err(format!("the tree has no element of the framework: {summary}"));
        }
        Ok(summary)
    }

    let milliseconds = environment_milliseconds("FERROUI_SMOKE_A11Y_AFTER_MS").unwrap_or(3000);
    println!("Will walk the accessibility tree of the main window after {milliseconds} ms");
    let _timer = DispatcherTimer::run_once(
        move || {
            let handle = main_window().and_then(|window| window.try_get_platform_handle()).map_or(0, |handle| handle.handle());
            let dispatcher = Dispatcher::ui_thread();
            let path = path.clone();
            std::thread::spawn(move || {
                let result = walk(handle, &path);
                uninitialize_com();
                match result {
                    Ok(summary) => println!("[ ok ] the accessibility tree of the catalog ({}): {summary}", path.display()),
                    Err(error) => {
                        println!("[FAIL] the accessibility tree of the catalog: {error}");
                        std::process::exit(1);
                    }
                }
                dispatcher.post(
                    || {
                        if let Some(main_window) = main_window() {
                            main_window.close();
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            });
        },
        Duration::from_millis(milliseconds),
        DispatcherPriority::NORMAL,
    );
}

/// `FERROUI_SMOKE_DEVTOOLS=<directory>` (debug builds): F12 goes to the input manager as the
/// platform passes a key (a raw key down and up on the main window), which opens DevTools; the run
/// then selects a text block in the combined tree and in the logical tree, records the click of a button of the catalog on
/// the events page, goes from there to the button in the visual tree, shows the hot keys, and
/// writes the frame of the DevTools window (and nothing else) an interval after each step, once it
/// is laid out. The main window is closed at the end.
#[cfg(debug_assertions)]
fn dev_tools_run(directory: PathBuf) {
    use ferroui_base::input::raw::{RawKeyEventArgs, RawKeyEventType};
    use ferroui_base::input::{IInputDevice, Key, RawInputModifiers, KeyDeviceType, KeyboardDevice, PhysicalKey};
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_controls::Button;
    use ferroui_diagnostics::diagnostics::view_models::{MainViewModel, TreeNode};
    use ferroui_diagnostics::diagnostics::views::MainWindow;
    use std::rc::Rc;

    if let Err(error) = std::fs::create_dir_all(&directory) {
        println!("DevTools: the directory {} could not be created: {error}", directory.display());
        return;
    }
    let interval = Duration::from_millis(environment_milliseconds("FERROUI_SMOKE_PAGES").unwrap_or(1500));

    fn dev_tools() -> Option<(Ref<Window>, Rc<MainViewModel>)> {
        let lifetime = Application::current().and_then(|application| application.application_lifetime())?;
        let windows = lifetime.as_classic_desktop_style_application_lifetime()?.windows();
        let window = windows.into_iter().find(|window| window.cast::<MainWindow>().is_some())?;
        let view_model = from_markup_value::<Rc<MainViewModel>>(&window.data_context())?;
        Some((window, view_model))
    }
    fn button_of_catalog() -> Option<Ref<Button>> {
        let window = main_window()?;
        let visual = window.upcast::<Visual>();
        // A button of the content, which is in the logical tree too (a part of a template is not).
        visual
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Button>())
            .find(|button| button.templated_parent().is_none())
    }
    fn first_text_block(node: &Rc<TreeNode>) -> Option<Ref<ferroui_controls::TextBlock>> {
        if let Some(text_block) = node.visual().cast::<ferroui_controls::TextBlock>() {
            return Some(text_block);
        }
        node.children().to_vec().iter().find_map(first_text_block)
    }
    let save = move |name: &str| match dev_tools() {
        Some((window, _)) => {
            let path = directory.join(format!("{name}.png"));
            match save_frame(&window, &path) {
                Ok(size) => println!("DevTools: {} ({} by {})", path.display(), size.width, size.height),
                Err(error) => println!("DevTools: {name} could not be written: {error}"),
            }
        }
        None => println!("DevTools: {name}: no DevTools window is open"),
    };

    let steps: Vec<Box<dyn Fn()>> = vec![
        Box::new(|| {
            let (Some(window), Some(manager), Some(keyboard)) =
                (main_window(), ferroui_base::input::InputManager::instance(), KeyboardDevice::instance())
            else {
                println!("DevTools: no main window, input manager or keyboard");
                return;
            };
            for type_ in [RawKeyEventType::KeyDown, RawKeyEventType::KeyUp] {
                let device: Rc<dyn IInputDevice> = keyboard.clone();
                let e = RawKeyEventArgs::new(
                    device,
                    0,
                    window.input_root(),
                    type_,
                    Key::F12,
                    RawInputModifiers::empty(),
                    PhysicalKey::F12,
                    None,
                    KeyDeviceType::Keyboard,
                );
                manager.process_input(Rc::new(e));
            }
            println!("DevTools: F12 pressed; open: {}", dev_tools().is_some());
        }),
        // The combined tree, the first tab: the text block of the home page selected, so that its
        // ancestors are expanded with the parts of their templates.
        Box::new(|| {
            if let Some((_, view_model)) = dev_tools() {
                let text_block = view_model.logical_tree().nodes().snapshot().iter().find_map(|node| first_text_block(node));
                if let Some(text_block) = text_block {
                    view_model.combined_tree().select_control(&text_block.upcast());
                }
            }
        }),
        {
            let save = save.clone();
            Box::new(move || save("01-combined-tree"))
        },
        // The time a selection takes until the details of the control are laid out: controls of the
        // catalog selected in turn in the combined tree (the property grid, the layout and the value
        // frames of each), as a developer clicks through the tree.
        Box::new(|| {
            if let (Some((window, view_model)), Some(catalog)) = (dev_tools(), main_window()) {
                let controls: Vec<Ref<ferroui_controls::Control>> = catalog
                    .upcast::<Visual>()
                    .get_visual_descendants()
                    .filter_map(|visual| visual.cast::<ferroui_controls::Control>())
                    .step_by(7)
                    .take(16)
                    .collect();
                let mut times = Vec::new();
                let counters_before = ferroui_base::diagnostics::perf_counters::snapshot();
                for control in controls.iter().cycle().take(std::env::var("FERROUI_SMOKE_SELECTIONS").ok().and_then(|n| n.parse().ok()).unwrap_or(48)) {
                    let start = std::time::Instant::now();
                    view_model.combined_tree().select_control(control);
                    window.update_layout();
                    times.push(start.elapsed().as_secs_f64() * 1000.0);
                }
                if ferroui_base::diagnostics::perf_counters::ENABLED {
                    // Per selection, with the build feature `perf-counters`.
                    let counters = ferroui_base::diagnostics::perf_counters::snapshot().since(&counters_before);
                    let selections = times.len().max(1) as u64;
                    for counter in ferroui_base::diagnostics::perf_counters::PerfCounter::ALL {
                        let value = counters.get(*counter);
                        if value > 0 {
                            println!("DevTools: per selection, {}: {}", counter.name(), value / selections);
                        }
                    }
                }
                times.sort_by(|a, b| a.total_cmp(b));
                println!(
                    "DevTools: selection to laid-out details, {} selections: median {:.1} ms, p90 {:.1} ms, max {:.1} ms",
                    times.len(),
                    times[times.len() / 2],
                    times[times.len() * 9 / 10],
                    times[times.len() - 1]
                );
            }
        }),
        Box::new(|| {
            // A text block of the home page: the button above is part of a template, which the
            // logical tree does not show.
            if let Some((_, view_model)) = dev_tools() {
                let text_block = view_model
                    .logical_tree()
                    .nodes()
                    .snapshot()
                    .iter()
                    .find_map(|node| first_text_block(node));
                if let Some(text_block) = text_block {
                    view_model.request_tree_navigate_to(&text_block.clone().upcast(), false);
                    println!("DevTools: selected the text block {:?} in the logical tree", text_block.text());
                }
            }
        }),
        {
            let save = save.clone();
            Box::new(move || save("02-logical-tree-details"))
        },
        Box::new(|| {
            if let (Some((_, view_model)), Some(button)) = (dev_tools(), button_of_catalog()) {
                view_model.set_selected_tab(MainViewModel::EVENTS_TAB);
                button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
                let events = view_model.events();
                println!("DevTools: {} event(s) recorded", events.recorded_events().count());
                if let Some(first) = events.recorded_events().to_vec().into_iter().next() {
                    events.set_selected_event(Some(first));
                }
            }
        }),
        {
            let save = save.clone();
            Box::new(move || save("03-events"))
        },
        // From the events page, as a double tap on a link of the route of an event navigates.
        Box::new(|| {
            if let (Some((_, view_model)), Some(button)) = (dev_tools(), button_of_catalog()) {
                view_model.request_tree_navigate_to(&button.clone().upcast(), true);
                println!("DevTools: selected {:?} in the visual tree", button.get_type().name());
            }
        }),
        {
            let save = save.clone();
            Box::new(move || save("04-visual-tree-button-details"))
        },
        Box::new(|| {
            if let Some((_, view_model)) = dev_tools() {
                view_model.set_selected_tab(MainViewModel::HOT_KEYS_TAB);
            }
        }),
        {
            let save = save.clone();
            Box::new(move || save("05-hot-keys"))
        },
        Box::new(|| {
            println!("DevTools: done, closing the windows");
            if let Some((window, _)) = dev_tools() {
                window.close();
            }
            if let Some(window) = main_window() {
                window.close();
            }
        }),
    ];
    let steps = Rc::new(steps);
    let next = Rc::new(std::cell::Cell::new(0));
    // Two intervals for the catalog to come up, then a step an interval.
    let _start = DispatcherTimer::run_once(
        move || {
            let (steps, next) = (steps.clone(), next.clone());
            let _timer = DispatcherTimer::run(
                move || {
                    let index = next.get();
                    next.set(index + 1);
                    if let Some(step) = steps.get(index) {
                        step();
                    }
                    index + 1 < steps.len()
                },
                interval,
                DispatcherPriority::NORMAL,
            );
        },
        interval * 2,
        DispatcherPriority::NORMAL,
    );
}

#[cfg(not(debug_assertions))]
fn dev_tools_run(_directory: PathBuf) {
    println!("DevTools: attached in debug builds only");
}

/// The smoke run asked for with `FERROUI_SMOKE_EXIT_MS`,
/// `FERROUI_SMOKE_PAGES` and `FERROUI_SMOKE_SCREENSHOTS`.
fn smoke_run() {
    if let Some(directory) = std::env::var_os("FERROUI_SMOKE_BACKDROP").filter(|directory| !directory.is_empty()) {
        backdrop_run(PathBuf::from(directory));
    } else if let Some(directory) = std::env::var_os("FERROUI_SMOKE_SCREENSHOTS").filter(|directory| !directory.is_empty()) {
        screenshot_run(PathBuf::from(directory));
    } else if let Some(directory) = std::env::var_os("FERROUI_SMOKE_DEVTOOLS").filter(|directory| !directory.is_empty()) {
        dev_tools_run(PathBuf::from(directory));
    } else if let Some(ms) = environment_milliseconds("FERROUI_SMOKE_PAGES") {
        control_catalog::show_every_page(Duration::from_millis(ms));
    }

    #[cfg(windows)]
    if let Some(path) = std::env::var_os("FERROUI_SMOKE_A11Y_TREE").filter(|path| !path.is_empty()) {
        accessibility_tree_run(PathBuf::from(path));
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

#[cfg(test)]
mod tests {
    use super::client_size_that_fits;
    use ferroui_base::{PixelPoint, PixelRect, Size};

    const CATALOG: Size = Size::new(1100.0, 800.0);

    fn frame(client: Size, horizontal: f64, vertical: f64) -> Size {
        Size::new(client.width + horizontal, client.height + vertical)
    }

    /// Windows and X11 at scaling 1: the desktop is in pixels and a unit of the window is one.
    #[test]
    fn a_window_within_the_working_area_is_left_alone() {
        let area = PixelRect::new(0, 0, 1920, 1040);
        let fitted = client_size_that_fits(area, PixelPoint::new(100, 100), CATALOG, frame(CATALOG, 16.0, 39.0), 1.0);
        assert_eq!(fitted, None);
    }

    #[test]
    fn a_window_larger_than_the_working_area_is_shrunk_to_it() {
        let area = PixelRect::new(0, 0, 1024, 728);
        let fitted = client_size_that_fits(area, PixelPoint::new(0, 0), CATALOG, frame(CATALOG, 16.0, 39.0), 1.0);
        assert_eq!(fitted, Some(Size::new(1008.0, 689.0)));
    }

    #[test]
    fn a_window_over_the_edge_keeps_its_size_when_it_fits_from_the_corner() {
        let area = PixelRect::new(0, 0, 1920, 1040);
        let fitted = client_size_that_fits(area, PixelPoint::new(1500, 600), CATALOG, frame(CATALOG, 16.0, 39.0), 1.0);
        assert_eq!(fitted, Some(CATALOG));
    }

    /// Windows and X11 at scaling 2: the desktop is in pixels, the window in units of two pixels.
    #[test]
    fn a_desktop_in_pixels_is_divided_by_the_scaling() {
        let area = PixelRect::new(0, 0, 1920, 1040);
        let fitted = client_size_that_fits(area, PixelPoint::new(0, 0), CATALOG, frame(CATALOG, 16.0, 39.0), 2.0);
        assert_eq!(fitted, Some(Size::new(944.0, 481.0)));
        // 1100 by 800 units are 2200 by 1600 pixels: within a desktop of 3840 by 2160.
        let large = PixelRect::new(0, 0, 3840, 2120);
        assert_eq!(client_size_that_fits(large, PixelPoint::new(200, 200), CATALOG, CATALOG, 2.0), None);
    }

    /// Wayland on an output of scale 2: the screen is 1280 by 800 logical units, the window
    /// renders at 2 and its desktop scaling is 1. Dividing by the render scaling here gave a
    /// window of 640 by 400.
    #[test]
    fn a_desktop_in_logical_units_is_not_divided_by_the_render_scaling() {
        let area = PixelRect::new(0, 0, 1280, 800);
        let tiled = Size::new(1280.0, 800.0);
        assert_eq!(client_size_that_fits(area, PixelPoint::new(0, 0), tiled, tiled, 1.0), None);
        assert_eq!(client_size_that_fits(area, PixelPoint::new(0, 0), CATALOG, CATALOG, 1.0), None);
        let small = PixelRect::new(0, 0, 640, 360);
        assert_eq!(client_size_that_fits(small, PixelPoint::new(0, 0), CATALOG, CATALOG, 1.0), Some(Size::new(640.0, 360.0)));
    }

    /// A working area that does not start at the origin (a task bar at the left or the top).
    #[test]
    fn a_window_before_the_start_of_the_working_area_is_moved() {
        let area = PixelRect::new(60, 30, 1860, 1050);
        let client = Size::new(800.0, 600.0);
        assert_eq!(client_size_that_fits(area, PixelPoint::new(0, 0), client, client, 1.0), Some(client));
        assert_eq!(client_size_that_fits(area, PixelPoint::new(60, 30), client, client, 1.0), None);
    }
}

#[cfg(feature = "cdp")]
ferroui_base::ferro_thread_local! {
    static CDP_SERVER: std::cell::RefCell<Option<ferroui_devtools::DevToolsServer>> = const { std::cell::RefCell::new(None) };
}

#[cfg(feature = "cdp")]
fn start_cdp() {
    let Ok(value) = std::env::var("FERROUI_CDP_PORT") else { return; };
    let port: u16 = value.parse().expect("FERROUI_CDP_PORT must be a port number (0 selects a free port)");
    let server = ferroui_devtools::DevToolsServer::start(port).expect("Could not start the CDP endpoint");
    println!("FerroUI CDP: http://{}", server.local_addr());
    CDP_SERVER.with(|slot| *slot.borrow_mut() = Some(server));
}
