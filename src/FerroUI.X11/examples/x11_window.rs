//! A window on an X server: an application with the X11 platform, the
//! Skia renderer in software and the Simple theme, whose main window is
//! filled with one colour.
//!
//! ```text
//! cargo run -p ferroui-x11 --example x11_window
//! xvfb-run -a cargo run -p ferroui-x11 --example x11_window -- --smoke
//! ```
//!
//! With `--smoke` the example checks itself against the server and exits:
//! it asks the server (not the framework) whether the window is mapped,
//! how large it is, what its title, class, process and protocols are, and
//! reads pixels back from the window to see that the frame arrived. It
//! then asks the window to close the way a window manager does (a
//! `WM_DELETE_WINDOW` message), which has to end the application. Every
//! check prints a line; the exit code is 0 only when all of them passed.

#[cfg(not(unix))]
fn main() {
    eprintln!("x11_window: this example needs an X server, which this system does not have");
}

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    app::run()
}

#[cfg(unix)]
mod app {
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::Color;
    use ferroui_base::styling::Styles;
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_base::{
        ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, Ref,
    };
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication, Window,
    };
    use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
    use ferroui_skia::SkiaApplicationExtensions;
    use ferroui_themes_simple::SimpleTheme;
    use ferroui_x11::x11_structs::{EventMask, XEventName};
    use ferroui_x11::xlib;
    use ferroui_x11::{FerroX11Platform, FerroX11PlatformExtensions, X11PlatformOptions, X11RenderingMode};
    use std::cell::Cell;
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    const TITLE: &str = "FerroUI X11 window";
    const WIDTH: f64 = 640.0;
    const HEIGHT: f64 = 400.0;
    /// The colour the window is filled with.
    const FILL: (u8, u8, u8) = (0x33, 0x66, 0x99);

    /// Whether every check of the smoke mode passed.
    static SMOKE_PASSED: AtomicBool = AtomicBool::new(false);

    #[repr(C)]
    pub struct App {
        base: Application,
    }

    ferro_class!(App: Application);
    ferro_impl_classes!(App: FerroObjectImpl);

    impl NewApplication for App {
        fn new_application() -> Ref<Self> {
            instantiate(Self { base: Application::construct() })
        }
    }

    impl ApplicationImpl for App {
        fn initialize(this: &Self) {
            Self::parent_initialize(this);
            this.set_name(Some(TITLE.to_string()));
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(desktop) =
                lifetime.as_ref().and_then(|lifetime| lifetime.as_classic_desktop_style_application_lifetime())
            {
                desktop.set_main_window(Some(create_main_window()));
            }

            Self::parent_on_framework_initialization_completed(this);
        }
    }

    fn create_main_window() -> Ref<Window> {
        let border = Border::new();
        border.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(FILL.0, FILL.1, FILL.2)))));

        let window = Window::new();
        window.set_title(Some(TITLE.to_string()));
        window.set_width(WIDTH);
        window.set_height(HEIGHT);
        window.set_content(Some(Control::boxed(border)));

        window.opened(|| println!("Window opened"));
        window.closed(|| println!("Window closed"));

        if std::env::args().any(|arg| arg == "--smoke") {
            smoke::start(&window);
        }

        window
    }

    pub fn run() -> ExitCode {
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        let args: Vec<String> = std::env::args().skip(1).filter(|arg| arg != "--smoke").collect();

        let mut options = X11PlatformOptions::new();
        options.rendering_mode = vec![X11RenderingMode::Software];

        let exit_code = AppBuilder::configure::<App>()
            .with(Rc::new(options))
            .use_harfbuzz()
            .use_x11()
            .use_skia()
            .start_with_classic_desktop_lifetime(&args);
        println!("The application ended with exit code {exit_code}");

        if smoke {
            if exit_code == 0 && SMOKE_PASSED.load(Ordering::SeqCst) {
                println!("SMOKE PASSED");
                ExitCode::SUCCESS
            } else {
                println!("SMOKE FAILED");
                ExitCode::FAILURE
            }
        } else {
            ExitCode::from(exit_code as u8)
        }
    }

    /// The checks of the smoke mode.
    mod smoke {
        use super::*;

        /// How often the checks are tried before they count as failed: the
        /// first frame takes a moment to arrive.
        const ATTEMPTS: u32 = 20;
        const INTERVAL: Duration = Duration::from_millis(500);
        /// How long the window may take to close after it was asked to.
        const CLOSE_TIMEOUT: Duration = Duration::from_secs(10);

        struct Check {
            name: &'static str,
            passed: bool,
            detail: String,
        }

        fn check(checks: &mut Vec<Check>, name: &'static str, passed: bool, detail: String) {
            checks.push(Check { name, passed, detail });
        }

        pub fn start(window: &Ref<Window>) {
            println!("Smoke mode: the window is checked through the X server and then closed");
            let attempt = Rc::new(Cell::new(0u32));
            schedule_attempt(window.clone(), attempt);
        }

        fn schedule_attempt(window: Ref<Window>, attempt: Rc<Cell<u32>>) {
            let _timer = DispatcherTimer::run_once(
                move || {
                    attempt.set(attempt.get() + 1);
                    let checks = run_checks(&window);
                    let passed = checks.iter().all(|check| check.passed);
                    if passed || attempt.get() >= ATTEMPTS {
                        println!("Checks, attempt {} of {ATTEMPTS}:", attempt.get());
                        for check in &checks {
                            println!(
                                "  [{}] {}: {}",
                                if check.passed { "ok" } else { "FAILED" },
                                check.name,
                                check.detail
                            );
                        }
                        SMOKE_PASSED.store(passed, Ordering::SeqCst);
                        request_close(&window);
                    } else {
                        schedule_attempt(window.clone(), attempt.clone());
                    }
                },
                INTERVAL,
                DispatcherPriority::BACKGROUND,
            );
        }

        fn run_checks(window: &Ref<Window>) -> Vec<Check> {
            let mut checks = Vec::new();
            let Some(platform) = FerroLocator::current().get_service::<FerroX11Platform>() else {
                check(&mut checks, "platform", false, "the X11 platform is not registered".to_string());
                return checks;
            };
            let Some(xid) = window.try_get_platform_handle().map(|handle| handle.handle() as xlib::XID) else {
                check(&mut checks, "handle", false, "the window has no platform handle".to_string());
                return checks;
            };
            let info = platform.info();
            let display = info.display();
            let atoms = info.atoms();
            // Everything the UI thread asked for has reached the server.
            xlib::x_sync(display, false);

            let scaling = window.render_scaling();
            // What was asked for, and what the framework believes the window has. A window manager may
            // grant another size than the one asked for (a window larger than the work area is
            // clamped), so the requested size is only expected without one.
            let requested = ((WIDTH * scaling) as i32, (HEIGHT * scaling) as i32);
            let client_size = window.client_size();
            let believed = ((client_size.width * scaling) as i32, (client_size.height * scaling) as i32);
            let has_window_manager = platform.globals().net_supported().is_some();
            let (mut server_width, mut server_height) = requested;

            // IsViewable is 2.
            let attributes = xlib::x_get_window_attributes(display, xid);
            match &attributes {
                Some(attributes) => {
                    check(
                        &mut checks,
                        "mapped",
                        attributes.map_state == 2,
                        format!("map state {} (2 is viewable), window {xid:#x}", attributes.map_state),
                    );
                    server_width = attributes.width;
                    server_height = attributes.height;
                    let on_server = (attributes.width, attributes.height);
                    check(
                        &mut checks,
                        "size",
                        on_server == believed && (has_window_manager || on_server == requested),
                        format!(
                            "{}x{} on the server, {}x{} in the framework, {}x{} requested at scaling {scaling}, {}",
                            on_server.0,
                            on_server.1,
                            believed.0,
                            believed.1,
                            requested.0,
                            requested.1,
                            if has_window_manager { "with a window manager" } else { "without a window manager" }
                        ),
                    );
                    check(
                        &mut checks,
                        "depth",
                        attributes.depth == 24 || attributes.depth == 32,
                        format!("depth {}", attributes.depth),
                    );
                }
                None => check(&mut checks, "attributes", false, "XGetWindowAttributes failed".to_string()),
            }

            let name = xlib::x_fetch_name(display, xid);
            check(&mut checks, "title (WM_NAME)", name.as_deref() == Some(TITLE), format!("{name:?}"));

            let net_name = xlib::x_get_window_property(display, xid, atoms._NET_WM_NAME, 0, 1024, false, atoms.UTF8_STRING);
            let net_name_text = String::from_utf8_lossy(&net_name.data).into_owned();
            check(
                &mut checks,
                "title (_NET_WM_NAME)",
                net_name.actual_format == 8 && net_name_text == TITLE,
                format!("{net_name_text:?}, format {}", net_name.actual_format),
            );

            let pid = xlib::x_get_window_property_as_int_ptr(display, xid, atoms._NET_WM_PID, atoms.CARDINAL);
            check(
                &mut checks,
                "process (_NET_WM_PID)",
                pid == Some(std::process::id() as _),
                format!("{pid:?}, this process is {}", std::process::id()),
            );

            let protocols =
                xlib::x_get_window_property_as_int_ptr_array(display, xid, atoms.WM_PROTOCOLS, atoms.ATOM).unwrap_or_default();
            check(
                &mut checks,
                "protocols (WM_PROTOCOLS)",
                protocols.contains(&atoms.WM_DELETE_WINDOW) && protocols.contains(&atoms._NET_WM_SYNC_REQUEST),
                format!(
                    "{:?}",
                    protocols.iter().map(|atom| atoms.get_atom_name(*atom).unwrap_or_default()).collect::<Vec<_>>()
                ),
            );

            let class = xlib::x_get_window_property(display, xid, atoms.WM_CLASS, 0, 1024, false, atoms.STRING);
            check(
                &mut checks,
                "class (WM_CLASS)",
                class.actual_format == 8 && class.data.iter().filter(|byte| **byte == 0).count() == 2,
                format!("{:?}", String::from_utf8_lossy(&class.data)),
            );

            let window_type =
                xlib::x_get_window_property_as_int_ptr(display, xid, atoms._NET_WM_WINDOW_TYPE, atoms.ATOM);
            check(
                &mut checks,
                "type (_NET_WM_WINDOW_TYPE)",
                window_type == Some(atoms._NET_WM_WINDOW_TYPE_NORMAL),
                format!("{:?}", window_type.and_then(|atom| atoms.get_atom_name(atom))),
            );

            // The pixels the server holds for the window: the centre and a point near each corner.
            let expected = (FILL.0 as u64) << 16 | (FILL.1 as u64) << 8 | FILL.2 as u64;
            let points = [
                ("centre", server_width / 2, server_height / 2),
                ("top left", 4, 4),
                ("bottom right", server_width - 5, server_height - 5),
            ];
            for (where_, x, y) in points {
                let pixel = xlib::x_get_pixel(display, xid, x, y).map(|pixel| pixel as u64);
                check(
                    &mut checks,
                    "pixel",
                    pixel.map(|pixel| pixel & 0x00ff_ffff) == Some(expected),
                    match pixel {
                        Some(pixel) => format!("{where_} ({x}, {y}): {pixel:#010x}, {expected:#08x} expected in the low 24 bits"),
                        None => format!("{where_} ({x}, {y}): XGetImage failed"),
                    },
                );
            }

            checks
        }

        /// Asks the window to close as a window manager does, and fails
        /// the run when it does not.
        fn request_close(window: &Ref<Window>) {
            let Some(platform) = FerroLocator::current().get_service::<FerroX11Platform>() else {
                return;
            };
            let Some(xid) = window.try_get_platform_handle().map(|handle| handle.handle() as xlib::XID) else {
                return;
            };
            let info = platform.info();
            let atoms = info.atoms();
            let mut event = xlib::new_event();
            {
                let message = xlib::client_message_event_mut(&mut event);
                message.type_ = XEventName::ClientMessage as i32;
                message.window = xid;
                message.message_type = atoms.WM_PROTOCOLS;
                message.format = 32;
                message.data.set_long(0, atoms.WM_DELETE_WINDOW as _);
                message.data.set_long(1, 0);
            }
            println!("Sending WM_DELETE_WINDOW to the window");
            xlib::x_send_event(info.display(), xid, false, EventMask::NO_EVENT_MASK.bits() as _, &mut event);
            xlib::x_flush(info.display());

            let _watchdog = DispatcherTimer::run_once(
                || {
                    println!("  [FAILED] close: the window did not close after WM_DELETE_WINDOW");
                    println!("SMOKE FAILED");
                    std::process::exit(1);
                },
                CLOSE_TIMEOUT,
                DispatcherPriority::BACKGROUND,
            );
        }
    }
}
