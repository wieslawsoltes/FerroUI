//! A window on an X server: an application with the X11 platform, the
//! Skia renderer in software and the Simple theme, whose main window is
//! filled with one colour.
//!
//! ```text
//! cargo run -p ferroui-x11 --features example --example x11_window
//! xvfb-run -a cargo run -p ferroui-x11 --features example --example x11_window -- --smoke
//! ```
//!
//! `--mode=glx` and `--mode=egl` render with OpenGL through GLX or EGL
//! (with software rendering as the fallback, as an application would list
//! it); without the option the window is rendered in software. GLX refuses
//! the software renderer of Mesa (`llvmpipe`) unless
//! `FERROUI_GLX_IGNORE_RENDERER_BLACKLIST=1` is set; `--expect-fallback`
//! makes the smoke mode expect exactly that refusal.
//!
//! `--shm` sends the frames of software rendering through the shared
//! memory extension of the server (`X11PlatformOptions::use_x_shm_framebuffer`).
//!
//! With `--smoke` the example checks itself against the server and exits.
//! Every check prints a line; the exit code is 0 only when all of them
//! passed. The phases, in order (`--skip=gpu,input,popup,resize,screens,clipboard`
//! leaves some out):
//!
//! - **window**: it asks the server (not the framework) whether the window
//!   is mapped, how large it is, what its title, class, process and
//!   protocols are, and reads pixels back from the window to see that the
//!   frame arrived.
//! - **gpu** (with `--mode=glx` or `--mode=egl`): the platform has the
//!   graphics of the mode and did not fall back; the window has a render
//!   window; a context of the platform graphics clears an offscreen
//!   framebuffer and reads the colour back with `glReadPixels`; with GLX
//!   the frame of the window is read back from the buffers of its render
//!   window with `glReadPixels` as well. (With EGL the frame is checked through the
//!   server only, by the pixels of the phase before: a window has one EGL
//!   surface, and it belongs to the compositor.)
//! - **input**: it has the server synthesize input through the XTEST
//!   extension (a pointer move, a button, the wheel, a key) and expects it
//!   at the input callback of the window, which is the contract between
//!   the platform and the framework.
//! - **popup**: it opens a popup over the window and asks the server for
//!   an override-redirect window of this process at the expected place
//!   with the expected pixels; a press inside the popup leaves it open and
//!   a press on the window beside it dismisses it.
//! - **shm** (with `--shm`): the server has the extension and the first
//!   surface of the window is the shared memory one, so the pixels of the
//!   other phases arrived through it.
//! - **resize**: the window is given another size; the server has to
//!   show it, and the fill colour at the new bottom right corner, which
//!   needs frames of the new size.
//! - **screens**: the screens of the platform have to cover the root
//!   window without overlapping, and to be as many as
//!   `--expect-screens=N` says (the caller makes them, with
//!   `xrandr --setmonitor`).
//! - **clipboard**: text goes to another client and comes back from one,
//!   once small and once larger than a property may be, so that it is
//!   transferred incrementally (`INCR`) in each direction. The other
//!   client is `xclip`, which has to be installed.
//! - **ime** (with `--ime=ibus` or `--ime=xim`): a text box gets the
//!   focus and the server synthesizes keys. With `ibus` the input method
//!   is the one over D-Bus, and this example is also the service it talks
//!   to (a double of the portal of IBus on the session bus, so the run
//!   needs one: `dbus-run-session`): the key "a" is consumed and another
//!   text is committed for it, the key "b" passes. With `xim` it is the
//!   input method of the server, the one Xlib has built in
//!   (`XMODIFIERS=@im=local`): a key produces its text through the input
//!   context, and a compose sequence produces one character (the caller
//!   gives a key the compose symbol: `xmodmap -e "keycode 135 = Multi_key"`).
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

    /// The rendering mode `--mode=` asks for: `software` without the option.
    fn mode() -> String {
        std::env::args().find_map(|arg| arg.strip_prefix("--mode=").map(str::to_string)).unwrap_or("software".to_string())
    }

    pub fn run() -> ExitCode {
        prepare_ime();
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        let args: Vec<String> = std::env::args()
            .skip(1)
            .filter(|arg| {
                arg != "--smoke"
                    && !arg.starts_with("--skip=")
                    && !arg.starts_with("--expect-screens=")
                    && !arg.starts_with("--mode=")
                    && !arg.starts_with("--ime=")
                    && arg != "--expect-fallback"
                    && arg != "--shm"
            })
            .collect();

        let mut options = X11PlatformOptions::new();
        options.rendering_mode = match mode().as_str() {
            "glx" => vec![X11RenderingMode::Glx, X11RenderingMode::Software],
            "egl" => vec![X11RenderingMode::Egl, X11RenderingMode::Software],
            _ => vec![X11RenderingMode::Software],
        };

        if std::env::args().any(|arg| arg == "--shm") {
            options.use_x_shm_framebuffer = Some(true);
        }

        let mut builder = AppBuilder::configure::<App>();
        if smoke {
            // What the framework logs at the level of warnings and above (why a rendering mode
            // did not initialize, for one), to the error stream.
            builder = builder.log_to_text_writer(std::io::stderr(), ferroui_base::logging::LogEventLevel::Warning, &[]);
        }
        let exit_code = builder
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

    /// A service that answers as the portal of IBus does, on the session
    /// bus of the environment (a private one: `dbus-run-session`). Not
    /// part of the platform: the smoke mode uses it to stand in for an
    /// input method daemon, so that what the input method does with a key
    /// is known.
    mod ibus_double {
        use std::sync::{Arc, Mutex, OnceLock};
        use zbus::zvariant::{ObjectPath, OwnedObjectPath, Structure, Value};

        pub const NAME: &str = "org.freedesktop.portal.IBus";
        const CONTEXT_PATH: &str = "/org/freedesktop/IBus/InputContext_1";
        const CONTEXT_INTERFACE: &str = "org.freedesktop.IBus.InputContext";
        /// The key symbol the double consumes, and the text it commits for it.
        pub const CONSUMED_KEY: u32 = 0x61;
        pub const COMMITTED_TEXT: &str = "\u{3042}";
        /// The bit of the state of a key event that says "release".
        const RELEASE_MASK: u32 = 1 << 30;

        type Log = Arc<Mutex<Vec<String>>>;

        struct Portal {
            log: Log,
        }

        #[zbus::interface(name = "org.freedesktop.IBus.Portal")]
        impl Portal {
            fn create_input_context(&self, client_name: &str) -> OwnedObjectPath {
                self.log.lock().unwrap().push(format!("CreateInputContext({client_name})"));
                ObjectPath::try_from(CONTEXT_PATH).unwrap().into()
            }
        }

        struct Context {
            log: Log,
        }

        #[zbus::interface(name = "org.freedesktop.IBus.InputContext")]
        impl Context {
            async fn process_key_event(
                &self,
                keyval: u32,
                keycode: u32,
                state: u32,
                #[zbus(connection)] connection: &zbus::Connection,
            ) -> bool {
                self.log.lock().unwrap().push(format!("ProcessKeyEvent({keyval:#x}, {keycode}, {state:#x})"));
                if keyval != CONSUMED_KEY {
                    return false;
                }
                if state & RELEASE_MASK == 0 {
                    // An IBusText: (sa{sv}sv), the text is its third member.
                    let attachments: std::collections::HashMap<String, Value<'static>> = Default::default();
                    let text = Value::Structure(Structure::from((
                        "IBusText".to_string(),
                        attachments,
                        COMMITTED_TEXT.to_string(),
                        Value::from(0u32),
                    )));
                    let _ = connection
                        .emit_signal(Option::<&str>::None, CONTEXT_PATH, CONTEXT_INTERFACE, "CommitText", &(text,))
                        .await;
                }
                true
            }

            fn set_cursor_location(&self, x: i32, y: i32, w: i32, h: i32) {
                self.log.lock().unwrap().push(format!("SetCursorLocation({x}, {y}, {w}, {h})"));
            }

            fn focus_in(&self) {
                self.log.lock().unwrap().push("FocusIn".to_string());
            }

            fn focus_out(&self) {
                self.log.lock().unwrap().push("FocusOut".to_string());
            }

            fn reset(&self) {
                self.log.lock().unwrap().push("Reset".to_string());
            }

            fn set_capabilities(&self, caps: u32) {
                self.log.lock().unwrap().push(format!("SetCapabilities({caps})"));
            }
        }

        struct Service {
            log: Log,
        }

        #[zbus::interface(name = "org.freedesktop.IBus.Service")]
        impl Service {
            fn destroy(&self) {
                self.log.lock().unwrap().push("Destroy".to_string());
            }
        }

        struct Double {
            _connection: zbus::blocking::Connection,
            log: Log,
        }

        static DOUBLE: OnceLock<Result<Double, String>> = OnceLock::new();

        /// Puts the service on the session bus. An error says why it could not.
        pub fn start() -> Result<(), String> {
            DOUBLE
                .get_or_init(|| {
                    let log: Log = Arc::default();
                    let connection = zbus::blocking::connection::Builder::session()
                        .and_then(|builder| builder.serve_at("/org/freedesktop/IBus", Portal { log: log.clone() }))
                        .and_then(|builder| builder.serve_at(CONTEXT_PATH, Context { log: log.clone() }))
                        .and_then(|builder| builder.serve_at(CONTEXT_PATH, Service { log: log.clone() }))
                        .and_then(|builder| builder.name(NAME))
                        .and_then(|builder| builder.build())
                        .map_err(|error| error.to_string())?;
                    Ok(Double { _connection: connection, log })
                })
                .as_ref()
                .map(|_| ())
                .map_err(Clone::clone)
        }

        /// The calls the service got so far.
        pub fn calls() -> Vec<String> {
            match DOUBLE.get() {
                Some(Ok(double)) => double.log.lock().unwrap().clone(),
                _ => Vec::new(),
            }
        }
    }

    /// The input method `--ime=` asks for: `ibus` (over D-Bus, against the
    /// double of this example) or `xim` (the input method of the server).
    fn ime() -> Option<String> {
        std::env::args().find_map(|arg| arg.strip_prefix("--ime=").map(str::to_string))
    }

    /// Sets the environment of the input method that is asked for, before
    /// the platform reads it, and starts the double of IBus.
    fn prepare_ime() {
        match ime().as_deref() {
            Some("ibus") => {
                std::env::set_var("FERROUI_IM_MODULE", "ibus");
                if let Err(error) = ibus_double::start() {
                    println!("The double of the input method service did not start: {error}");
                }
            }
            Some("xim") => {
                std::env::set_var("FERROUI_IM_MODULE", "xim");
                // The input method Xlib has built in: compose sequences, no server needed.
                if std::env::var_os("XMODIFIERS").is_none() {
                    std::env::set_var("XMODIFIERS", "@im=local");
                }
            }
            _ => {}
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
        use ferroui_base::input::text_input::ITextInputMethodImpl;
        use ferroui_base::input::Key;
        use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
        use ferroui_base::{Point, Thickness, Vector};
        use ferroui_controls::TextBox;
        use ferroui_freedesktop::IX11InputMethodFactory;
        use std::any::TypeId;
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
                let mode = mode();
                println!("Rendering mode asked for: {mode}");
                if std::env::args().any(|arg| arg == "--expect-fallback") {
                    // The mode is expected not to initialize (GLX on a renderer of its blacklist):
                    // the platform has to have gone on to software rendering.
                    report.phase("fallback");
                    let fell_back = platform.glx_graphics().is_none() && platform.egl_graphics().is_none();
                    report.check(
                        "software rendering took over",
                        fell_back,
                        format!("the platform has {} platform graphics", if fell_back { "no" } else { "the" }),
                    );
                } else if mode != "software" && options.runs("gpu") {
                    report.phase("gpu");
                    gpu_checks(&report, &platform, &window, &mode);
                }
                if std::env::args().any(|arg| arg == "--shm") {
                    report.phase("shm");
                    shm_checks(&report, &platform, &window);
                }
                if options.runs("input") {
                    report.phase("input");
                    input_checks(&report, &platform, &window).await;
                }
                if options.runs("popup") {
                    report.phase("popup");
                    popup_checks(&report, &platform, &window, &content).await;
                }
                if options.runs("resize") {
                    report.phase("resize");
                    resize_checks(&report, &platform, &window).await;
                }
                if options.runs("screens") {
                    report.phase("screens");
                    screens_checks(&report, &platform, &options);
                }
                if options.runs("clipboard") {
                    report.phase("clipboard");
                    clipboard_checks(&report, &window).await;
                }
                if let Some(kind) = ime() {
                    report.phase("ime");
                    ime_checks(&report, &platform, &window, &content, &kind).await;
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

        /// Rendering with OpenGL: the platform graphics of the mode, the
        /// render window, a context of the graphics that draws and reads
        /// back, and with GLX the frame of the window in its front buffer.
        fn gpu_checks(report: &Report, platform: &Rc<FerroX11Platform>, window: &Ref<Window>, mode: &str) {
            use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformGraphics, IPlatformGraphicsContext};
            use ferroui_opengl::gl_consts::{
                GL_BACK, GL_COLOR_ATTACHMENT0, GL_COLOR_BUFFER_BIT, GL_FRAMEBUFFER, GL_FRAMEBUFFER_COMPLETE, GL_RENDERBUFFER,
                GL_RGBA, GL_RGBA8, GL_UNSIGNED_BYTE,
            };
            use ferroui_opengl::IGlContext;
            use ferroui_x11::glx::GlxContext;
            use ferroui_x11::x11_window::X11Window;
            const GL_FRONT: i32 = 0x0404;

            let graphics: Option<Arc<dyn IPlatformGraphics>> = match mode {
                "glx" => platform.glx_graphics().map(|graphics| graphics as Arc<dyn IPlatformGraphics>),
                "egl" => platform.egl_graphics().map(|graphics| graphics as Arc<dyn IPlatformGraphics>),
                _ => None,
            };
            let Some(graphics) = graphics else {
                report.check(
                    "platform graphics",
                    false,
                    format!("the platform has no graphics of the mode {mode}: it fell back to software (the log says why)"),
                );
                return;
            };
            report.check("platform graphics", true, format!("the platform registered the graphics of {mode}"));

            let info = platform.info();
            let display = info.display();
            let Some(window_impl) = window.platform_impl() else {
                report.check("handle", false, "the window has no platform implementation".to_string());
                return;
            };
            let Some(x11_window) = window_impl.as_any().downcast_ref::<X11Window>() else {
                report.check("handle", false, "the window is not a window of the X11 platform".to_string());
                return;
            };
            let (xid, render_xid) = (x11_window.xid(), x11_window.render_xid());
            let parent = xlib::x_query_tree(display, render_xid).map(|(_, parent, _)| parent);
            let window_size = xlib::x_get_geometry(display, xid).map(|geometry| (geometry.width, geometry.height));
            let render_size = xlib::x_get_geometry(display, render_xid).map(|geometry| (geometry.width, geometry.height));
            let viewable = xlib::x_get_window_attributes(display, render_xid).map(|attributes| attributes.map_state == 2);
            report.check(
                "render window",
                render_xid != xid && parent == Some(xid) && render_size == window_size && viewable == Some(true),
                format!(
                    "{render_xid:#x}, a child of {parent:x?} (the window is {xid:#x}), {render_size:?} in a window of \
                     {window_size:?}, viewable {viewable:?}"
                ),
            );

            // A context of the platform graphics, as the compositor creates one.
            let context = graphics.create_context();
            let features: &dyn IOptionalFeatureProvider = &*context;
            let Some(gl_context) = features.try_get::<dyn IGlContext>() else {
                report.check("context", false, "the context of the platform graphics is not an OpenGL context".to_string());
                return;
            };
            let gl = gl_context.gl_interface();
            report.check(
                "context",
                gl.version().is_some() && gl.renderer().is_some(),
                format!(
                    "{:?} {}.{}; version {:?}, renderer {:?}, vendor {:?}; {} stencil bits, {} samples",
                    gl_context.version().type_(),
                    gl_context.version().major(),
                    gl_context.version().minor(),
                    gl.version(),
                    gl.renderer(),
                    gl.vendor(),
                    gl_context.stencil_size(),
                    gl_context.sample_count()
                ),
            );

            // Drawing and reading back, away from the window: a framebuffer of four by four pixels
            // cleared to the fill colour.
            {
                let current = gl_context.make_current();
                let framebuffer = gl.gen_framebuffer();
                gl.bind_framebuffer(GL_FRAMEBUFFER, framebuffer);
                let renderbuffer = gl.gen_renderbuffer();
                gl.bind_renderbuffer(GL_RENDERBUFFER, renderbuffer);
                gl.renderbuffer_storage(GL_RENDERBUFFER, GL_RGBA8, 4, 4);
                gl.framebuffer_renderbuffer(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_RENDERBUFFER, renderbuffer);
                let status = gl.check_framebuffer_status(GL_FRAMEBUFFER);
                gl.viewport(0, 0, 4, 4);
                gl.clear_color(FILL.0 as f32 / 255.0, FILL.1 as f32 / 255.0, FILL.2 as f32 / 255.0, 1.0);
                gl.clear(GL_COLOR_BUFFER_BIT);
                let mut pixel = [0u8; 4];
                // SAFETY: one pixel of four bytes is read into a buffer of four bytes.
                unsafe { gl.read_pixels(1, 1, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.as_mut_ptr().cast()) };
                let error = gl.get_error();
                gl.bind_framebuffer(GL_FRAMEBUFFER, 0);
                gl.delete_renderbuffer(renderbuffer);
                gl.delete_framebuffer(framebuffer);
                current.dispose();
                report.check(
                    "offscreen draw read back (glReadPixels)",
                    status == GL_FRAMEBUFFER_COMPLETE && pixel == [FILL.0, FILL.1, FILL.2, 255] && error == 0,
                    format!(
                        "{pixel:?}, {:?} expected; framebuffer status {status:#x}, error {error:#x}",
                        [FILL.0, FILL.1, FILL.2, 255]
                    ),
                );
            }

            // The frame of the window, from the buffers of its render window. A second context may
            // be current on a drawable of GLX; a window has one surface of EGL, which the
            // compositor holds, so with EGL the frame is checked through the server alone. Which
            // buffer holds the frame after it was presented is the implementation's choice: both
            // are read, and one of them has to hold it.
            if let Some(glx_context) = context.as_any().downcast_ref::<GlxContext>() {
                const NAME: &str = "frame read back (glReadPixels)";
                match (glx_context.make_current_with(render_xid), render_size) {
                    (Ok(current), Some((width, height))) => {
                        gl.bind_framebuffer(GL_FRAMEBUFFER, 0);
                        let read = |buffer: i32| {
                            if gl.is_read_buffer_available() {
                                gl.read_buffer(buffer);
                            }
                            let mut pixel = [0u8; 4];
                            // SAFETY: one pixel of four bytes is read into a buffer of four bytes.
                            unsafe {
                                gl.read_pixels(width / 2, height / 2, 1, 1, GL_RGBA, GL_UNSIGNED_BYTE, pixel.as_mut_ptr().cast())
                            };
                            pixel
                        };
                        let front = read(GL_FRONT);
                        let back = read(GL_BACK);
                        let error = gl.get_error();
                        current.dispose();
                        let expected = [FILL.0, FILL.1, FILL.2];
                        report.check(
                            NAME,
                            (front[..3] == expected || back[..3] == expected) && error == 0,
                            format!(
                                "centre ({}, {}): front buffer {front:?}, back buffer {back:?}, {expected:?} expected in \
                                 the first three of one of them; error {error:#x}",
                                width / 2,
                                height / 2
                            ),
                        );
                    }
                    (Err(error), _) => report.check(NAME, false, error.to_string()),
                    (Ok(current), None) => {
                        current.dispose();
                        report.check(NAME, false, "no geometry".to_string());
                    }
                }
            }

            IPlatformGraphicsContext::dispose(&*context);
        }

        /// Software rendering through the shared memory extension: the
        /// server has it, and the surface the renderer takes first is the
        /// shared memory one.
        fn shm_checks(report: &Report, platform: &Rc<FerroX11Platform>, window: &Ref<Window>) {
            use ferroui_x11::x_shm::X11ShmFramebufferSurface;

            let has_extension = platform.info().has_x_shm();
            report.check("extension (MIT-SHM)", has_extension, format!("the server has the extension: {has_extension}"));
            let Some(window_impl) = window.platform_impl() else {
                report.check("surface", false, "the window has no platform implementation".to_string());
                return;
            };
            let surfaces = window_impl.surfaces();
            let first_is_shm =
                surfaces.first().is_some_and(|surface| surface.as_any().downcast_ref::<X11ShmFramebufferSurface>().is_some());
            report.check(
                "surface",
                first_is_shm,
                format!("the window has {} surfaces; the first is the shared memory framebuffer: {first_is_shm}", surfaces.len()),
            );
        }

        /// Another size: the server has to show the window with it and the
        /// fill colour up to its new corner, which takes frames of the new
        /// size (a new framebuffer, new shared memory images, a resized
        /// render window).
        async fn resize_checks(report: &Report, platform: &Rc<FerroX11Platform>, window: &Ref<Window>) {
            const NEW_SIZE: (f64, f64) = (520.0, 320.0);
            let info = platform.info();
            let display = info.display();
            let Some(xid) = xid_of(window) else {
                report.check("handle", false, "the window has no platform handle".to_string());
                return;
            };
            let scaling = window.render_scaling();
            let expected = ((NEW_SIZE.0 * scaling) as i32, (NEW_SIZE.1 * scaling) as i32);
            window.set_width(NEW_SIZE.0);
            window.set_height(NEW_SIZE.1);

            let fill = rgb(FILL);
            let mut seen = (None, None, None);
            let resized = wait_for(STEP_TIMEOUT, || {
                xlib::x_sync(display, false);
                let size = xlib::x_get_geometry(display, xid).map(|geometry| (geometry.width, geometry.height));
                let corner = xlib::x_get_pixel(display, xid, expected.0 - 5, expected.1 - 5).map(|pixel| pixel as u64);
                let centre = xlib::x_get_pixel(display, xid, expected.0 / 2, expected.1 / 2).map(|pixel| pixel as u64);
                seen = (size, corner, centre);
                size == Some(expected)
                    && corner.map(|pixel| pixel & 0x00ff_ffff) == Some(fill)
                    && centre.map(|pixel| pixel & 0x00ff_ffff) == Some(fill)
            })
            .await;
            let client_size = window.client_size();
            report.check(
                "size and pixels after a resize",
                resized,
                format!(
                    "{:?} on the server, {expected:?} asked for, {}x{} in the framework; bottom right {:x?}, centre {:x?}, \
                     {fill:#08x} expected in the low 24 bits",
                    seen.0, client_size.width, client_size.height, seen.1, seen.2
                ),
            );
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

        /// Presses and releases the key that produces `key_sym`; false
        /// when no key of the keyboard mapping does.
        async fn tap(display: xlib::XDisplay, key_sym: u64) -> bool {
            let key_code = xtest::key_code(display, key_sym);
            if key_code == 0 {
                return false;
            }
            xtest::key(display, key_code, true);
            delay(Duration::from_millis(50)).await;
            xtest::key(display, key_code, false);
            delay(Duration::from_millis(50)).await;
            true
        }

        /// Text input through an input method: a text box gets the focus,
        /// the server synthesizes keys, and the text has to arrive the way
        /// the input method decides.
        ///
        /// With `ibus` the input method is the one over D-Bus, against the
        /// double of this example: it consumes the key "a" and commits
        /// another text for it, and lets every other key pass. With `xim`
        /// it is the input method of the server (the one Xlib has built
        /// in): keys produce their text through the input context, and a
        /// compose sequence produces one character.
        async fn ime_checks(
            report: &Report,
            platform: &Rc<FerroX11Platform>,
            window: &Ref<Window>,
            content: &Ref<Border>,
            kind: &str,
        ) {
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

            let has_factory = FerroLocator::current().get_service::<dyn IX11InputMethodFactory>().is_some();
            let has_feature = window_impl.try_get_feature(TypeId::of::<dyn ITextInputMethodImpl>()).is_some();
            match kind {
                "ibus" => {
                    report.check(
                        "input method over D-Bus",
                        has_factory,
                        format!(
                            "the platform registered {} input method factory for {}=ibus",
                            if has_factory { "an" } else { "no" },
                            ferroui_x11::x11_platform::IM_MODULE_VARIABLE
                        ),
                    );
                    let created = wait_for(STEP_TIMEOUT, || {
                        ibus_double::calls().iter().any(|call| call.starts_with("CreateInputContext("))
                    })
                    .await;
                    report.check(
                        "input context",
                        created,
                        format!("the service was called with {:?}", ibus_double::calls()),
                    );
                }
                _ => {
                    report.check(
                        "input method of the server",
                        info.has_xim() && !has_factory,
                        format!(
                            "XOpenIM for XMODIFIERS={:?} gave {} input method; a factory over D-Bus is {}",
                            std::env::var("XMODIFIERS").unwrap_or_default(),
                            if info.has_xim() { "an" } else { "no" },
                            if has_factory { "registered" } else { "not registered" }
                        ),
                    );
                }
            }
            report.check(
                "input method of the window",
                has_feature,
                format!("the window has {} text input method", if has_feature { "a" } else { "no" }),
            );

            // A text box in the corner of the window, with the focus, in an active window.
            let text_box = TextBox::new();
            text_box.set_width(240.0);
            text_box.set_height(32.0);
            text_box.set_horizontal_alignment(HorizontalAlignment::Left);
            text_box.set_vertical_alignment(VerticalAlignment::Top);
            text_box.set_margin(Thickness::uniform(8.0));
            content.set_child(&text_box);
            delay(Duration::from_millis(300)).await;
            // Key events go to the window with the focus or, without a window manager, to the one
            // under the pointer.
            if let Some((origin_x, origin_y, _)) = xlib::x_translate_coordinates(display, xid, info.root_window(), 0, 0) {
                xtest::motion(display, origin_x + 300, origin_y + 200);
            }
            window.activate();
            let focused = text_box.focus();
            let active = wait_for(STEP_TIMEOUT, || window.is_active()).await;
            report.check(
                "focus",
                focused && active,
                format!("the text box took the focus: {focused}; the window is active: {active}"),
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
                format!("{:?}", &log[log.len().saturating_sub(8)..])
            };
            let text_is = |expected: &str| text_box.text().as_deref() == Some(expected);

            if kind == "ibus" {
                let focus_in = wait_for(STEP_TIMEOUT, || ibus_double::calls().iter().any(|call| call == "FocusIn")).await;
                report.check(
                    "focus of the input context",
                    focus_in,
                    format!("the service was called with {:?}", ibus_double::calls()),
                );

                // The key the input method consumes: the application gets the committed text and
                // not the key.
                let committed = ibus_double::COMMITTED_TEXT;
                let tapped = tap(display, u64::from(ibus_double::CONSUMED_KEY)).await;
                let text = wait_for(STEP_TIMEOUT, || seen(&|recorded| *recorded == Recorded::Text(committed.to_string()))).await;
                delay(Duration::from_millis(300)).await;
                let key_passed = seen(&|recorded| matches!(recorded, Recorded::Key(_, Key::A, _)));
                report.check(
                    "committed text",
                    tapped && text && !key_passed,
                    format!(
                        "expected the text {committed:?} and no key event for the consumed key (seen: {key_passed}); \
                         last input {}",
                        tail()
                    ),
                );
                let in_text_box = wait_for(STEP_TIMEOUT, || text_is(committed)).await;
                report.check(
                    "committed text in the text box",
                    in_text_box,
                    format!("the text box has {:?}, expected {committed:?}", text_box.text()),
                );

                // A key the input method lets pass: the key and its text, after the text before.
                log.borrow_mut().clear();
                let tapped = tap(display, 0x62).await;
                let key_down = wait_for(STEP_TIMEOUT, || {
                    seen(&|recorded| matches!(recorded, Recorded::Key(RawKeyEventType::KeyDown, Key::B, _)))
                })
                .await;
                let key_up = wait_for(STEP_TIMEOUT, || {
                    seen(&|recorded| matches!(recorded, Recorded::Key(RawKeyEventType::KeyUp, Key::B, _)))
                })
                .await;
                let expected = format!("{committed}b");
                let in_text_box = wait_for(STEP_TIMEOUT, || text_is(&expected)).await;
                report.check(
                    "a key the input method passes on",
                    tapped && key_down && key_up && in_text_box,
                    format!(
                        "down {key_down}, up {key_up}; the text box has {:?}, expected {expected:?}; last input {}",
                        text_box.text(),
                        tail()
                    ),
                );

                // The input method was offered both keys, each press before its release.
                let offered: Vec<String> =
                    ibus_double::calls().into_iter().filter(|call| call.starts_with("ProcessKeyEvent(")).collect();
                let key_code_a = xtest::key_code(display, 0x61);
                let key_code_b = xtest::key_code(display, 0x62);
                let expected_calls = vec![
                    format!("ProcessKeyEvent(0x61, {key_code_a}, 0x0)"),
                    format!("ProcessKeyEvent(0x61, {key_code_a}, 0x40000000)"),
                    format!("ProcessKeyEvent(0x62, {key_code_b}, 0x0)"),
                    format!("ProcessKeyEvent(0x62, {key_code_b}, 0x40000000)"),
                ];
                report.check(
                    "keys offered to the input method",
                    offered == expected_calls,
                    format!("{offered:?}, expected {expected_calls:?}"),
                );

                // The cursor of the text box is reported in pixels of the screen: a place inside the
                // window as the server has it.
                let origin = xlib::x_translate_coordinates(display, xid, info.root_window(), 0, 0);
                let attributes = xlib::x_get_window_attributes(display, xid);
                let location = ibus_double::calls().iter().rev().find_map(|call| {
                    let numbers = call.strip_prefix("SetCursorLocation(")?.strip_suffix(')')?;
                    let numbers: Vec<i32> = numbers.split(", ").filter_map(|number| number.parse().ok()).collect();
                    (numbers.len() == 4).then(|| (numbers[0], numbers[1], numbers[2], numbers[3]))
                });
                let inside = match (location, origin, &attributes) {
                    (Some((x, y, _, height)), Some((origin_x, origin_y, _)), Some(attributes)) => {
                        x >= origin_x
                            && y >= origin_y
                            && x <= origin_x + attributes.width
                            && y + height <= origin_y + attributes.height
                            && height > 0
                    }
                    _ => false,
                };
                report.check(
                    "cursor location",
                    inside,
                    format!(
                        "the last location reported is {location:?}; the window is at {:?} of the root",
                        origin.map(|(x, y, _)| (x, y))
                    ),
                );
            } else {
                // A key produces its text through the input context.
                let tapped = tap(display, 0x61).await;
                let text = wait_for(STEP_TIMEOUT, || seen(&|recorded| *recorded == Recorded::Text("a".to_string()))).await;
                let in_text_box = wait_for(STEP_TIMEOUT, || text_is("a")).await;
                report.check(
                    "text through the input context",
                    tapped && text && in_text_box,
                    format!("the text box has {:?}, expected \"a\"; last input {}", text_box.text(), tail()),
                );

                // A compose sequence (the compose key, an apostrophe, "e") is one character. The
                // input method filters the keys of the sequence, so only the input method of the
                // server gives this text. The caller gives a key the compose symbol
                // (`xmodmap -e "keycode 135 = Multi_key"`).
                log.borrow_mut().clear();
                let composed = "\u{e9}";
                if xtest::key_code(display, 0xff20) == 0 {
                    report.check(
                        "compose sequence",
                        false,
                        "no key of the keyboard mapping is the compose key (Multi_key)".to_string(),
                    );
                } else {
                    let tapped = tap(display, 0xff20).await && tap(display, 0x27).await && tap(display, 0x65).await;
                    let text =
                        wait_for(STEP_TIMEOUT, || seen(&|recorded| *recorded == Recorded::Text(composed.to_string()))).await;
                    let expected = format!("a{composed}");
                    let in_text_box = wait_for(STEP_TIMEOUT, || text_is(&expected)).await;
                    report.check(
                        "compose sequence",
                        tapped && text && in_text_box,
                        format!(
                            "the text box has {:?}, expected {expected:?} (locale {:?}); last input {}",
                            text_box.text(),
                            std::env::var("LANG").unwrap_or_default(),
                            tail()
                        ),
                    );
                }
            }

            window_impl.set_input(framework_input);
            content.set_child(None::<Ref<Control>>);
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
