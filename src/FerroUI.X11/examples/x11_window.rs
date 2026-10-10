//! A window on an X server: an application with the X11 platform, the
//! Skia renderer in software and the Simple theme, whose main window is
//! filled with one colour.
//!
//! ```text
//! cargo run -p ferroui-x11 --features example --example x11_window
//! xvfb-run -a cargo run -p ferroui-x11 --features example --example x11_window -- --smoke
//! ```
//!
//! With `--smoke` the example checks itself against the server and exits.
//! Every check prints a line; the exit code is 0 only when all of them
//! passed. The phases, in order (`--skip=input,popup,screens,clipboard`
//! leaves some out):
//!
//! - **window**: it asks the server (not the framework) whether the window
//!   is mapped, how large it is, what its title, class, process and
//!   protocols are, and reads pixels back from the window to see that the
//!   frame arrived.
//! - **input**: it has the server synthesize input through the XTEST
//!   extension (a pointer move, a button, the wheel, a key) and expects it
//!   at the input callback of the window, which is the contract between
//!   the platform and the framework.
//! - **popup**: it opens a popup over the window and asks the server for
//!   an override-redirect window of this process at the expected place
//!   with the expected pixels; a press inside the popup leaves it open and
//!   a press on the window beside it dismisses it.
//! - **screens**: the screens of the platform have to cover the root
//!   window without overlapping, and to be as many as
//!   `--expect-screens=N` says (the caller makes them, with
//!   `xrandr --setmonitor`).
//! - **clipboard**: text goes to another client and comes back from one,
//!   once small and once larger than a property may be, so that it is
//!   transferred incrementally (`INCR`) in each direction. The other
//!   client is `xclip`, which has to be installed.
//!
//! It then asks the window to close the way a window manager does (a
//! `WM_DELETE_WINDOW` message), which has to end the application.

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
    use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
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
    use std::cell::{Cell, RefCell};
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
        window.set_content(Some(Control::boxed(border.clone())));

        window.opened(|| println!("Window opened"));
        window.closed(|| println!("Window closed"));

        if std::env::args().any(|arg| arg == "--smoke") {
            smoke::start(&window, &border);
        }

        window
    }

    pub fn run() -> ExitCode {
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        let args: Vec<String> = std::env::args()
            .skip(1)
            .filter(|arg| arg != "--smoke" && !arg.starts_with("--skip=") && !arg.starts_with("--expect-screens="))
            .collect();

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


    /// Input the server synthesizes (the XTEST extension, `libXtst`). Not
    /// part of the platform, which never fakes input: the smoke mode uses
    /// it to stand in for a device.
    mod xtest {
        use ferroui_x11::xlib::{self, XDisplay};
        use std::sync::OnceLock;

        // The type of the library has the name of another extension in the bindings.
        type Library = x11_dl::xtest::Xf86vmode;

        fn library() -> Option<&'static Library> {
            static LIBRARY: OnceLock<Option<Library>> = OnceLock::new();
            LIBRARY.get_or_init(|| Library::open().ok()).as_ref()
        }

        fn raw(display: XDisplay) -> *mut x11_dl::xlib::Display {
            display.as_ptr().cast()
        }

        /// Whether the library is there and the server has the extension.
        pub fn available(display: XDisplay) -> bool {
            let Some(library) = library() else {
                return false;
            };
            let (mut event_base, mut error_base, mut major, mut minor) = (0, 0, 0, 0);
            // SAFETY: the connection is open (connections of the platform are never closed) and
            // every out pointer is a valid place for its result.
            unsafe {
                (library.XTestQueryExtension)(raw(display), &mut event_base, &mut error_base, &mut major, &mut minor) != 0
            }
        }

        /// Moves the pointer to a point of the screen of the pointer.
        pub fn motion(display: XDisplay, x: i32, y: i32) {
            if let Some(library) = library() {
                // SAFETY: the connection is open; the other arguments are plain values (-1 is the
                // screen the pointer is on, 0 no delay).
                unsafe { (library.XTestFakeMotionEvent)(raw(display), -1, x, y, 0) };
                xlib::x_flush(display);
            }
        }

        /// Presses or releases a pointer button.
        pub fn button(display: XDisplay, button: u32, press: bool) {
            if let Some(library) = library() {
                // SAFETY: the connection is open; the other arguments are plain values.
                unsafe { (library.XTestFakeButtonEvent)(raw(display), button, press as i32, 0) };
                xlib::x_flush(display);
            }
        }

        /// Presses or releases a key.
        pub fn key(display: XDisplay, key_code: u32, press: bool) {
            if let Some(library) = library() {
                // SAFETY: the connection is open; the other arguments are plain values.
                unsafe { (library.XTestFakeKeyEvent)(raw(display), key_code, press as i32, 0) };
                xlib::x_flush(display);
            }
        }

        /// The key code that produces a key symbol in the keyboard mapping of the server, 0
        /// when no key does.
        pub fn key_code(display: XDisplay, key_sym: u64) -> u32 {
            // SAFETY: the connection is open; the key symbol is a plain value.
            unsafe { (xlib::libraries().xlib.XKeysymToKeycode)(raw(display), key_sym as _) as u32 }
        }
    }

    /// The checks of the smoke mode.
    mod smoke {
        use super::*;
        use ferroui_base::input::platform::ClipboardExtensions;
        use ferroui_base::input::raw::{
            IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
            RawPointerEventType, RawTextInputEventArgs,
        };
        use ferroui_base::input::Key;
        use ferroui_base::{Point, Vector};
        use ferroui_controls::primitives::popup_positioning::{PopupAnchor, PopupGravity};
        use ferroui_controls::primitives::Popup;
        use ferroui_controls::PlacementMode;
        use std::future::Future;
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::sync::{Arc, Mutex};
        use std::task::{Poll, Waker};

        /// How often the checks of the window are tried before they count
        /// as failed: the first frame takes a moment to arrive.
        const ATTEMPTS: u32 = 20;
        const INTERVAL: Duration = Duration::from_millis(500);
        /// How long something the server or another client has to do may
        /// take before its check fails.
        const STEP_TIMEOUT: Duration = Duration::from_secs(5);
        /// How long a clipboard transfer with the other client may take.
        const TRANSFER_TIMEOUT: Duration = Duration::from_secs(30);
        /// How long the window may take to close after it was asked to.
        const CLOSE_TIMEOUT: Duration = Duration::from_secs(10);
        /// After this the run has hung and the process is ended.
        const RUN_TIMEOUT: Duration = Duration::from_secs(240);

        /// Where the popup is opened, from the top left corner of the
        /// window, and how large it is.
        const POPUP_OFFSET: (f64, f64) = (40.0, 30.0);
        const POPUP_SIZE: (f64, f64) = (120.0, 80.0);
        const POPUP_FILL: (u8, u8, u8) = (0xcc, 0x33, 0x00);

        /// The size of the large clipboard text: more than the largest
        /// property the platform writes (one mebibyte) and than the part
        /// size of the other client, so that both sides transfer it in
        /// parts.
        const LARGE_TEXT_SIZE: usize = 3 * 1024 * 1024;
        const SMALL_TEXT: &str = "FerroUI clipboard: za\u{17c}\u{f3}\u{142}\u{107} \u{20ac}";

        /// The lines of the checks, printed as they are made.
        struct Report {
            failed: Cell<u32>,
            passed: Cell<u32>,
        }

        impl Report {
            fn check(&self, name: &str, passed: bool, detail: String) {
                println!("  [{}] {name}: {detail}", if passed { "ok" } else { "FAILED" });
                let counter = if passed { &self.passed } else { &self.failed };
                counter.set(counter.get() + 1);
            }

            fn phase(&self, name: &str) {
                println!("Phase: {name}");
            }
        }

        struct Check {
            name: &'static str,
            passed: bool,
            detail: String,
        }

        fn check(checks: &mut Vec<Check>, name: &'static str, passed: bool, detail: String) {
            checks.push(Check { name, passed, detail });
        }

        /// What `--skip=` and `--expect-screens=` say.
        struct Options {
            skip: Vec<String>,
            expect_screens: Option<usize>,
        }

        impl Options {
            fn from_args() -> Self {
                let mut options = Options { skip: Vec::new(), expect_screens: None };
                for arg in std::env::args() {
                    if let Some(skip) = arg.strip_prefix("--skip=") {
                        options.skip.extend(skip.split(',').map(str::to_string));
                    }
                    if let Some(count) = arg.strip_prefix("--expect-screens=") {
                        options.expect_screens = count.parse().ok();
                    }
                }
                options
            }

            fn runs(&self, phase: &str) -> bool {
                !self.skip.iter().any(|skipped| skipped == phase)
            }
        }

        /// A future that completes after `duration`, on the dispatcher.
        fn delay(duration: Duration) -> impl Future<Output = ()> {
            let state: Rc<RefCell<(bool, Option<Waker>)>> = Rc::new(RefCell::new((false, None)));
            let timer_state = state.clone();
            let timer = DispatcherTimer::run_once(
                move || {
                    let waker = {
                        let mut state = timer_state.borrow_mut();
                        state.0 = true;
                        state.1.take()
                    };
                    if let Some(waker) = waker {
                        waker.wake();
                    }
                },
                duration,
                DispatcherPriority::BACKGROUND,
            );
            std::future::poll_fn(move |cx| {
                let _keep = &timer;
                let mut state = state.borrow_mut();
                if state.0 {
                    Poll::Ready(())
                } else {
                    state.1 = Some(cx.waker().clone());
                    Poll::Pending
                }
            })
        }

        /// Waits until `condition` holds, looking every 50 milliseconds;
        /// `false` when it did not within `timeout`.
        async fn wait_for(timeout: Duration, mut condition: impl FnMut() -> bool) -> bool {
            let started = std::time::Instant::now();
            loop {
                if condition() {
                    return true;
                }
                if started.elapsed() >= timeout {
                    return false;
                }
                delay(Duration::from_millis(50)).await;
            }
        }

        pub fn start(window: &Ref<Window>, content: &Ref<Border>) {
            println!("Smoke mode: the window is checked through the X server and then closed");
            // A run that hangs (a transfer that never completes stops the checks from going on) ends
            // here, with a line that says so.
            std::thread::spawn(|| {
                std::thread::sleep(RUN_TIMEOUT);
                println!("  [FAILED] run: the smoke mode did not finish within {} seconds", RUN_TIMEOUT.as_secs());
                println!("SMOKE FAILED");
                std::process::exit(1);
            });

            let window = window.clone();
            let content = content.clone();
            let _task = Dispatcher::ui_thread().invoke_async_task_local(move || run(window, content));
        }

        async fn run(window: Ref<Window>, content: Ref<Border>) {
            let options = Options::from_args();
            let report = Report { failed: Cell::new(0), passed: Cell::new(0) };

            report.phase("window");
            let mut attempt = 0;
            loop {
                delay(INTERVAL).await;
                attempt += 1;
                let checks = window_checks(&window);
                if checks.iter().all(|check| check.passed) || attempt >= ATTEMPTS {
                    println!("  attempt {attempt} of {ATTEMPTS}");
                    for check in checks {
                        report.check(check.name, check.passed, check.detail);
                    }
                    break;
                }
            }

            if let Some(platform) = FerroLocator::current().get_service::<FerroX11Platform>() {
                if options.runs("input") {
                    report.phase("input");
                    input_checks(&report, &platform, &window).await;
                }
                if options.runs("popup") {
                    report.phase("popup");
                    popup_checks(&report, &platform, &window, &content).await;
                }
                if options.runs("screens") {
                    report.phase("screens");
                    screens_checks(&report, &platform, &options);
                }
                if options.runs("clipboard") {
                    report.phase("clipboard");
                    clipboard_checks(&report, &window).await;
                }
            }

            println!("{} checks passed, {} failed", report.passed.get(), report.failed.get());
            SMOKE_PASSED.store(report.failed.get() == 0 && report.passed.get() > 0, Ordering::SeqCst);
            request_close(&window);
        }

        fn xid_of(window: &Ref<Window>) -> Option<xlib::XID> {
            window.try_get_platform_handle().map(|handle| handle.handle() as xlib::XID)
        }

        fn window_checks(window: &Ref<Window>) -> Vec<Check> {
            let mut checks = Vec::new();
            let Some(platform) = FerroLocator::current().get_service::<FerroX11Platform>() else {
                check(&mut checks, "platform", false, "the X11 platform is not registered".to_string());
                return checks;
            };
            let Some(xid) = xid_of(window) else {
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
            let expected = rgb(FILL);
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

        fn rgb(colour: (u8, u8, u8)) -> u64 {
            (colour.0 as u64) << 16 | (colour.1 as u64) << 8 | colour.2 as u64
        }

        /// What arrived at the input callback of the window.
        #[derive(Clone, Debug, PartialEq)]
        enum Recorded {
            Pointer(RawPointerEventType, Point),
            Wheel(Vector),
            Key(RawKeyEventType, Key, Option<String>),
            Text(String),
        }

        fn record(args: &Rc<dyn IRawInputEventArgs>) -> Option<Recorded> {
            if let Some(wheel) = args.downcast_ref::<RawMouseWheelEventArgs>() {
                return Some(Recorded::Wheel(wheel.delta()));
            }
            if let Some(pointer) = args.downcast_ref::<RawPointerEventArgs>() {
                return Some(Recorded::Pointer(pointer.type_(), pointer.position()));
            }
            if let Some(key) = args.downcast_ref::<RawKeyEventArgs>() {
                return Some(Recorded::Key(key.type_(), key.key(), key.key_symbol()));
            }
            if let Some(text) = args.downcast_ref::<RawTextInputEventArgs>() {
                return Some(Recorded::Text(text.text().to_string()));
            }
            None
        }

        /// Input from the server's side: every step is synthesized with
        /// XTEST and expected at the input callback of the window.
        async fn input_checks(report: &Report, platform: &Rc<FerroX11Platform>, window: &Ref<Window>) {
            let info = platform.info();
            let display = info.display();
            if !xtest::available(display) {
                report.check("XTEST", false, "libXtst is missing or the server has no XTEST extension".to_string());
                return;
            }
            let (Some(xid), Some(window_impl)) = (xid_of(window), window.platform_impl()) else {
                report.check("handle", false, "the window has no platform implementation".to_string());
                return;
            };
            let Some((origin_x, origin_y, _)) = xlib::x_translate_coordinates(display, xid, info.root_window(), 0, 0) else {
                report.check("origin", false, "XTranslateCoordinates failed".to_string());
                return;
            };
            let scaling = window.render_scaling();
            println!(
                "  the window is at ({origin_x}, {origin_y}) of the root; pointer input through {}",
                if platform.xi2().is_some() { "the X Input extension" } else { "core events" }
            );

            // The callback of the contract, with a recorder in front of the one of the framework.
            let log: Rc<RefCell<Vec<Recorded>>> = Rc::new(RefCell::new(Vec::new()));
            let framework_input = window_impl.input();
            {
                let log = log.clone();
                let framework_input = framework_input.clone();
                window_impl.set_input(Some(Rc::new(move |args: Rc<dyn IRawInputEventArgs>| {
                    if let Some(recorded) = record(&args) {
                        log.borrow_mut().push(recorded);
                    }
                    if let Some(framework_input) = &framework_input {
                        framework_input(args);
                    }
                })));
            }
            let seen = |wanted: &dyn Fn(&Recorded) -> bool| log.borrow().iter().any(|recorded| wanted(recorded));
            let tail = || {
                let log = log.borrow();
                format!("{:?}", &log[log.len().saturating_sub(6)..])
            };

            // A move: first somewhere in the window, then to the point that is expected.
            let (x, y) = (120, 90);
            let expected = Point::new(x as f64 / scaling, y as f64 / scaling);
            xtest::motion(display, origin_x + 50, origin_y + 60);
            delay(Duration::from_millis(200)).await;
            xtest::motion(display, origin_x + x, origin_y + y);
            let at_expected = |position: &Point| (position.x - expected.x).abs() < 0.5 && (position.y - expected.y).abs() < 0.5;
            let moved = wait_for(STEP_TIMEOUT, || {
                seen(&|recorded| matches!(recorded, Recorded::Pointer(RawPointerEventType::Move, position) if at_expected(position)))
            })
            .await;
            report.check("pointer move", moved, format!("expected a move to {expected:?}; last input {}", tail()));

            // The left button, down and up, at the same point.
            log.borrow_mut().clear();
            xtest::button(display, 1, true);
            let down = wait_for(STEP_TIMEOUT, || {
                seen(&|recorded| {
                    matches!(recorded, Recorded::Pointer(RawPointerEventType::LeftButtonDown, position) if at_expected(position))
                })
            })
            .await;
            xtest::button(display, 1, false);
            let up = wait_for(STEP_TIMEOUT, || {
                seen(&|recorded| {
                    matches!(recorded, Recorded::Pointer(RawPointerEventType::LeftButtonUp, position) if at_expected(position))
                })
            })
            .await;
            report.check("button", down && up, format!("down {down}, up {up} at {expected:?}; last input {}", tail()));

            // The wheel: buttons 4 and 5 are one step up and one step down.
            for (button, delta, name) in [(4, Vector::new(0.0, 1.0), "wheel up"), (5, Vector::new(0.0, -1.0), "wheel down")] {
                log.borrow_mut().clear();
                xtest::button(display, button, true);
                xtest::button(display, button, false);
                let turned = wait_for(STEP_TIMEOUT, || seen(&|recorded| *recorded == Recorded::Wheel(delta))).await;
                report.check(name, turned, format!("expected a wheel delta of {delta:?}; last input {}", tail()));
            }

            // A key that produces text: "a" (the key symbol 0x61). The server sends key events to the
            // window with the focus or, without a window manager, to the one under the pointer.
            log.borrow_mut().clear();
            let key_code = xtest::key_code(display, 0x61);
            if key_code == 0 {
                report.check("key", false, "no key of the keyboard mapping produces \"a\"".to_string());
            } else {
                xtest::key(display, key_code, true);
                let key_down = wait_for(STEP_TIMEOUT, || {
                    seen(&|recorded| matches!(recorded, Recorded::Key(RawKeyEventType::KeyDown, Key::A, _)))
                })
                .await;
                let text = wait_for(STEP_TIMEOUT, || seen(&|recorded| *recorded == Recorded::Text("a".to_string()))).await;
                xtest::key(display, key_code, false);
                let key_up = wait_for(STEP_TIMEOUT, || {
                    seen(&|recorded| matches!(recorded, Recorded::Key(RawKeyEventType::KeyUp, Key::A, _)))
                })
                .await;
                let symbol = log.borrow().iter().find_map(|recorded| match recorded {
                    Recorded::Key(RawKeyEventType::KeyDown, Key::A, symbol) => Some(symbol.clone()),
                    _ => None,
                });
                report.check(
                    "key",
                    key_down && key_up && symbol == Some(Some("a".to_string())),
                    format!("key code {key_code}: down {key_down}, up {key_up}, symbol {symbol:?}; last input {}", tail()),
                );
                report.check("text", text, format!("expected the text \"a\"; last input {}", tail()));
            }

            window_impl.set_input(framework_input);
        }

        /// The override-redirect windows of this process that are children
        /// of the root and viewable, as the server lists them.
        fn popup_windows(platform: &Rc<FerroX11Platform>) -> Vec<(xlib::XID, (i32, i32))> {
            let info = platform.info();
            let display = info.display();
            let atoms = info.atoms();
            xlib::x_sync(display, false);
            let Some((_, _, children)) = xlib::x_query_tree(display, info.root_window()) else {
                return Vec::new();
            };
            children
                .into_iter()
                .filter_map(|child| {
                    let attributes = xlib::x_get_window_attributes(display, child)?;
                    let pid = xlib::x_get_window_property_as_int_ptr(display, child, atoms._NET_WM_PID, atoms.CARDINAL);
                    (attributes.override_redirect != 0
                        && attributes.map_state == 2
                        && pid == Some(std::process::id() as _))
                    .then_some((child, (attributes.width, attributes.height)))
                })
                .collect()
        }

        /// A popup: an override-redirect window the server knows, placed
        /// relative to the window, drawn, kept open by a press inside it
        /// and dismissed by a press on the window beside it. (The platform
        /// takes no pointer grab for a popup, as the reference: the
        /// framework dismisses it when the press arrives at its parent.)
        async fn popup_checks(
            report: &Report,
            platform: &Rc<FerroX11Platform>,
            window: &Ref<Window>,
            content: &Ref<Border>,
        ) {
            let info = platform.info();
            let display = info.display();
            let Some(xid) = xid_of(window) else {
                report.check("handle", false, "the window has no platform handle".to_string());
                return;
            };
            let Some((origin_x, origin_y, _)) = xlib::x_translate_coordinates(display, xid, info.root_window(), 0, 0) else {
                report.check("origin", false, "XTranslateCoordinates failed".to_string());
                return;
            };
            let scaling = window.render_scaling();
            let before = popup_windows(platform).len();

            let fill = Border::new();
            fill.set_width(POPUP_SIZE.0);
            fill.set_height(POPUP_SIZE.1);
            fill.set_background(Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(
                POPUP_FILL.0,
                POPUP_FILL.1,
                POPUP_FILL.2,
            )))));

            let closed = Rc::new(Cell::new(false));
            let popup = Popup::new();
            popup.set_child(fill);
            popup.set_placement_target(content);
            popup.set_placement(PlacementMode::AnchorAndGravity);
            popup.set_placement_anchor(PopupAnchor::TOP_LEFT);
            popup.set_placement_gravity(PopupGravity::BOTTOM_RIGHT);
            popup.set_horizontal_offset(POPUP_OFFSET.0);
            popup.set_vertical_offset(POPUP_OFFSET.1);
            popup.set_is_light_dismiss_enabled(true);
            let _closed = popup.closed({
                let closed = closed.clone();
                move || closed.set(true)
            });
            popup.open();

            let expected_position =
                (origin_x + (POPUP_OFFSET.0 * scaling) as i32, origin_y + (POPUP_OFFSET.1 * scaling) as i32);
            let expected_size = ((POPUP_SIZE.0 * scaling) as i32, (POPUP_SIZE.1 * scaling) as i32);
            let expected_pixel = rgb(POPUP_FILL);

            // The window, at its place and drawn: the frame takes a moment.
            let mut found: Option<(xlib::XID, (i32, i32), (i32, i32), Option<u64>)> = None;
            let shown = wait_for(STEP_TIMEOUT, || {
                let windows = popup_windows(platform);
                let Some((popup_xid, size)) = windows.last().copied().filter(|_| windows.len() == before + 1) else {
                    return false;
                };
                let Some(position) =
                    xlib::x_translate_coordinates(display, popup_xid, info.root_window(), 0, 0).map(|(x, y, _)| (x, y))
                else {
                    return false;
                };
                let pixel = xlib::x_get_pixel(display, popup_xid, size.0 / 2, size.1 / 2).map(|pixel| pixel as u64);
                found = Some((popup_xid, position, size, pixel));
                position == expected_position
                    && size == expected_size
                    && pixel.map(|pixel| pixel & 0x00ff_ffff) == Some(expected_pixel)
            })
            .await;
            match found {
                Some((popup_xid, position, size, pixel)) => {
                    report.check(
                        "override-redirect window",
                        true,
                        format!("{popup_xid:#x}, a viewable child of the root that the window manager does not manage"),
                    );
                    report.check(
                        "position",
                        position == expected_position,
                        format!(
                            "{position:?} of the root, {expected_position:?} expected: the window at ({origin_x}, {origin_y}) \
                             and the offset {POPUP_OFFSET:?} at scaling {scaling}"
                        ),
                    );
                    report.check("size", size == expected_size, format!("{size:?}, {expected_size:?} expected"));
                    report.check(
                        "pixel",
                        shown,
                        format!("centre: {pixel:x?}, {expected_pixel:#08x} expected in the low 24 bits"),
                    );
                }
                None => {
                    report.check(
                        "override-redirect window",
                        false,
                        format!(
                            "the server shows {} override-redirect windows of this process, {} before the popup opened; \
                             the popup is {}",
                            popup_windows(platform).len(),
                            before,
                            if popup.is_open() { "open" } else { "not open" }
                        ),
                    );
                    popup.close();
                    return;
                }
            }

            if !xtest::available(display) {
                report.check("XTEST", false, "libXtst is missing or the server has no XTEST extension".to_string());
                popup.close();
                return;
            }

            // A press inside the popup goes to the popup.
            xtest::motion(display, expected_position.0 + expected_size.0 / 2, expected_position.1 + expected_size.1 / 2);
            delay(Duration::from_millis(200)).await;
            xtest::button(display, 1, true);
            xtest::button(display, 1, false);
            delay(Duration::from_millis(500)).await;
            let windows = popup_windows(platform).len();
            report.check(
                "a press inside keeps it open",
                popup.is_open() && !closed.get() && windows == before + 1,
                format!(
                    "open {}, closed raised {}, {} popup windows on the server",
                    popup.is_open(),
                    closed.get(),
                    windows.saturating_sub(before)
                ),
            );

            // A press on the window, to the right of and below the popup, dismisses it.
            let outside = (
                origin_x + ((POPUP_OFFSET.0 + POPUP_SIZE.0 + 100.0) * scaling) as i32,
                origin_y + ((POPUP_OFFSET.1 + POPUP_SIZE.1 + 100.0) * scaling) as i32,
            );
            xtest::motion(display, outside.0, outside.1);
            delay(Duration::from_millis(200)).await;
            xtest::button(display, 1, true);
            xtest::button(display, 1, false);
            let dismissed =
                wait_for(STEP_TIMEOUT, || !popup.is_open() && closed.get() && popup_windows(platform).len() == before).await;
            report.check(
                "a press outside dismisses it",
                dismissed,
                format!(
                    "pressed at {outside:?}: open {}, closed raised {}, {} popup windows left on the server",
                    popup.is_open(),
                    closed.get(),
                    popup_windows(platform).len().saturating_sub(before)
                ),
            );
            if popup.is_open() {
                popup.close();
            }
        }

        /// The screens of the platform against the root window of the
        /// server.
        fn screens_checks(report: &Report, platform: &Rc<FerroX11Platform>, options: &Options) {
            let info = platform.info();
            let screens = platform.screens().all_screens();
            for screen in &screens {
                println!(
                    "  screen {:?}: bounds {:?}, working area {:?}, scaling {}, primary {}",
                    screen.display_name(),
                    screen.bounds(),
                    screen.working_area(),
                    screen.scaling(),
                    screen.is_primary()
                );
            }
            match options.expect_screens {
                Some(expected) => report.check(
                    "count",
                    screens.len() == expected,
                    format!("{} screens, {expected} expected (--expect-screens)", screens.len()),
                ),
                None => report.check("count", !screens.is_empty(), format!("{} screens", screens.len())),
            }

            let Some(root) = xlib::x_get_geometry(info.display(), info.root_window()) else {
                report.check("root", false, "XGetGeometry of the root window failed".to_string());
                return;
            };
            let area: i64 = screens.iter().map(|screen| screen.bounds().width as i64 * screen.bounds().height as i64).sum();
            let inside = screens.iter().all(|screen| {
                let bounds = screen.bounds();
                bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= root.width && bounds.y + bounds.height <= root.height
            });
            let overlapping = screens.iter().enumerate().any(|(i, a)| {
                screens.iter().skip(i + 1).any(|b| {
                    let (a, b) = (a.bounds(), b.bounds());
                    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
                })
            });
            report.check(
                "the screens cover the root window",
                inside && !overlapping && area == root.width as i64 * root.height as i64,
                format!(
                    "root {}x{}; the screens are inside it: {inside}, overlap: {overlapping}, their area: {area}",
                    root.width, root.height
                ),
            );
            let primary = screens.iter().filter(|screen| screen.is_primary()).count();
            report.check(
                "scaling and primary",
                screens.iter().all(|screen| screen.scaling() > 0.0) && primary <= 1,
                format!("{primary} primary"),
            );
        }

        /// Text of `size` bytes that is the same in every text encoding.
        fn large_text(size: usize, seed: u8) -> String {
            let mut text = String::with_capacity(size + 64);
            let mut line = 0u32;
            while text.len() < size {
                text.push_str(&format!("{seed:02x} line {line:08} of a text that is transferred in parts\n"));
                line += 1;
            }
            text.truncate(size);
            text
        }

        /// What a helper process printed, when it has ended.
        type HelperOutput = Arc<Mutex<Option<Result<Vec<u8>, String>>>>;

        /// Has the other client ask for the clipboard and print it. The
        /// client runs while the dispatcher does, which is what answers it.
        fn other_client_reads() -> HelperOutput {
            let output: HelperOutput = Arc::new(Mutex::new(None));
            let slot = output.clone();
            std::thread::spawn(move || {
                let result = Command::new("xclip")
                    .args(["-selection", "clipboard", "-t", "UTF8_STRING", "-o"])
                    .stdin(Stdio::null())
                    .output();
                let result = match result {
                    Ok(output) if output.status.success() => Ok(output.stdout),
                    Ok(output) => Err(format!("xclip: {}: {}", output.status, String::from_utf8_lossy(&output.stderr))),
                    Err(error) => Err(format!("xclip could not be started: {error}")),
                };
                *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(result);
            });
            output
        }

        /// Has the other client own the clipboard with `text`. It keeps
        /// owning it in the background until another client takes it.
        fn other_client_owns(text: &str) -> Result<(), String> {
            let mut child = Command::new("xclip")
                .args(["-selection", "clipboard", "-t", "UTF8_STRING", "-i"])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("xclip could not be started: {error}"))?;
            let mut stdin = child.stdin.take().ok_or("xclip has no standard input")?;
            stdin.write_all(text.as_bytes()).map_err(|error| format!("writing to xclip: {error}"))?;
            drop(stdin);
            let status = child.wait().map_err(|error| format!("waiting for xclip: {error}"))?;
            if status.success() {
                Ok(())
            } else {
                Err(format!("xclip: {status}"))
            }
        }

        fn summary(text: &[u8]) -> String {
            if text.len() <= 64 {
                format!("{:?}", String::from_utf8_lossy(text))
            } else {
                format!("{} bytes", text.len())
            }
        }

        /// The clipboard between this client and another one, in both
        /// directions, small and in parts.
        async fn clipboard_checks(report: &Report, window: &Ref<Window>) {
            let Some(clipboard) = window.clipboard() else {
                report.check("clipboard", false, "the window has no clipboard".to_string());
                return;
            };

            let to_other = large_text(LARGE_TEXT_SIZE, 0xa1);
            let from_other = large_text(LARGE_TEXT_SIZE, 0xb2);

            for (name, text) in [("to another client", SMALL_TEXT), ("to another client, in parts (INCR)", &to_other)] {
                if let Err(error) = clipboard.set_text_async(Some(text)).await {
                    report.check(name, false, format!("setting the text failed: {error:?}"));
                    continue;
                }
                let output = other_client_reads();
                let ended = wait_for(TRANSFER_TIMEOUT, || {
                    output.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).is_some()
                })
                .await;
                let result = output.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).take();
                match result {
                    Some(Ok(read)) => report.check(
                        name,
                        read == text.as_bytes(),
                        format!("the other client read {}, {} was set", summary(&read), summary(text.as_bytes())),
                    ),
                    Some(Err(error)) => report.check(name, false, error),
                    None => report.check(name, false, format!("the other client did not end (ended: {ended})")),
                }
            }

            for (name, text) in [("from another client", SMALL_TEXT), ("from another client, in parts (INCR)", &from_other)] {
                if let Err(error) = other_client_owns(text) {
                    report.check(name, false, error);
                    continue;
                }
                // The other client took the selection; this one hears of it with its next events.
                delay(Duration::from_millis(300)).await;
                match clipboard.try_get_text_async().await {
                    Ok(Some(read)) => report.check(
                        name,
                        read == text,
                        format!("read {}, the other client owns {}", summary(read.as_bytes()), summary(text.as_bytes())),
                    ),
                    Ok(None) => report.check(name, false, "the clipboard has no text".to_string()),
                    Err(error) => report.check(name, false, format!("reading the text failed: {error:?}")),
                }
            }

            // Taking the clipboard back ends the other client, which owned it in the background.
            let taken_back = clipboard.set_text_async(Some(SMALL_TEXT)).await;
            report.check("owned again", taken_back.is_ok(), format!("{taken_back:?}"));
        }

        /// Asks the window to close as a window manager does, and fails
        /// the run when it does not.
        fn request_close(window: &Ref<Window>) {
            let Some(platform) = FerroLocator::current().get_service::<FerroX11Platform>() else {
                return;
            };
            let Some(xid) = xid_of(window) else {
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
