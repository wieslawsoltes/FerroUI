//! A window on a Wayland compositor, and the smoke mode of the backend.
//!
//! ```sh
//! cargo run -p ferroui-wayland --features example --example wayland_window
//! cargo run -p ferroui-wayland --features example --example wayland_window -- --smoke --mode=egl
//! ```
//!
//! Without arguments it opens a window with a filled background and runs
//! until the window is closed.
//!
//! With `--smoke` it checks the backend against the compositor and exits
//! with 0 when every check passed. Every check prints a line that begins
//! with `[ ok ]` or `[FAILED]`. A Wayland client cannot ask the compositor
//! about its own window, synthesize input or read the screen through the
//! core protocol, so the smoke mode needs **`sway`** (any compositor of
//! wlroots with its protocols would do for the second half):
//!
//! - what the compositor knows of the window and of the outputs is asked
//!   through its IPC (`swaymsg`, with `SWAYSOCK` in the environment), which
//!   also changes the mode of the output and closes the window;
//! - the example is its own test client: a second connection binds the
//!   screen copy manager (`zwlr_screencopy_manager_v1`), the virtual pointer
//!   (`zwlr_virtual_pointer_manager_v1`) and the virtual keyboard
//!   (`zwp_virtual_keyboard_manager_v1`), reads the composed output and
//!   synthesizes input.
//!
//! The configuration of the compositor has to give the window the whole
//! output (no bar, no borders), so that a point of the window is the same
//! point of the output. Options:
//!
//! - `--mode=software` (the default) renders through `wl_shm` buffers: the
//!   list of OpenGL profiles is empty, so no display of EGL is created;
//!   `--mode=egl` renders through EGL and fails when the worker has no GPU
//!   backend; `--mode=dmabuf` asks for the dmabuf swapchain
//!   (`use_dmabuf_swapchain`): where the compositor has
//!   `zwp_linux_dmabuf_v1` the worker has to render through it, and where it
//!   has not (sway on the software renderer of wlroots) the backend falls
//!   back to a `wl_egl_window`, which the run reports as not measured.
//! - `--expect-scale=N`: the scale of the output, which the window has to
//!   take.
//! - `--skip=a,b`: phases to leave out (`screens`, `foreign`, `frames`, `input`, `popup`,
//!   `clipboard`, `dragdrop`, `textinput`, `export`, `decorations`, `cursor`, `resize`, `state`,
//!   `title`).
//!
//! The phase `foreign` starts a second worker of the backend on a display
//! another library opened (the `wl_display` of a connection the example
//! makes with `wayland-client`, handed over as a foreign display) and checks
//! that it binds its globals there and that the display stays the other
//! library's.
//!
//! The phase `textinput` makes the test client the input method of the seat
//! (`zwp_input_method_manager_v2`, which sway has): it is activated when a
//! text box of the window takes the focus, sends a preedit, commits text and
//! deletes around the cursor, and maps an input popup, whose place and
//! rectangle are the cursor rectangle the window sent.
//!
//! With `--input-method=TEXT` (and optionally `--input-method-preedit=TEXT`)
//! the example is no application but that input method alone, for another
//! client: it waits until a text input is enabled, commits the text, sets
//! the preedit, and ends when the text input is disabled or after a minute.
//! `scripts/wayland/catalog-pictures.sh` types into the ControlCatalog with it.
//!
//! Touch is not checked: wlroots has no protocol that synthesizes it.
//!
//! The feature `example-check` builds the example without the renderer and
//! the text shaper, whose native libraries cannot be built for Linux on
//! another system: it is how the example is type-checked from there. An
//! example built that way does not run.

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("wayland_window: this example needs a Wayland compositor, which this system does not have");
}

#[cfg(target_os = "linux")]
fn main() -> std::process::ExitCode {
    app::run()
}

#[cfg(target_os = "linux")]
mod app {
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::Color;
    use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, Ref, Thickness};
    use ferroui_controls::{AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication, Window};
    use ferroui_wayland::{FerroWaylandPlatformExtensions, WaylandPlatformOptions};
    use std::cell::{Cell, RefCell};
    use std::process::ExitCode;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    const TITLE: &str = "FerroUI Wayland window";
    const APP_ID: &str = "org.ferroui.WaylandWindow";
    /// The colour the window is filled with.
    const FILL: (u8, u8, u8) = (0x33, 0x66, 0x99);
    /// A square of another colour at a known place: it shows which way up and at which scale
    /// a frame arrived.
    const MARKER_ORIGIN: (f64, f64) = (100.0, 60.0);
    const MARKER_SIZE: f64 = 40.0;
    const MARKER_FILL: (u8, u8, u8) = (0xcc, 0x33, 0x00);

    /// The colours of the popups the smoke mode opens: one under the marker, one beside it.
    const POPUP_FILL: (u8, u8, u8) = (0xee, 0xcc, 0x22);
    const POPUP_SIZE: (f64, f64) = (160.0, 90.0);
    const NESTED_FILL: (u8, u8, u8) = (0x22, 0xaa, 0x55);
    const NESTED_SIZE: (f64, f64) = (80.0, 40.0);

    ferroui_base::ferro_thread_local! {
        /// The marker of the main window: what the popup of the smoke mode is placed at.
        static MARKER: RefCell<Option<Ref<Border>>> = const { RefCell::new(None) };
        /// The drag and drop of the smoke mode: what a press on the marker drags, what the
        /// window answers a drag with, and what it saw.
        static DRAG: RefCell<DragProbe> = RefCell::new(DragProbe::default());
    }

    /// The text a press on the marker drags in the smoke mode.
    const DRAG_TEXT: &str = "dragged by the smoke mode";

    /// What the window does with a drag in the smoke mode, and what it saw of one.
    #[derive(Default)]
    struct DragProbe {
        /// A press on the marker starts a drag.
        armed: bool,
        /// The window answers a drag with no effect.
        refuse: bool,
        /// The events of the drop target: the kind, the place, the text of the data.
        events: Vec<(&'static str, ferroui_base::Point, Option<String>)>,
        /// The result of the drag the marker started.
        result: Option<ferroui_base::input::DragDropEffects>,
    }

    /// Makes `target` a drop target that records what it sees and `source` the source of a
    /// drag of a text, when the smoke mode armed it.
    fn attach_drag_and_drop(target: &Ref<Border>, source: &Ref<Border>) {
        use ferroui_base::input::{DataTransfer, DataTransferExtensions, DataTransferItem, DragDrop, DragDropEffects, DragEventArgs, InputElement};
        use ferroui_base::interactivity::Interactive;

        DragDrop::set_allow_drop(target, true);
        let weak_target = target.downgrade();
        let record = |kind: &'static str| {
            let weak_target = weak_target.clone();
            move |_: &Interactive, e: &DragEventArgs| {
                let position = weak_target.upgrade().map(|target| e.get_position(&target)).unwrap_or_default();
                let refuse = DRAG.with(|drag| {
                    let mut drag = drag.borrow_mut();
                    let text = if kind == "drop" { e.data_transfer().try_get_text() } else { None };
                    drag.events.push((kind, position, text));
                    drag.refuse
                });
                e.set_drag_effects(if refuse { DragDropEffects::NONE } else { DragDropEffects::COPY });
            }
        };
        DragDrop::add_drag_enter_handler(target, record("enter"));
        DragDrop::add_drag_over_handler(target, record("over"));
        DragDrop::add_drop_handler(target, record("drop"));
        DragDrop::add_drag_leave_handler(target, |_, _| {
            DRAG.with(|drag| drag.borrow_mut().events.push(("leave", ferroui_base::Point::default(), None)));
        });

        source.add_handler(&InputElement::pointer_pressed_event(), |_, e| {
            if !DRAG.with(|drag| drag.borrow().armed) {
                return;
            }
            let transfer = DataTransfer::new();
            transfer.add(DataTransferItem::create_text(Some(DRAG_TEXT)));
            let drag = DragDrop::do_drag_drop_async(e, transfer, DragDropEffects::COPY | DragDropEffects::MOVE);
            drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                let result = drag.await;
                DRAG.with(|drag| drag.borrow_mut().result = Some(result));
            }));
        });
    }

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
            #[cfg(feature = "example")]
            this.styles().add(ferroui_themes_simple::SimpleTheme::new().upcast::<ferroui_base::styling::Styles>());
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

    fn brush(colour: (u8, u8, u8)) -> Rc<ImmutableSolidColorBrush> {
        Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(colour.0, colour.1, colour.2)))
    }

    fn create_main_window() -> Ref<Window> {
        let marker = Border::new();
        marker.set_background(Some(brush(MARKER_FILL)));
        marker.set_width(MARKER_SIZE);
        marker.set_height(MARKER_SIZE);
        marker.set_horizontal_alignment(HorizontalAlignment::Left);
        marker.set_vertical_alignment(VerticalAlignment::Top);
        marker.set_margin(Thickness::new(MARKER_ORIGIN.0, MARKER_ORIGIN.1, 0.0, 0.0));

        let border = Border::new();
        border.set_background(Some(brush(FILL)));
        border.set_child(&marker);
        MARKER.with(|cell| *cell.borrow_mut() = Some(marker.clone()));
        attach_drag_and_drop(&border, &marker);

        let window = Window::new();
        window.set_title(Some(TITLE.to_string()));
        window.set_content(Some(Control::boxed(border)));

        window.opened(|| println!("Window opened"));
        window.closed(|| println!("Window closed"));

        if std::env::args().any(|arg| arg == "--smoke") {
            smoke::start(&window);
        }

        window
    }

    /// The rendering mode `--mode=` asks for: `software` without the option.
    fn mode() -> String {
        std::env::args().find_map(|arg| arg.strip_prefix("--mode=").map(str::to_string)).unwrap_or("software".to_string())
    }

    /// The mode `--input-method=TEXT`: the input method of the seat for another client.
    fn run_input_method(text: &str, preedit: Option<&str>) -> ExitCode {
        let step = Duration::from_millis(100);
        let mut probe = match probe::Probe::connect() {
            Ok(probe) => probe,
            Err(error) => {
                println!("Input method: no connection: {error}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(error) = probe.input_method_start() {
            println!("Input method: {error}");
            return ExitCode::FAILURE;
        }
        println!("Input method: waiting for a text input");
        let started = std::time::Instant::now();
        while !probe.input_method().active {
            if !probe.alive() || probe.input_method().unavailable || started.elapsed() > Duration::from_secs(120) {
                println!("Input method: no text input was enabled (unavailable: {})", probe.input_method().unavailable);
                return ExitCode::FAILURE;
            }
            std::thread::sleep(step);
        }
        // The client sends its state in several commits: wait until it is quiet.
        let mut serial = probe.input_method().done_count;
        loop {
            std::thread::sleep(step * 3);
            let now = probe.input_method().done_count;
            if now == serial {
                break;
            }
            serial = now;
        }
        let state = probe.input_method();
        println!("Input method: activated; content type {:?}, surrounding text {:?}", state.content_type, state.surrounding);
        let sent = probe.input_method_send(None, Some(text), None);
        println!("Input method: committed {text:?}: {sent:?}");
        if let Some(preedit) = preedit {
            std::thread::sleep(step * 5);
            let length = preedit.len() as i32;
            let sent = probe.input_method_send(None, None, Some((preedit, length, length)));
            println!("Input method: preedit {preedit:?}: {sent:?}");
        }
        let started = std::time::Instant::now();
        while probe.alive() && probe.input_method().active && started.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(step);
        }
        println!("Input method: done; the last surrounding text was {:?}", probe.input_method().surrounding);
        probe.input_method_stop();
        ExitCode::SUCCESS
    }

    pub fn run() -> ExitCode {
        if let Some(text) = std::env::args().find_map(|arg| arg.strip_prefix("--input-method=").map(str::to_string)) {
            let preedit = std::env::args().find_map(|arg| arg.strip_prefix("--input-method-preedit=").map(str::to_string));
            return run_input_method(&text, preedit.as_deref());
        }
        let smoke = std::env::args().any(|arg| arg == "--smoke");
        let args: Vec<String> = std::env::args()
            .skip(1)
            .filter(|arg| {
                arg != "--smoke"
                    && !arg.starts_with("--skip=")
                    && !arg.starts_with("--mode=")
                    && !arg.starts_with("--expect-scale=")
            })
            .collect();

        let mut options = WaylandPlatformOptions::new();
        options.app_id = Some(APP_ID.to_string());
        match mode().as_str() {
            "egl" => {}
            // The swapchain the backend allocates with GBM and hands over as dma-bufs.
            "dmabuf" => options.use_dmabuf_swapchain = Some(true),
            // No profile to make a context with: the display of EGL is not created, and the
            // worker renders through shared memory buffers.
            _ => options.gl_profiles = Vec::new(),
        }

        let mut builder = AppBuilder::configure::<App>();
        if smoke {
            // What the framework logs at the level of warnings and above, to the error stream.
            builder = builder.log_to_text_writer(std::io::stderr(), ferroui_base::logging::LogEventLevel::Warning, &[]);
        }
        let builder = builder.with(Rc::new(options));
        #[cfg(feature = "example")]
        let builder = {
            use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
            use ferroui_skia::SkiaApplicationExtensions;
            builder.use_harfbuzz().use_skia()
        };
        let exit_code = builder.use_wayland().start_with_classic_desktop_lifetime(&args);
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

    /// A value of JSON, as much of it as the answers of the compositor need.
    mod json {
        #[derive(Clone, Debug, PartialEq)]
        pub enum Json {
            Null,
            Bool(bool),
            Number(f64),
            String(String),
            Array(Vec<Json>),
            Object(Vec<(String, Json)>),
        }

        impl Json {
            pub fn get(&self, key: &str) -> Option<&Json> {
                match self {
                    Json::Object(members) => members.iter().find(|(name, _)| name == key).map(|(_, value)| value),
                    _ => None,
                }
            }

            pub fn as_str(&self) -> Option<&str> {
                match self {
                    Json::String(value) => Some(value),
                    _ => None,
                }
            }

            pub fn as_f64(&self) -> Option<f64> {
                match self {
                    Json::Number(value) => Some(*value),
                    _ => None,
                }
            }

            pub fn as_bool(&self) -> Option<bool> {
                match self {
                    Json::Bool(value) => Some(*value),
                    _ => None,
                }
            }

            pub fn as_array(&self) -> &[Json] {
                match self {
                    Json::Array(items) => items,
                    _ => &[],
                }
            }

            /// Every object of the value, depth first, itself included.
            pub fn objects(&self) -> Vec<&Json> {
                let mut found = Vec::new();
                self.collect(&mut found);
                found
            }

            fn collect<'a>(&'a self, found: &mut Vec<&'a Json>) {
                match self {
                    Json::Object(members) => {
                        found.push(self);
                        for (_, value) in members {
                            value.collect(found);
                        }
                    }
                    Json::Array(items) => {
                        for item in items {
                            item.collect(found);
                        }
                    }
                    _ => {}
                }
            }
        }

        pub fn parse(text: &str) -> Option<Json> {
            let chars: Vec<char> = text.chars().collect();
            let mut position = 0;
            let value = value(&chars, &mut position)?;
            Some(value)
        }

        fn skip(chars: &[char], position: &mut usize) {
            while *position < chars.len() && chars[*position].is_whitespace() {
                *position += 1;
            }
        }

        fn value(chars: &[char], position: &mut usize) -> Option<Json> {
            skip(chars, position);
            match *chars.get(*position)? {
                '{' => {
                    *position += 1;
                    let mut members = Vec::new();
                    loop {
                        skip(chars, position);
                        if *chars.get(*position)? == '}' {
                            *position += 1;
                            return Some(Json::Object(members));
                        }
                        let Json::String(name) = string(chars, position)? else {
                            return None;
                        };
                        skip(chars, position);
                        if *chars.get(*position)? != ':' {
                            return None;
                        }
                        *position += 1;
                        members.push((name, value(chars, position)?));
                        skip(chars, position);
                        if *chars.get(*position)? == ',' {
                            *position += 1;
                        }
                    }
                }
                '[' => {
                    *position += 1;
                    let mut items = Vec::new();
                    loop {
                        skip(chars, position);
                        if *chars.get(*position)? == ']' {
                            *position += 1;
                            return Some(Json::Array(items));
                        }
                        items.push(value(chars, position)?);
                        skip(chars, position);
                        if *chars.get(*position)? == ',' {
                            *position += 1;
                        }
                    }
                }
                '"' => string(chars, position),
                't' | 'f' | 'n' => {
                    for (word, json) in [("true", Json::Bool(true)), ("false", Json::Bool(false)), ("null", Json::Null)] {
                        let end = *position + word.len();
                        if end <= chars.len() && chars[*position..end].iter().collect::<String>() == word {
                            *position = end;
                            return Some(json);
                        }
                    }
                    None
                }
                _ => {
                    let start = *position;
                    while *position < chars.len() && matches!(chars[*position], '0'..='9' | '-' | '+' | '.' | 'e' | 'E') {
                        *position += 1;
                    }
                    chars[start..*position].iter().collect::<String>().parse().ok().map(Json::Number)
                }
            }
        }

        fn string(chars: &[char], position: &mut usize) -> Option<Json> {
            if *chars.get(*position)? != '"' {
                return None;
            }
            *position += 1;
            let mut text = String::new();
            loop {
                let c = *chars.get(*position)?;
                *position += 1;
                match c {
                    '"' => return Some(Json::String(text)),
                    '\\' => {
                        let escaped = *chars.get(*position)?;
                        *position += 1;
                        match escaped {
                            'n' => text.push('\n'),
                            't' => text.push('\t'),
                            'r' => text.push('\r'),
                            'b' => text.push('\u{8}'),
                            'f' => text.push('\u{c}'),
                            'u' => {
                                let digits: String = chars.get(*position..*position + 4)?.iter().collect();
                                *position += 4;
                                text.push(char::from_u32(u32::from_str_radix(&digits, 16).ok()?).unwrap_or('\u{fffd}'));
                            }
                            other => text.push(other),
                        }
                    }
                    other => text.push(other),
                }
            }
        }
    }

    /// The compositor's own account of itself, through its IPC.
    mod sway {
        use super::json::{self, Json};
        use std::process::Command;

        /// Runs `swaymsg` and returns what it printed; `None` when it cannot be run or fails.
        pub fn message(arguments: &[&str]) -> Option<String> {
            let output = Command::new("swaymsg").args(arguments).output().ok()?;
            if !output.status.success() {
                eprintln!("swaymsg {arguments:?} failed: {}", String::from_utf8_lossy(&output.stderr).trim());
                return None;
            }
            Some(String::from_utf8_lossy(&output.stdout).into_owned())
        }

        pub fn outputs() -> Option<Json> {
            json::parse(&message(&["-t", "get_outputs", "-r"])?)
        }

        /// The view of a title in the tree of the compositor.
        pub fn view(title: &str) -> Option<Json> {
            let tree = json::parse(&message(&["-t", "get_tree", "-r"])?)?;
            tree.objects()
                .into_iter()
                .find(|node| {
                    node.get("name").and_then(Json::as_str) == Some(title)
                        && node.get("pid").is_some()
                        && node.get("type").and_then(Json::as_str).is_some_and(|kind| kind == "con" || kind == "floating_con")
                })
                .cloned()
        }

        /// A rectangle member of a node: x, y, width, height.
        pub fn rect(node: &Json, member: &str) -> Option<(i32, i32, i32, i32)> {
            let rect = node.get(member)?;
            let field = |name: &str| rect.get(name).and_then(Json::as_f64).map(|value| value as i32);
            Some((field("x")?, field("y")?, field("width")?, field("height")?))
        }
    }

    /// The test client: a second connection to the compositor that reads the composed output
    /// and synthesizes input, through the protocols wlroots has for that.
    mod probe {
        use ferroui_wayland::server::interop::pipe2_stream::{Pipe2Stream, PipeDirection};
        use ferroui_wayland::server::interop::unsafe_native_methods::{memfd_of_length, pipe2_cloexec, MemoryMapping};
        use std::collections::HashMap;
        use std::io::Write;
        use std::os::fd::AsFd;
        use std::sync::mpsc::{channel, Receiver};
        use std::sync::Arc;
        use wayland_client::backend::ObjectId;
        use wayland_client::{event_created_child, Proxy};
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1};
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_manager_v1::ZwlrDataControlManagerV1;
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1};
        use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1};
        use std::os::fd::OwnedFd;
        use wayland_client::protocol::wl_buffer::WlBuffer;
        use wayland_client::protocol::wl_compositor::WlCompositor;
        use wayland_client::protocol::wl_output::WlOutput;
        use wayland_client::protocol::wl_surface::WlSurface;
        use wayland_protocols::xdg::foreign::zv2::client::zxdg_imported_v2::{self, ZxdgImportedV2};
        use wayland_protocols::xdg::foreign::zv2::client::zxdg_importer_v2::ZxdgImporterV2;
        use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_manager_v2::ZwpInputMethodManagerV2;
        use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_method_v2::{self, ZwpInputMethodV2};
        use wayland_protocols_misc::zwp_input_method_v2::client::zwp_input_popup_surface_v2::{self, ZwpInputPopupSurfaceV2};
        use wayland_client::protocol::wl_registry::{self, WlRegistry};
        use wayland_client::protocol::wl_seat::WlSeat;
        use wayland_client::protocol::wl_shm::{self, WlShm};
        use wayland_client::protocol::wl_shm_pool::WlShmPool;
        use wayland_client::protocol::{wl_keyboard, wl_pointer};
        use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle, WEnum};
        use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1;
        use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1;
        use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1};
        use wayland_protocols_wlr::screencopy::v1::client::zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1;
        use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1;
        use wayland_protocols_wlr::virtual_pointer::v1::client::zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1;

        /// The buffer a frame of the screen copy protocol asks for.
        #[derive(Clone, Copy, Debug)]
        struct BufferInfo {
            format: wl_shm::Format,
            width: u32,
            height: u32,
            stride: u32,
        }

        /// What the input method of the test client was told: the state of the text input of
        /// the focused client, as of the last `done`.
        #[derive(Clone, Debug, Default)]
        pub struct InputMethodState {
            pub active: bool,
            pub activations: u32,
            pub deactivations: u32,
            /// The number of `done` events: the serial a commit of the input method names.
            pub done_count: u32,
            /// The seat has an input method already.
            pub unavailable: bool,
            /// The text around the cursor, and the cursor and the anchor in bytes.
            pub surrounding: Option<(String, u32, u32)>,
            /// The content hint and the content purpose, as numbers of the protocol.
            pub content_type: Option<(u32, u32)>,
            /// The rectangles the input popup was given, relative to the popup.
            pub rectangles: Vec<(i32, i32, i32, i32)>,
        }

        #[derive(Default)]
        struct State {
            /// The compositor said that the top-level this client imported is gone.
            imported_destroyed: bool,
            /// The input method as of the last `done`, and what arrived since.
            input_method: InputMethodState,
            pending_input_method: InputMethodState,
            globals: Vec<(u32, String, u32)>,
            buffer: Option<BufferInfo>,
            y_invert: bool,
            ready: bool,
            failed: bool,
            /// The offers the data control device announced, with their MIME types.
            offers: HashMap<ObjectId, (ZwlrDataControlOfferV1, Vec<String>)>,
            /// The offer that is the selection.
            selection: Option<ObjectId>,
            /// How many times the selection changed.
            selections: u32,
            /// What the source of the probe gives, by MIME type.
            outgoing: Vec<(String, Arc<Vec<u8>>)>,
            /// The MIME types the source of the probe was asked for.
            sent: Vec<String>,
            source_cancelled: bool,
        }

        impl Dispatch<ZwlrDataControlDeviceV1, ()> for State {
            fn event(
                state: &mut Self,
                _: &ZwlrDataControlDeviceV1,
                event: zwlr_data_control_device_v1::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                match event {
                    zwlr_data_control_device_v1::Event::DataOffer { id } => {
                        state.offers.insert(id.id(), (id, Vec::new()));
                    }
                    zwlr_data_control_device_v1::Event::Selection { id } => {
                        let selection = id.map(|offer| offer.id());
                        state.selections += 1;
                        // The offers before this one are over.
                        let stale: Vec<ObjectId> = state.offers.keys().filter(|key| Some(*key) != selection.as_ref()).cloned().collect();
                        for key in stale {
                            if let Some((offer, _)) = state.offers.remove(&key) {
                                offer.destroy();
                            }
                        }
                        state.selection = selection;
                    }
                    zwlr_data_control_device_v1::Event::PrimarySelection { id: Some(offer) } => {
                        state.offers.remove(&offer.id());
                        offer.destroy();
                    }
                    _ => {}
                }
            }

            event_created_child!(State, ZwlrDataControlDeviceV1, [
                zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
            ]);
        }

        impl Dispatch<ZwlrDataControlOfferV1, ()> for State {
            fn event(
                state: &mut Self,
                proxy: &ZwlrDataControlOfferV1,
                event: zwlr_data_control_offer_v1::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
                    if let Some((_, mime_types)) = state.offers.get_mut(&proxy.id()) {
                        mime_types.push(mime_type);
                    }
                }
            }
        }

        impl Dispatch<ZwlrDataControlSourceV1, ()> for State {
            fn event(
                state: &mut Self,
                _: &ZwlrDataControlSourceV1,
                event: zwlr_data_control_source_v1::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                match event {
                    zwlr_data_control_source_v1::Event::Send { mime_type, fd } => {
                        let data = state.outgoing.iter().find(|(mime, _)| *mime == mime_type).map(|(_, data)| data.clone());
                        state.sent.push(mime_type);
                        // Written on a thread of its own: the reader is the application, whose
                        // UI thread is the thread of this call.
                        std::thread::spawn(move || {
                            let mut stream = Pipe2Stream::new(fd, PipeDirection::Out);
                            if let Some(data) = data {
                                let _ = stream.write_all(&data);
                            }
                        });
                    }
                    zwlr_data_control_source_v1::Event::Cancelled => state.source_cancelled = true,
                    _ => {}
                }
            }
        }

        impl Dispatch<ZwpInputMethodV2, ()> for State {
            fn event(
                state: &mut Self,
                _: &ZwpInputMethodV2,
                event: zwp_input_method_v2::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                let pending = &mut state.pending_input_method;
                match event {
                    zwp_input_method_v2::Event::Activate => {
                        // An activation starts from nothing.
                        pending.active = true;
                        pending.surrounding = None;
                        pending.content_type = None;
                    }
                    zwp_input_method_v2::Event::Deactivate => pending.active = false,
                    zwp_input_method_v2::Event::SurroundingText { text, cursor, anchor } => {
                        pending.surrounding = Some((text, cursor, anchor));
                    }
                    zwp_input_method_v2::Event::ContentType { hint, purpose } => {
                        pending.content_type = Some((u32::from(hint), u32::from(purpose)));
                    }
                    zwp_input_method_v2::Event::Done => {
                        let current = &mut state.input_method;
                        if pending.active && !current.active {
                            current.activations += 1;
                        }
                        if !pending.active && current.active {
                            current.deactivations += 1;
                        }
                        current.active = pending.active;
                        current.surrounding = pending.surrounding.clone();
                        current.content_type = pending.content_type;
                        current.done_count += 1;
                    }
                    zwp_input_method_v2::Event::Unavailable => state.input_method.unavailable = true,
                    _ => {}
                }
            }
        }

        impl Dispatch<ZxdgImportedV2, ()> for State {
            fn event(state: &mut Self, _: &ZxdgImportedV2, event: zxdg_imported_v2::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
                if let zxdg_imported_v2::Event::Destroyed = event {
                    state.imported_destroyed = true;
                }
            }
        }

        impl Dispatch<ZwpInputPopupSurfaceV2, ()> for State {
            fn event(
                state: &mut Self,
                _: &ZwpInputPopupSurfaceV2,
                event: zwp_input_popup_surface_v2::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                if let zwp_input_popup_surface_v2::Event::TextInputRectangle { x, y, width, height } = event {
                    state.input_method.rectangles.push((x, y, width, height));
                }
            }
        }

        impl Dispatch<WlRegistry, ()> for State {
            fn event(state: &mut Self, _: &WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
                if let wl_registry::Event::Global { name, interface, version } = event {
                    state.globals.push((name, interface, version));
                }
            }
        }

        impl Dispatch<ZwlrScreencopyFrameV1, ()> for State {
            fn event(
                state: &mut Self,
                _: &ZwlrScreencopyFrameV1,
                event: zwlr_screencopy_frame_v1::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
                match event {
                    zwlr_screencopy_frame_v1::Event::Buffer { format: WEnum::Value(format), width, height, stride } => {
                        state.buffer = Some(BufferInfo { format, width, height, stride });
                    }
                    zwlr_screencopy_frame_v1::Event::Flags { flags: WEnum::Value(flags) } => {
                        state.y_invert = flags.contains(zwlr_screencopy_frame_v1::Flags::YInvert);
                    }
                    zwlr_screencopy_frame_v1::Event::Ready { .. } => state.ready = true,
                    zwlr_screencopy_frame_v1::Event::Failed => state.failed = true,
                    _ => {}
                }
            }
        }

        delegate_noop!(State: ignore WlShm);
        delegate_noop!(State: ignore WlCompositor);
        delegate_noop!(State: ignore WlSurface);
        delegate_noop!(State: ignore ZwpInputMethodManagerV2);
        delegate_noop!(State: ignore ZxdgImporterV2);
        delegate_noop!(State: ignore WlSeat);
        delegate_noop!(State: ignore WlOutput);
        delegate_noop!(State: ignore WlShmPool);
        delegate_noop!(State: ignore WlBuffer);
        delegate_noop!(State: ignore ZwlrScreencopyManagerV1);
        delegate_noop!(State: ignore ZwlrVirtualPointerManagerV1);
        delegate_noop!(State: ignore ZwlrVirtualPointerV1);
        delegate_noop!(State: ignore ZwpVirtualKeyboardManagerV1);
        delegate_noop!(State: ignore ZwlrDataControlManagerV1);
        delegate_noop!(State: ignore ZwpVirtualKeyboardV1);

        /// A picture of an output: red, green and blue of every pixel, top row first.
        pub struct Picture {
            pub width: u32,
            pub height: u32,
            pixels: Vec<(u8, u8, u8)>,
        }

        impl Picture {
            pub fn pixel(&self, x: u32, y: u32) -> Option<(u8, u8, u8)> {
                if x >= self.width || y >= self.height {
                    return None;
                }
                self.pixels.get((y * self.width + x) as usize).copied()
            }
        }

        pub struct Probe {
            connection: Connection,
            queue: EventQueue<State>,
            state: State,
            shm: WlShm,
            output: WlOutput,
            screencopy: Option<ZwlrScreencopyManagerV1>,
            pointer: Option<ZwlrVirtualPointerV1>,
            keyboard: Option<ZwpVirtualKeyboardV1>,
            data_control: Option<(ZwlrDataControlManagerV1, ZwlrDataControlDeviceV1)>,
            data_source: Option<ZwlrDataControlSourceV1>,
            seat: WlSeat,
            compositor: Option<WlCompositor>,
            importer: Option<ZxdgImporterV2>,
            imported: Option<ZxdgImportedV2>,
            input_method_manager: Option<ZwpInputMethodManagerV2>,
            input_method: Option<ZwpInputMethodV2>,
            /// The popup of the input method: its surface, its role, its buffer and the memory
            /// of the buffer.
            input_popup: Option<(WlSurface, ZwpInputPopupSurfaceV2, WlBuffer, OwnedFd)>,
            time: u32,
        }

        impl Probe {
            /// Connects to the compositor of the environment and binds what it has of the three
            /// protocols.
            pub fn connect() -> Result<Self, String> {
                let connection = Connection::connect_to_env().map_err(|error| error.to_string())?;
                let mut queue: EventQueue<State> = connection.new_event_queue();
                let handle = queue.handle();
                let registry = connection.display().get_registry(&handle, ());
                let mut state = State::default();
                queue.roundtrip(&mut state).map_err(|error| error.to_string())?;

                let find = |interface: &str| state.globals.iter().find(|(_, name, _)| name == interface).map(|(name, _, _)| *name);
                let shm: WlShm = registry.bind(find("wl_shm").ok_or("the compositor has no wl_shm")?, 1, &handle, ());
                let seat: WlSeat = registry.bind(find("wl_seat").ok_or("the compositor has no wl_seat")?, 1, &handle, ());
                let output: WlOutput = registry.bind(find("wl_output").ok_or("the compositor has no wl_output")?, 1, &handle, ());
                let screencopy: Option<ZwlrScreencopyManagerV1> =
                    find("zwlr_screencopy_manager_v1").map(|name| registry.bind(name, 1, &handle, ()));
                let pointer = find("zwlr_virtual_pointer_manager_v1").map(|name| {
                    let manager: ZwlrVirtualPointerManagerV1 = registry.bind(name, 1, &handle, ());
                    manager.create_virtual_pointer(Some(&seat), &handle, ())
                });
                let keyboard = find("zwp_virtual_keyboard_manager_v1").map(|name| {
                    let manager: ZwpVirtualKeyboardManagerV1 = registry.bind(name, 1, &handle, ());
                    manager.create_virtual_keyboard(&seat, &handle, ())
                });
                let data_control = find("zwlr_data_control_manager_v1").map(|name| {
                    let manager: ZwlrDataControlManagerV1 = registry.bind(name, 1, &handle, ());
                    let device = manager.get_data_device(&seat, &handle, ());
                    (manager, device)
                });
                let compositor: Option<WlCompositor> = find("wl_compositor").map(|name| registry.bind(name, 4, &handle, ()));
                let input_method_manager: Option<ZwpInputMethodManagerV2> =
                    find("zwp_input_method_manager_v2").map(|name| registry.bind(name, 1, &handle, ()));
                let importer: Option<ZxdgImporterV2> = find("zxdg_importer_v2").map(|name| registry.bind(name, 1, &handle, ()));
                queue.roundtrip(&mut state).map_err(|error| error.to_string())?;

                Ok(Self {
                    connection,
                    queue,
                    state,
                    shm,
                    output,
                    screencopy,
                    pointer,
                    keyboard,
                    data_control,
                    data_source: None,
                    seat,
                    compositor,
                    importer,
                    imported: None,
                    input_method_manager,
                    input_method: None,
                    input_popup: None,
                    time: 1000,
                })
            }

            /// Whether the compositor has the importer of `xdg-foreign`.
            pub fn has_importer(&self) -> bool {
                self.importer.is_some()
            }

            /// Imports a top-level another client exported, by its handle. A handle the
            /// compositor does not know is answered with `destroyed`.
            pub fn import_top_level(&mut self, handle: &str) -> Result<(), String> {
                let importer = self.importer.as_ref().ok_or("the compositor has no zxdg_importer_v2")?;
                self.state.imported_destroyed = false;
                if let Some(previous) = self.imported.replace(importer.import_toplevel(handle.to_string(), &self.queue.handle(), ())) {
                    previous.destroy();
                }
                self.sync()
            }

            /// Whether the compositor said that the imported top-level is gone.
            pub fn imported_destroyed(&mut self) -> bool {
                let _ = self.sync();
                self.state.imported_destroyed
            }

            /// Whether the compositor lets a client be the input method of a seat.
            pub fn has_input_method(&self) -> bool {
                self.input_method_manager.is_some()
            }

            /// Makes this client the input method of the seat.
            pub fn input_method_start(&mut self) -> Result<(), String> {
                let manager = self.input_method_manager.as_ref().ok_or("the compositor has no zwp_input_method_manager_v2")?;
                if self.input_method.is_none() {
                    self.input_method = Some(manager.get_input_method(&self.seat, &self.queue.handle(), ()));
                }
                self.sync()?;
                if self.state.input_method.unavailable {
                    return Err("the seat has an input method already".to_string());
                }
                Ok(())
            }

            /// What the input method was told, as of the last `done` of the compositor.
            pub fn input_method(&mut self) -> InputMethodState {
                let _ = self.sync();
                self.state.input_method.clone()
            }

            /// One batch of the input method: characters to delete around the cursor (in
            /// bytes), a text to commit, a preedit with its cursor (in bytes); then the commit
            /// with the serial, which is the number of `done` events so far.
            pub fn input_method_send(
                &mut self,
                delete: Option<(u32, u32)>,
                commit: Option<&str>,
                preedit: Option<(&str, i32, i32)>,
            ) -> Result<(), String> {
                self.sync()?;
                let input_method = self.input_method.as_ref().ok_or("no input method")?;
                if let Some((before, after)) = delete {
                    input_method.delete_surrounding_text(before, after);
                }
                if let Some(text) = commit {
                    input_method.commit_string(text.to_string());
                }
                if let Some((text, begin, end)) = preedit {
                    input_method.set_preedit_string(text.to_string(), begin, end);
                }
                input_method.commit(self.state.input_method.done_count);
                self.sync()
            }

            /// Maps a popup of the input method: a surface of one colour, which the compositor
            /// places at the cursor rectangle of the text input.
            pub fn input_method_popup(&mut self, size: (i32, i32), colour: (u8, u8, u8)) -> Result<(), String> {
                let handle = self.queue.handle();
                let compositor = self.compositor.as_ref().ok_or("the compositor has no wl_compositor of version 4")?;
                let input_method = self.input_method.as_ref().ok_or("no input method")?;
                let stride = size.0 * 4;
                let length = (stride * size.1) as usize;
                let fd = memfd_of_length(c"ferroui-wayland-input-popup", length).map_err(|error| error.to_string())?;
                {
                    let mut mapping = MemoryMapping::shared_read_write(fd.as_fd(), length).map_err(|error| error.to_string())?;
                    let data = mapping.as_mut_slice().ok_or("the buffer of the popup cannot be written")?;
                    for pixel in data.chunks_exact_mut(4) {
                        pixel.copy_from_slice(&[colour.2, colour.1, colour.0, 0xff]);
                    }
                }
                let pool = self.shm.create_pool(fd.as_fd(), length as i32, &handle, ());
                let buffer = pool.create_buffer(0, size.0, size.1, stride, wl_shm::Format::Argb8888, &handle, ());
                pool.destroy();
                let surface = compositor.create_surface(&handle, ());
                let popup = input_method.get_input_popup_surface(&surface, &handle, ());
                surface.attach(Some(&buffer), 0, 0);
                surface.damage_buffer(0, 0, size.0, size.1);
                surface.commit();
                self.input_popup = Some((surface, popup, buffer, fd));
                self.sync()
            }

            /// Gives up the popup and the input method.
            pub fn input_method_stop(&mut self) {
                if let Some((surface, popup, buffer, _)) = self.input_popup.take() {
                    popup.destroy();
                    surface.destroy();
                    buffer.destroy();
                }
                if let Some(input_method) = self.input_method.take() {
                    input_method.destroy();
                }
                let _ = self.sync();
                self.state.input_method = InputMethodState::default();
                self.state.pending_input_method = InputMethodState::default();
            }

            /// Whether the compositor lets this client see and set the selection (the data
            /// control protocol of wlroots, which `wl-copy` and `wl-paste` use as well).
            pub fn has_data_control(&self) -> bool {
                self.data_control.is_some()
            }

            /// How many times the selection changed, as this client saw it.
            pub fn selections(&mut self) -> u32 {
                let _ = self.sync();
                self.state.selections
            }

            /// The MIME types of the selection, `None` without a selection.
            pub fn selection_mime_types(&mut self) -> Option<Vec<String>> {
                let _ = self.sync();
                let selection = self.state.selection.as_ref()?;
                self.state.offers.get(selection).map(|(_, mime_types)| mime_types.clone())
            }

            /// Asks the owner of the selection for its data of a MIME type. The data arrives on
            /// the channel once the owner wrote all of it; the owner may be this process, so
            /// the pipe is read on a thread of its own.
            pub fn read_selection(&mut self, mime_type: &str) -> Result<Receiver<Vec<u8>>, String> {
                self.sync()?;
                let selection = self.state.selection.as_ref().ok_or("there is no selection")?;
                let (offer, _) = self.state.offers.get(selection).ok_or("the selection has no offer")?;
                let (read, write) = pipe2_cloexec().map_err(|error| error.to_string())?;
                offer.receive(mime_type.to_string(), write.as_fd());
                drop(write);
                self.connection.flush().map_err(|error| error.to_string())?;
                let (sender, receiver) = channel();
                std::thread::spawn(move || {
                    if let Ok(bytes) = Pipe2Stream::new(read, PipeDirection::In).read_to_end() {
                        let _ = sender.send(bytes);
                    }
                });
                Ok(receiver)
            }

            /// Makes this client the owner of the selection, with the data given by MIME type.
            pub fn set_selection(&mut self, data: Vec<(&str, Vec<u8>)>) -> Result<(), String> {
                let handle = self.queue.handle();
                let (manager, device) = self.data_control.as_ref().ok_or("no data control")?;
                let source = manager.create_data_source(&handle, ());
                for (mime_type, _) in &data {
                    source.offer(mime_type.to_string());
                }
                self.state.outgoing = data.into_iter().map(|(mime_type, bytes)| (mime_type.to_string(), Arc::new(bytes))).collect();
                self.state.sent.clear();
                self.state.source_cancelled = false;
                device.set_selection(Some(&source));
                if let Some(previous) = self.data_source.replace(source) {
                    previous.destroy();
                }
                self.sync()
            }

            /// The MIME types the source of this client was asked for since it was set.
            pub fn sent(&mut self) -> Vec<String> {
                let _ = self.sync();
                self.state.sent.clone()
            }

            /// Whether the source of this client was replaced by another selection.
            pub fn source_cancelled(&mut self) -> bool {
                let _ = self.sync();
                self.state.source_cancelled
            }

            pub fn has_screencopy(&self) -> bool {
                self.screencopy.is_some()
            }

            pub fn has_pointer(&self) -> bool {
                self.pointer.is_some()
            }

            pub fn has_keyboard(&self) -> bool {
                self.keyboard.is_some()
            }

            fn tick(&mut self) -> u32 {
                self.time += 10;
                self.time
            }

            /// Sends what was requested and waits until the compositor handled it.
            pub fn sync(&mut self) -> Result<(), String> {
                self.queue.roundtrip(&mut self.state).map(|_| ()).map_err(|error| error.to_string())
            }

            /// Reads the output as the compositor composed it.
            pub fn capture(&mut self) -> Result<Picture, String> {
                let screencopy = self.screencopy.clone().ok_or("the compositor has no screen copy manager")?;
                let handle = self.queue.handle();
                self.state.buffer = None;
                self.state.ready = false;
                self.state.failed = false;
                self.state.y_invert = false;

                let frame = screencopy.capture_output(0, &self.output, &handle, ());
                while self.state.buffer.is_none() && !self.state.failed {
                    self.queue.blocking_dispatch(&mut self.state).map_err(|error| error.to_string())?;
                }
                let info = self.state.buffer.ok_or("the compositor refused the screen copy")?;
                let length = info.stride as usize * info.height as usize;
                let fd = memfd_of_length(c"ferroui-wayland-screencopy", length).map_err(|error| error.to_string())?;
                let pool = self.shm.create_pool(fd.as_fd(), length as i32, &handle, ());
                let buffer =
                    pool.create_buffer(0, info.width as i32, info.height as i32, info.stride as i32, info.format, &handle, ());
                pool.destroy();
                frame.copy(&buffer);
                while !self.state.ready && !self.state.failed {
                    self.queue.blocking_dispatch(&mut self.state).map_err(|error| error.to_string())?;
                }
                buffer.destroy();
                frame.destroy();
                if self.state.failed {
                    return Err("the screen copy failed".to_string());
                }

                // The byte order of the two families of formats a compositor answers with.
                let blue_first = match info.format {
                    wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => true,
                    wl_shm::Format::Abgr8888 | wl_shm::Format::Xbgr8888 => false,
                    other => return Err(format!("the screen copy has the format {other:?}, which this example does not read")),
                };
                let mapping = MemoryMapping::private_read(fd.as_fd(), length).map_err(|error| error.to_string())?;
                let data = mapping.as_slice();
                let mut pixels = Vec::with_capacity((info.width * info.height) as usize);
                for row in 0..info.height {
                    let source_row = if self.state.y_invert { info.height - 1 - row } else { row };
                    let start = (source_row * info.stride) as usize;
                    for column in 0..info.width as usize {
                        let pixel = &data[start + column * 4..start + column * 4 + 4];
                        pixels.push(if blue_first { (pixel[2], pixel[1], pixel[0]) } else { (pixel[0], pixel[1], pixel[2]) });
                    }
                }
                Ok(Picture { width: info.width, height: info.height, pixels })
            }

            /// Moves the pointer to a point of an output of `extent` logical units.
            pub fn pointer_move(&mut self, x: u32, y: u32, extent: (u32, u32)) -> Result<(), String> {
                let time = self.tick();
                let pointer = self.pointer.as_ref().ok_or("no virtual pointer")?;
                pointer.motion_absolute(time, x, y, extent.0, extent.1);
                pointer.frame();
                self.sync()
            }

            /// Presses or releases the left button.
            pub fn pointer_button(&mut self, pressed: bool) -> Result<(), String> {
                const BTN_LEFT: u32 = 0x110;
                let time = self.tick();
                let pointer = self.pointer.as_ref().ok_or("no virtual pointer")?;
                let state = if pressed { wl_pointer::ButtonState::Pressed } else { wl_pointer::ButtonState::Released };
                pointer.button(time, BTN_LEFT, state);
                pointer.frame();
                self.sync()
            }

            /// Turns the wheel by `steps` notches: positive is down.
            pub fn pointer_wheel(&mut self, steps: i32) -> Result<(), String> {
                let time = self.tick();
                let pointer = self.pointer.as_ref().ok_or("no virtual pointer")?;
                pointer.axis_source(wl_pointer::AxisSource::Wheel);
                pointer.axis_discrete(time, wl_pointer::Axis::VerticalScroll, f64::from(steps) * 15.0, steps);
                pointer.frame();
                self.sync()
            }

            /// Gives the virtual keyboard its keymap: the text of a keymap.
            pub fn keyboard_keymap(&mut self, keymap: &str) -> Result<(), String> {
                let keyboard = self.keyboard.as_ref().ok_or("no virtual keyboard")?;
                // The size includes the terminator.
                let length = keymap.len() + 1;
                let fd = memfd_of_length(c"ferroui-wayland-keymap", length).map_err(|error| error.to_string())?;
                {
                    let mut mapping = MemoryMapping::shared_read_write(fd.as_fd(), length).map_err(|error| error.to_string())?;
                    let data = mapping.as_mut_slice().ok_or("the keymap cannot be written")?;
                    data[..keymap.len()].copy_from_slice(keymap.as_bytes());
                    data[keymap.len()] = 0;
                }
                keyboard.keymap(wl_keyboard::KeymapFormat::XkbV1 as u32, fd.as_fd(), length as u32);
                keyboard.modifiers(0, 0, 0, 0);
                self.sync()
            }

            /// Presses or releases a key by its evdev code.
            pub fn keyboard_key(&mut self, key: u32, pressed: bool) -> Result<(), String> {
                let time = self.tick();
                let keyboard = self.keyboard.as_ref().ok_or("no virtual keyboard")?;
                keyboard.key(time, key, u32::from(pressed));
                self.sync()
            }

            /// Whether the connection of the probe still works: the compositor did not end it.
            pub fn alive(&mut self) -> bool {
                self.sync().is_ok() && self.connection.protocol_error().is_none()
            }
        }
    }

    /// The text of the default keymap of the system (`xkb_keymap_new_from_names` without
    /// names), which the virtual keyboard gives the compositor.
    fn default_keymap_text() -> Result<String, String> {
        use xkbcommon_dl::{xkb_context_flags, xkb_keymap_compile_flags, xkb_keymap_format, xkbcommon_option};

        let xkb = xkbcommon_option().ok_or("libxkbcommon is not available")?;
        // SAFETY: the calls of the library with the objects it returned, each released once;
        // the text is copied before it is freed with the allocator of the C library, which
        // made it.
        unsafe {
            let context = (xkb.xkb_context_new)(xkb_context_flags::XKB_CONTEXT_NO_FLAGS);
            if context.is_null() {
                return Err("xkb_context_new failed".to_string());
            }
            let keymap =
                (xkb.xkb_keymap_new_from_names)(context, std::ptr::null(), xkb_keymap_compile_flags::XKB_KEYMAP_COMPILE_NO_FLAGS);
            if keymap.is_null() {
                (xkb.xkb_context_unref)(context);
                return Err("the default keymap cannot be compiled (is the keyboard data of the system installed?)".to_string());
            }
            let text = (xkb.xkb_keymap_get_as_string)(keymap, xkb_keymap_format::XKB_KEYMAP_FORMAT_TEXT_V1);
            let result = if text.is_null() {
                Err("xkb_keymap_get_as_string failed".to_string())
            } else {
                let copy = std::ffi::CStr::from_ptr(text).to_string_lossy().into_owned();
                libc_free(text.cast_mut().cast());
                Ok(copy)
            };
            (xkb.xkb_keymap_unref)(keymap);
            (xkb.xkb_context_unref)(context);
            result
        }
    }

    extern "C" {
        #[link_name = "free"]
        fn libc_free(pointer: *mut std::ffi::c_void);
    }

    mod smoke {
        use super::probe::{Picture, Probe};
        use super::*;
        use ferroui_base::input::raw::{
            IRawInputEventArgs, RawKeyEventArgs, RawKeyEventType, RawMouseWheelEventArgs, RawPointerEventArgs,
            RawPointerEventType, RawTextInputEventArgs,
        };
        use ferroui_base::input::{Key, PhysicalKey, StandardCursorType};
        use ferroui_base::platform::ICursorFactory;
        use ferroui_base::rendering::IRenderLoop;
        use ferroui_base::{PixelPoint, PixelSize, Point, Size, Vector};
        use ferroui_controls::platform::{IScreenImpl, IWindowingPlatform};
        use ferroui_base::input::WindowDecorationsElementRole;
        use ferroui_controls::platform::PlatformRequestedDrawnDecoration;
        use ferroui_controls::primitives::Popup;
        use ferroui_controls::{PlacementMode, WindowDecorations, WindowState};
        use wayland_client::Proxy;
        use ferroui_wayland::screens::SnapshotScreensImpl;
        use ferroui_wayland::server::wayland_worker_client::WaylandWorkerClient;
        use std::future::Future;
        use std::sync::Arc;
        use std::task::{Poll, Waker};

        /// How often the checks of the window are tried before they count as failed: the first
        /// frame takes a moment to arrive.
        const ATTEMPTS: u32 = 20;
        const INTERVAL: Duration = Duration::from_millis(500);
        /// How long something the compositor has to do may take before its check fails.
        const STEP_TIMEOUT: Duration = Duration::from_secs(5);
        /// How long the window may take to close after it was asked to.
        const CLOSE_TIMEOUT: Duration = Duration::from_secs(10);
        /// After this the run has hung and the process is ended.
        const RUN_TIMEOUT: Duration = Duration::from_secs(240);
        /// The mode the output is given in the resize phase.
        const NEW_MODE: (u32, u32) = (1024, 600);

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

        /// What `--skip=` and `--expect-scale=` say.
        struct Options {
            skip: Vec<String>,
            expect_scale: Option<f64>,
        }

        impl Options {
            fn from_args() -> Self {
                let mut options = Options { skip: Vec::new(), expect_scale: None };
                for arg in std::env::args() {
                    if let Some(skip) = arg.strip_prefix("--skip=") {
                        options.skip.extend(skip.split(',').map(str::to_string));
                    }
                    if let Some(scale) = arg.strip_prefix("--expect-scale=") {
                        options.expect_scale = scale.parse().ok();
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
            )
            .detach();
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

        /// Waits until `condition` holds, looking every 50 milliseconds; `false` when it did not
        /// within `timeout`.
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

        pub fn start(window: &Ref<Window>) {
            println!("Smoke mode: the window is checked through the compositor and then closed");
            // A run that hangs ends here, with a line that says so.
            std::thread::spawn(|| {
                std::thread::sleep(RUN_TIMEOUT);
                println!("  [FAILED] run: the smoke mode did not finish within {} seconds", RUN_TIMEOUT.as_secs());
                println!("SMOKE FAILED");
                std::process::exit(1);
            });

            let window = window.clone();
            let _task = Dispatcher::ui_thread().invoke_async_task_local(move || run(window));
        }

        async fn run(window: Ref<Window>) {
            let options = Options::from_args();
            let report = Report { failed: Cell::new(0), passed: Cell::new(0) };

            report.phase("platform");
            let client = platform_checks(&report);

            let mut probe = match Probe::connect() {
                Ok(probe) => Some(probe),
                Err(error) => {
                    report.check("test client", false, format!("the second connection cannot be made: {error}"));
                    None
                }
            };

            if options.runs("screens") {
                report.phase("screens");
                screens_checks(&report);
            }

            if options.runs("foreign") {
                report.phase("foreign");
                foreign_display_checks(&report).await;
            }

            report.phase("window");
            let mut attempt = 0;
            loop {
                delay(INTERVAL).await;
                attempt += 1;
                let checks = window_checks(&window, &options);
                if checks.iter().all(|check| check.1) || attempt >= ATTEMPTS {
                    println!("  attempt {attempt} of {ATTEMPTS}");
                    for (name, passed, detail) in checks {
                        report.check(name, passed, detail);
                    }
                    break;
                }
            }

            if let Some(probe) = &mut probe {
                if options.runs("frames") {
                    report.phase("frames");
                    frames_checks(&report, probe, &window).await;
                }
                if options.runs("input") {
                    report.phase("input");
                    input_checks(&report, probe, &window).await;
                }
                if options.runs("cursor") {
                    report.phase("cursor");
                    cursor_checks(&report, probe, &window, client.as_ref()).await;
                }
                if options.runs("popup") {
                    report.phase("popup");
                    popup_checks(&report, probe, &window, client.as_ref()).await;
                }
                if options.runs("clipboard") {
                    report.phase("clipboard");
                    clipboard_checks(&report, probe, client.as_ref(), &options).await;
                }
                if options.runs("dragdrop") {
                    report.phase("dragdrop");
                    drag_drop_checks(&report, probe, &window, client.as_ref()).await;
                }
                if options.runs("textinput") {
                    report.phase("textinput");
                    text_input_checks(&report, probe, &window, client.as_ref()).await;
                }
                if options.runs("export") {
                    report.phase("export");
                    export_checks(&report, probe, &window, client.as_ref()).await;
                }
                if options.runs("resize") {
                    report.phase("resize");
                    resize_checks(&report, probe, &window).await;
                }
                if options.runs("state") {
                    report.phase("state");
                    state_checks(&report, &window).await;
                }
                if options.runs("title") {
                    report.phase("title");
                    title_checks(&report, probe, &window).await;
                }
            }

            // Last: the window is drawn with its own decorations from here on, and floats.
            if let Some(probe) = &mut probe {
                if options.runs("decorations") {
                    report.phase("decorations");
                    decorations_checks(&report, probe, &window, client.as_ref()).await;
                }
            }

            println!("{} checks passed, {} failed", report.passed.get(), report.failed.get());
            SMOKE_PASSED.store(report.failed.get() == 0 && report.passed.get() > 0, Ordering::SeqCst);
            request_close(&window);
        }

        /// The services of the platform, and what the worker renders with.
        fn platform_checks(report: &Report) -> Option<Rc<WaylandWorkerClient>> {
            let locator = FerroLocator::current();
            let client = locator.get_service::<WaylandWorkerClient>();
            report.check(
                "platform",
                client.is_some() && locator.get_service::<dyn IWindowingPlatform>().is_some(),
                "the Wayland worker client and a windowing platform are registered".to_string(),
            );
            report.check(
                "services",
                locator.get_service::<dyn IScreenImpl>().is_some() && locator.get_service::<dyn ICursorFactory>().is_some(),
                "the screens and the cursor factory are registered".to_string(),
            );
            let render_loop = locator.get_service::<Arc<dyn IRenderLoop>>();
            report.check(
                "render loop",
                render_loop.as_ref().is_some_and(|render_loop| render_loop.runs_in_background()),
                "the render loop is the one of the worker thread".to_string(),
            );

            let mode = mode();
            println!("Rendering mode asked for: {mode}");
            if let Some(client) = &client {
                let uses_contexts =
                    client.invoke_oob(|worker| worker.state.worker.platform_graphics().uses_contexts()).recv_timeout(STEP_TIMEOUT);
                match (mode.as_str(), uses_contexts) {
                    ("dmabuf", Ok(_)) => dmabuf_checks(report, client),
                    ("egl", Ok(uses_contexts)) => report.check(
                        "platform graphics",
                        uses_contexts,
                        format!("the worker has {} display of EGL", if uses_contexts { "a" } else { "no" }),
                    ),
                    (_, Ok(uses_contexts)) => report.check(
                        "software rendering",
                        !uses_contexts,
                        format!("the worker has {} display of EGL", if uses_contexts { "a" } else { "no" }),
                    ),
                    (_, Err(_)) => report.check("worker", false, "the worker did not answer".to_string()),
                }
            }
            client
        }

        /// What the worker renders with when the dmabuf swapchain is asked for.
        fn dmabuf_checks(report: &Report, client: &WaylandWorkerClient) {
            use ferroui_wayland::server::transient::rendering::wayland_egl_dma_buf_platform_graphics::WaylandEglDmaBufPlatformGraphics;
            use ferroui_wayland::server::transient::rendering::wayland_egl_wsi_platform_graphics::WaylandEglWsiPlatformGraphics;
            let answer = client
                .invoke_oob(|worker| {
                    let has_global = worker.state.globals.as_ref().is_some_and(|globals| globals.linux_dmabuf.is_some());
                    let graphics = worker.state.worker.platform_graphics().current_graphics();
                    let backend = match &graphics {
                        Some(graphics) if graphics.as_any().is::<WaylandEglDmaBufPlatformGraphics>() => "dmabuf",
                        Some(graphics) if graphics.as_any().is::<WaylandEglWsiPlatformGraphics>() => "wl_egl_window",
                        Some(_) => "another",
                        None => "none (software)",
                    };
                    let formats = graphics
                        .as_ref()
                        .and_then(|graphics| graphics.as_any().downcast_ref::<WaylandEglDmaBufPlatformGraphics>())
                        .map(|graphics| graphics.display().supported_formats().len());
                    (has_global, backend, formats)
                })
                .recv_timeout(STEP_TIMEOUT);
            let Ok((has_global, backend, formats)) = answer else {
                report.check("worker", false, "the worker did not answer".to_string());
                return;
            };
            println!(
                "Dmabuf swapchain: the compositor {} zwp_linux_dmabuf_v1 (version 4 or later); the GPU backend is {backend}{}",
                if has_global { "has" } else { "has no" },
                formats.map(|formats| format!(", with {formats} format and modifier pairs")).unwrap_or_default()
            );
            if has_global {
                report.check(
                    "dmabuf swapchain",
                    backend == "dmabuf",
                    format!("the compositor has zwp_linux_dmabuf_v1 and the worker renders through {backend}"),
                );
            } else {
                println!(
                    "  [not measured] dmabuf swapchain: the compositor has no zwp_linux_dmabuf_v1 (no render node: sway on the \
                     software renderer of wlroots); the backend fell back to a wl_egl_window, as the reference does"
                );
                report.check(
                    "platform graphics",
                    backend == "wl_egl_window",
                    format!("without zwp_linux_dmabuf_v1 the worker renders through {backend}"),
                );
            }
        }

        /// A second worker on a display another library opened (`Start` with a foreign
        /// display): it binds its globals on a queue of its own, the display stays the other
        /// library's, and both keep working.
        async fn foreign_display_checks(report: &Report) {
            use ferroui_wayland::server::interop::wayland_connection::WaylandConnection;
            use ferroui_wayland::server::wayland_worker::WaylandWorker;
            use wayland_client::{Connection, Proxy};

            // The other library: wayland-client, which opens the display and owns it.
            let display = match Connection::connect_to_env() {
                Ok(display) => display,
                Err(error) => {
                    report.check("foreign display", false, format!("the display cannot be opened: {error}"));
                    return;
                }
            };
            let pointer = display.backend().display_ptr();
            // SAFETY: the display of the connection above, which is never closed (it is
            // forgotten at the end of the phase and lives as long as the process).
            let foreign = unsafe { WaylandConnection::from_foreign_display_ptr(pointer.cast()) };
            let worker = match WaylandWorker::new() {
                Ok(worker) => worker,
                Err(error) => {
                    report.check("foreign display", false, format!("the worker cannot be made: {error}"));
                    return;
                }
            };
            let mut options = WaylandPlatformOptions::new();
            options.gl_profiles = Vec::new();
            if let Err(error) = worker.start_with_foreign_display(options.for_worker(), foreign, None) {
                report.check("foreign display", false, format!("the worker cannot be started: {error}"));
                return;
            }

            let mut answer = None;
            for _ in 0..ATTEMPTS {
                delay(INTERVAL).await;
                let state = worker
                    .invoke_oob(|worker| {
                        let connection = worker.connection.as_ref()?;
                        let globals = worker.state.globals.as_ref()?;
                        Some((
                            connection.owns_display(),
                            connection.display_ptr() as usize,
                            globals.wl_compositor.version(),
                            worker.state.worker.platform_graphics().is_ready(),
                        ))
                    })
                    .recv_timeout(STEP_TIMEOUT);
                if let Ok(Some(state)) = state {
                    answer = Some(state);
                    break;
                }
            }
            let Some((owns_display, worker_display, compositor_version, graphics_ready)) = answer else {
                report.check("foreign display", false, "the worker bound no globals on the foreign display".to_string());
                std::mem::forget(display);
                return;
            };
            report.check(
                "foreign display",
                !owns_display && worker_display == pointer as usize,
                format!(
                    "the worker runs on the wl_display of the other library ({}) and does not own it",
                    if worker_display == pointer as usize { "the same pointer" } else { "another pointer" }
                ),
            );
            report.check(
                "foreign globals",
                compositor_version >= 4 && graphics_ready,
                format!("wl_compositor version {compositor_version}, platform graphics ready: {graphics_ready}"),
            );
            // The other library's own queue still works beside the worker's.
            let roundtrip = display.roundtrip();
            report.check(
                "foreign owner",
                roundtrip.is_ok(),
                format!("a round trip of the library that opened the display: {roundtrip:?}"),
            );
            // The display stays open for the worker, which runs until the process ends.
            std::mem::forget(display);
        }

        /// The screens of the platform against the outputs the compositor reports.
        fn screens_checks(report: &Report) {
            let Some(outputs) = sway::outputs() else {
                report.check("outputs", false, "the compositor did not answer get_outputs (is SWAYSOCK set?)".to_string());
                return;
            };
            let outputs: Vec<(String, (i32, i32, i32, i32))> = outputs
                .as_array()
                .iter()
                .filter(|output| output.get("active").and_then(json::Json::as_bool).unwrap_or(true))
                .filter_map(|output| {
                    Some((output.get("name")?.as_str()?.to_string(), sway::rect(output, "rect")?))
                })
                .collect();

            let Some(screens) = FerroLocator::current().get_service::<SnapshotScreensImpl>() else {
                report.check("screens", false, "the screens of the platform are not registered".to_string());
                return;
            };
            let screens = IScreenImpl::all_screens(&*screens);
            report.check(
                "count",
                screens.len() == outputs.len() && !screens.is_empty(),
                format!("{} screens, the compositor has {} outputs", screens.len(), outputs.len()),
            );
            for (name, (x, y, width, height)) in &outputs {
                let screen = screens.iter().find(|screen| screen.display_name().as_deref() == Some(name.as_str()));
                let matches = screen.is_some_and(|screen| {
                    let bounds = screen.bounds();
                    (bounds.x, bounds.y, bounds.width, bounds.height) == (*x, *y, *width, *height) && screen.scaling() == 1.0
                });
                report.check(
                    "screen",
                    matches,
                    format!(
                        "output {name} at ({x}, {y}) of {width} by {height} logical units; the screen of that name: {}",
                        screen.map_or("none".to_string(), |screen| format!("{:?}", screen.bounds()))
                    ),
                );
            }
        }

        /// The logical size of the first output of the compositor, and its scale.
        fn output_geometry() -> Option<((i32, i32), f64, String)> {
            let outputs = sway::outputs()?;
            let output = outputs.as_array().first()?;
            let (_, _, width, height) = sway::rect(output, "rect")?;
            let scale = output.get("scale").and_then(json::Json::as_f64).unwrap_or(1.0);
            Some(((width, height), scale, output.get("name")?.as_str()?.to_string()))
        }

        /// What the compositor knows of the window, and what the framework took from it.
        fn window_checks(window: &Ref<Window>, options: &Options) -> Vec<(&'static str, bool, String)> {
            let mut checks = Vec::new();
            let Some(view) = sway::view(TITLE) else {
                checks.push(("view", false, format!("the compositor has no view with the title \"{TITLE}\"")));
                return checks;
            };
            checks.push(("view", true, format!("the compositor has a view with the title \"{TITLE}\"")));

            let app_id = view.get("app_id").and_then(json::Json::as_str).map(str::to_string);
            checks.push(("application identifier", app_id.as_deref() == Some(APP_ID), format!("{app_id:?}, expected \"{APP_ID}\"")));

            let focused = view.get("focused").and_then(json::Json::as_bool) == Some(true);
            checks.push((
                "focus",
                focused && window.is_active(),
                format!("the compositor says focused: {focused}; the window is active: {}", window.is_active()),
            ));

            let Some(((output_width, output_height), output_scale, _)) = output_geometry() else {
                checks.push(("output", false, "the compositor did not report its output".to_string()));
                return checks;
            };
            let rect = sway::rect(&view, "rect");
            let client_size = window.client_size();
            let whole_output = rect == Some((0, 0, output_width, output_height));
            checks.push((
                "geometry",
                whole_output,
                format!("the view is at {rect:?}; the output has {output_width} by {output_height} logical units"),
            ));
            checks.push((
                "client size",
                client_size == Size::new(f64::from(output_width), f64::from(output_height)),
                format!("the framework has {client_size:?}"),
            ));

            let scaling = window.render_scaling();
            let expected = options.expect_scale.unwrap_or(output_scale);
            checks.push((
                "scale",
                (scaling - expected).abs() < 1e-6 && (output_scale - expected).abs() < 1e-6,
                format!("the window renders at {scaling}; the output has {output_scale}; expected {expected}"),
            ));
            checks
        }

        fn close_to(pixel: Option<(u8, u8, u8)>, colour: (u8, u8, u8)) -> bool {
            pixel.is_some_and(|(r, g, b)| {
                [(r, colour.0), (g, colour.1), (b, colour.2)].iter().all(|(a, b)| (i32::from(*a) - i32::from(*b)).abs() <= 3)
            })
        }

        /// The picture of the output once the pixel at a point has a colour, or the last one.
        async fn picture_with(probe: &mut Probe, point: (u32, u32), colour: (u8, u8, u8)) -> Result<Picture, String> {
            let started = std::time::Instant::now();
            loop {
                let picture = probe.capture()?;
                if close_to(picture.pixel(point.0, point.1), colour) || started.elapsed() >= STEP_TIMEOUT {
                    return Ok(picture);
                }
                delay(Duration::from_millis(200)).await;
            }
        }

        /// The exports of the first top-level of the worker, and whether the connection has the
        /// exporter of `xdg-foreign`.
        fn exports(client: &Rc<WaylandWorkerClient>) -> Option<(bool, usize)> {
            client
                .invoke_oob(|worker| {
                    let globals = worker.state.globals.as_ref()?;
                    let exports = worker.state.top_levels.values().map(|top_level| top_level.active_export_count()).sum();
                    Some((globals.xdg_exporter.is_some(), exports))
                })
                .recv_timeout(STEP_TIMEOUT)
                .ok()
                .flatten()
        }

        /// The parent handle of a portal dialog: the window exported through `xdg-foreign`, and
        /// imported by the test client.
        async fn export_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>, client: Option<&Rc<WaylandWorkerClient>>) {
            use ferroui_wayland::window_impl::WindowImpl;
            let Some(client) = client else {
                report.check("export", false, "the platform has no worker client".to_string());
                return;
            };
            let before = exports(client);
            report.check(
                "exporter",
                before == Some((true, 0)),
                format!("the worker has zxdg_exporter_v2 and no export yet: {before:?}"),
            );
            let Some(window_impl) = window.platform_impl() else {
                report.check("window", false, "the window has no platform implementation".to_string());
                return;
            };
            let Some(lease) = window_impl.as_any().downcast_ref::<WindowImpl>().map(WindowImpl::acquire_portal_parent_lease) else {
                report.check("window", false, "the platform implementation is not a window of the backend".to_string());
                return;
            };
            let lease = pumped(probe, lease).await.flatten();
            let handle = lease.as_ref().map(|lease| lease.handle());
            let exported = exports(client);
            report.check(
                "parent handle",
                handle.as_deref().is_some_and(|handle| handle.strip_prefix("wayland:").is_some_and(|raw| !raw.is_empty()))
                    && exported == Some((true, 1)),
                format!("the lease of the window has the handle {handle:?}; the worker has {exported:?} (exporter, exports)"),
            );
            let Some(lease) = lease else {
                return;
            };
            if probe.has_importer() {
                // The compositor knows the handle: another client imports the window by it, and
                // is told when the export ends.
                let raw = handle.as_deref().and_then(|handle| handle.strip_prefix("wayland:")).unwrap_or_default().to_string();
                let imported = probe.import_top_level(&raw);
                delay(Duration::from_millis(200)).await;
                let destroyed = probe.imported_destroyed();
                report.check(
                    "import",
                    imported.is_ok() && !destroyed,
                    format!("the test client imported the window by the handle: {imported:?}; told that it is gone: {destroyed}"),
                );
                let unknown = probe.import_top_level("no-such-handle");
                let refused = wait_for(STEP_TIMEOUT, || probe.imported_destroyed()).await;
                report.check("unknown handle", unknown.is_ok() && refused, format!("a handle nobody exported is refused: {refused}"));
                let _ = probe.import_top_level(&raw);
                delay(Duration::from_millis(200)).await;
                let known_again = !probe.imported_destroyed();
                pumped(probe, lease.dispose_async()).await;
                let ended = wait_for(STEP_TIMEOUT, || probe.imported_destroyed()).await;
                let after = exports(client);
                report.check(
                    "lease disposed",
                    known_again && ended && after == Some((true, 0)),
                    format!("with the lease the export ends: the importer is told ({ended}); the worker has {after:?}"),
                );
            } else {
                pumped(probe, lease.dispose_async()).await;
                let after = wait_for(STEP_TIMEOUT, || exports(client) == Some((true, 0))).await;
                report.check("lease disposed", after, format!("with the lease the export ends: {:?}", exports(client)));
            }
            let alive = probe.alive() && exports(client).is_some();
            report.check("connections", alive, format!("the test client and the worker are still connected: {alive}"));
        }

        /// The colour of the popup of the input method of the test client.
        const INPUT_POPUP_FILL: (u8, u8, u8) = (0xff, 0x00, 0xff);
        const INPUT_POPUP_SIZE: (i32, i32) = (48, 24);
        /// Where the text box of the phase is in the window.
        const TEXT_BOX_ORIGIN: (f64, f64) = (160.0, 140.0);

        /// The text input of the worker: the number of seats with a text input object, and
        /// the state of every surface that registered a sink.
        fn text_input_state(
            client: &Rc<WaylandWorkerClient>,
        ) -> Option<(usize, Vec<ferroui_wayland::server::transient::wayland_text_input_v3::TextInputSurfaceSnapshot>)> {
            client.invoke_oob(|worker| worker.state.text_input_snapshot()).recv_timeout(STEP_TIMEOUT).ok().flatten()
        }

        /// Waits until the input method of the test client has heard nothing new for a moment:
        /// the window sends its state in several commits, each answered with a `done`, and a
        /// commit of the input method has to name the last.
        async fn input_method_quiet(probe: &mut Probe) {
            let mut serial = probe.input_method().done_count;
            for _ in 0..20 {
                delay(Duration::from_millis(150)).await;
                let now = probe.input_method().done_count;
                if now == serial {
                    return;
                }
                serial = now;
            }
        }

        /// The input method of a top-level over `zwp_text_input_v3`, with the test client as
        /// the input method of the seat.
        async fn text_input_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>, client: Option<&Rc<WaylandWorkerClient>>) {
            use ferroui_controls::presenters::TextPresenter;
            use ferroui_controls::TextBox;

            let Some(client) = client else {
                report.check("text input", false, "the platform has no worker client".to_string());
                return;
            };
            let seats = text_input_state(client).map(|(seats, _)| seats);
            report.check(
                "text input manager",
                seats.is_some_and(|seats| seats > 0),
                format!("the worker has zwp_text_input_manager_v3 and a text input object for {seats:?} seats"),
            );
            if !probe.has_input_method() {
                report.check("input method", false, "the compositor has no zwp_input_method_manager_v2".to_string());
                return;
            }
            let started = probe.input_method_start();
            report.check("input method", started.is_ok(), format!("the test client is the input method of the seat: {started:?}"));
            if started.is_err() || seats.is_none() {
                return;
            }

            // A page with a text box in place of the content of the window, for this phase.
            let text_box = TextBox::new();
            text_box.set_width(320.0);
            text_box.set_horizontal_alignment(HorizontalAlignment::Left);
            text_box.set_vertical_alignment(VerticalAlignment::Top);
            text_box.set_margin(Thickness::new(TEXT_BOX_ORIGIN.0, TEXT_BOX_ORIGIN.1, 0.0, 0.0));
            let page = Border::new();
            page.set_background(Some(brush(FILL)));
            page.set_child(&text_box);
            let previous_content = window.content();
            window.set_content(Some(Control::boxed(page)));
            delay(Duration::from_millis(300)).await;
            let preedit_of = |text_box: &Ref<TextBox>| -> Option<String> {
                text_box
                    .clone()
                    .upcast::<ferroui_base::Visual>()
                    .get_visual_descendants()
                    .find_map(|visual| visual.cast::<TextPresenter>())
                    .and_then(|presenter| presenter.preedit_text())
            };
            let text_of = |text_box: &Ref<TextBox>| text_box.text().unwrap_or_default();

            // 1. The focus of a text box enables the text input, and the compositor activates
            //    the input method with the state of the client.
            let focused = text_box.focus();
            let enabled = wait_for(STEP_TIMEOUT, || {
                probe.input_method().active
                    && text_input_state(client).is_some_and(|(_, surfaces)| {
                        surfaces.iter().any(|surface| surface.has_client && surface.last_sent_enabled && !surface.entered_seats.is_empty())
                    })
            })
            .await;
            input_method_quiet(probe).await;
            let worker = text_input_state(client).map(|(_, surfaces)| surfaces).unwrap_or_default();
            report.check(
                "enable",
                focused && enabled,
                format!("a text box took the focus ({focused}) and the text input of its surface is enabled: {worker:?}"),
            );
            let state = probe.input_method();
            report.check(
                "activate",
                state.active && state.activations >= 1 && state.content_type.is_some_and(|(_, purpose)| purpose == 0),
                format!(
                    "the input method was activated {} time(s) with the content type {:?} (hint, purpose; normal is 0) and the surrounding text {:?}",
                    state.activations, state.content_type, state.surrounding
                ),
            );
            report.check(
                "surrounding text",
                state.surrounding.as_ref().is_some_and(|(text, cursor, anchor)| text.is_empty() && *cursor == 0 && *anchor == 0),
                format!("the empty text box is announced as {:?}", state.surrounding),
            );

            // 2. A preedit of the input method is the preedit of the text box, and not its text.
            let sent = probe.input_method_send(None, None, Some(("ni", 2, 2)));
            let arrived = wait_for(STEP_TIMEOUT, || {
                let _ = probe.sync();
                preedit_of(&text_box).as_deref() == Some("ni")
            })
            .await;
            report.check(
                "preedit",
                sent.is_ok() && arrived && text_of(&text_box).is_empty(),
                format!("the preedit \"ni\": {sent:?}; the text box has the preedit {:?} and the text {:?}", preedit_of(&text_box), text_of(&text_box)),
            );

            // 3. A commit replaces the preedit with text; the input method hears of the new
            //    text around the cursor, in bytes.
            input_method_quiet(probe).await;
            let committed = "\u{4f60}\u{597d}";
            let sent = probe.input_method_send(None, Some(committed), None);
            let arrived = wait_for(STEP_TIMEOUT, || {
                let _ = probe.sync();
                text_of(&text_box) == committed && preedit_of(&text_box).is_none()
            })
            .await;
            report.check(
                "commit",
                sent.is_ok() && arrived,
                format!("two characters committed: {sent:?}; the text box has the text {:?} and the preedit {:?}", text_of(&text_box), preedit_of(&text_box)),
            );
            let told = wait_for(STEP_TIMEOUT, || {
                probe.input_method().surrounding.is_some_and(|(text, cursor, anchor)| text == committed && cursor == 6 && anchor == 6)
            })
            .await;
            report.check(
                "surrounding text after the commit",
                told,
                format!("the input method is told {:?} (the cursor after six bytes)", probe.input_method().surrounding),
            );

            // 4. One batch with a commit and a preedit: both, in the order of the protocol.
            input_method_quiet(probe).await;
            let sent = probe.input_method_send(None, Some("!"), Some(("x", 1, 1)));
            let expected = format!("{committed}!");
            let arrived = wait_for(STEP_TIMEOUT, || {
                let _ = probe.sync();
                text_of(&text_box) == expected && preedit_of(&text_box).as_deref() == Some("x")
            })
            .await;
            report.check(
                "commit and preedit",
                sent.is_ok() && arrived,
                format!("a batch with both: {sent:?}; the text {:?}, the preedit {:?}", text_of(&text_box), preedit_of(&text_box)),
            );

            // 5. Deleting around the cursor: four bytes before it are two characters. The
            //    batch has no preedit, which clears the one of the batch before.
            let told = wait_for(STEP_TIMEOUT, || probe.input_method().surrounding.is_some_and(|(text, _, _)| text == expected)).await;
            input_method_quiet(probe).await;
            let sent = probe.input_method_send(Some((4, 0)), None, None);
            let arrived = wait_for(STEP_TIMEOUT, || {
                let _ = probe.sync();
                text_of(&text_box) == "\u{4f60}" && preedit_of(&text_box).is_none()
            })
            .await;
            report.check(
                "delete surrounding text",
                told && sent.is_ok() && arrived,
                format!("four bytes before the cursor deleted: {sent:?}; the text {:?}, the preedit {:?}", text_of(&text_box), preedit_of(&text_box)),
            );

            // 6. The cursor rectangle: the compositor places a popup of the input method below
            //    it and tells the popup the rectangle, relative to the popup.
            input_method_quiet(probe).await;
            let rectangle = text_input_state(client)
                .and_then(|(_, surfaces)| surfaces.into_iter().find(|surface| surface.has_client))
                .and_then(|surface| surface.cursor_rectangle);
            let mapped = probe.input_method_popup(INPUT_POPUP_SIZE, INPUT_POPUP_FILL);
            let reported = wait_for(STEP_TIMEOUT, || !probe.input_method().rectangles.is_empty()).await;
            let rectangles = probe.input_method().rectangles;
            let last = rectangles.last().copied();
            report.check(
                "cursor rectangle sent",
                rectangle.is_some_and(|(x, y, w, h)| {
                    f64::from(x) > TEXT_BOX_ORIGIN.0 && f64::from(y) > TEXT_BOX_ORIGIN.1 && w >= 1 && h > 1
                }),
                format!("the worker sent the cursor rectangle {rectangle:?} (x, y, width, height of the surface): inside the text box at {TEXT_BOX_ORIGIN:?}"),
            );
            if mapped.is_ok() && !reported {
                // sway has the popups of an input method from 1.10; 1.9 (measured: its binary has
                // no handler for one) neither places the surface nor tells it a rectangle, and
                // the input method protocol has no other way to see the rectangle.
                println!(
                    "  [not measured] cursor rectangle at the compositor: the compositor gave the popup of the input method no rectangle (it has no input popups)"
                );
            } else {
                report.check(
                    "cursor rectangle",
                    mapped.is_ok() && matches!((rectangle, last), (Some((_, _, w, h)), Some((_, _, rw, rh))) if (w, h) == (rw, rh)),
                    format!(
                        "the worker sent the cursor rectangle {rectangle:?}; the popup of the input method ({mapped:?}) is told {last:?}, relative to itself"
                    ),
                );
            }
            if let (Some((x, y, _, h)), true, true) = (rectangle, probe.has_screencopy(), reported) {
                let scaling = window.render_scaling();
                // A point inside the popup when its top left corner is the bottom left corner of
                // the cursor rectangle, which is where sway puts it.
                let inside = (f64::from(x) + f64::from(INPUT_POPUP_SIZE.0) / 2.0, f64::from(y + h) + f64::from(INPUT_POPUP_SIZE.1) / 2.0);
                let point = ((inside.0 * scaling) as u32, (inside.1 * scaling) as u32);
                let picture = picture_with(probe, point, INPUT_POPUP_FILL).await;
                let pixel = picture.as_ref().ok().and_then(|picture| picture.pixel(point.0, point.1));
                report.check(
                    "input popup",
                    close_to(pixel, INPUT_POPUP_FILL),
                    format!(
                        "the pixel at {point:?}, below the cursor rectangle, is {pixel:?}; the popup of the input method is {INPUT_POPUP_FILL:?}"
                    ),
                );
            }

            // 7. The keyboard still types: the input method has no grab, and a key arrives as
            //    a key and as its text.
            if probe.has_keyboard() {
                const KEY_A: u32 = 30;
                let keymap = default_keymap_text().and_then(|keymap| probe.keyboard_keymap(&keymap));
                delay(Duration::from_millis(300)).await;
                let sent = probe.keyboard_key(KEY_A, true).and_then(|()| probe.keyboard_key(KEY_A, false));
                let typed = wait_for(STEP_TIMEOUT, || {
                    let _ = probe.sync();
                    text_of(&text_box) == "\u{4f60}a"
                })
                .await;
                report.check(
                    "key while enabled",
                    keymap.is_ok() && sent.is_ok() && typed,
                    format!("the key of \"a\" with the text input enabled: {sent:?}; the text {:?}", text_of(&text_box)),
                );
            }

            // 8. The text box goes (the content of the window is put back): the text input is
            //    disabled and the input method deactivated.
            window.set_content(previous_content);
            let disabled = wait_for(STEP_TIMEOUT, || {
                !probe.input_method().active
                    && text_input_state(client).is_some_and(|(_, surfaces)| surfaces.iter().all(|surface| !surface.has_client))
            })
            .await;
            let state = probe.input_method();
            report.check(
                "disable",
                disabled && state.deactivations == state.activations,
                format!(
                    "without the text box the text input is disabled: the input method was activated {} and deactivated {} time(s); the worker has {:?}",
                    state.activations, state.deactivations,
                    text_input_state(client).map(|(_, surfaces)| surfaces)
                ),
            );

            probe.input_method_stop();
            let alive = probe.alive();
            let worker_alive = text_input_state(client).is_some();
            report.check(
                "connections",
                alive && worker_alive,
                format!("the test client ({alive}) and the worker ({worker_alive}) are still connected"),
            );
            // The marker is back for the phases that follow.
            delay(Duration::from_millis(300)).await;
        }

        /// The frames of the window as the compositor composed them.
        async fn frames_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>) {
            if !probe.has_screencopy() {
                report.check("screen copy", false, "the compositor has no zwlr_screencopy_manager_v1".to_string());
                return;
            }
            let scaling = window.render_scaling();
            let client_size = window.client_size();
            let pixel_size =
                PixelSize::new((client_size.width * scaling).round() as i32, (client_size.height * scaling).round() as i32);
            let at = |x: f64, y: f64| ((x * scaling) as u32, (y * scaling) as u32);

            let marker_centre = at(MARKER_ORIGIN.0 + MARKER_SIZE / 2.0, MARKER_ORIGIN.1 + MARKER_SIZE / 2.0);
            let picture = match picture_with(probe, marker_centre, MARKER_FILL).await {
                Ok(picture) => picture,
                Err(error) => {
                    report.check("screen copy", false, error);
                    return;
                }
            };
            report.check(
                "size",
                (picture.width as i32, picture.height as i32) == (pixel_size.width, pixel_size.height),
                format!("the output has {} by {} pixels; the window {pixel_size:?}", picture.width, picture.height),
            );
            for (name, (x, y)) in [
                ("top left", at(5.0, 5.0)),
                ("centre", at(client_size.width / 2.0, client_size.height / 2.0)),
                ("bottom right", at(client_size.width - 5.0, client_size.height - 5.0)),
            ] {
                let pixel = picture.pixel(x, y);
                report.check(name, close_to(pixel, FILL), format!("the pixel at ({x}, {y}) is {pixel:?}, the fill is {FILL:?}"));
            }
            let pixel = picture.pixel(marker_centre.0, marker_centre.1);
            report.check(
                "marker",
                close_to(pixel, MARKER_FILL),
                format!("the pixel at {marker_centre:?} is {pixel:?}, the marker is {MARKER_FILL:?}"),
            );
            // Just outside the marker, towards the origin: the fill. A frame at another scale,
            // or upside down, has the marker elsewhere.
            let outside = at(MARKER_ORIGIN.0 - 4.0, MARKER_ORIGIN.1 - 4.0);
            let pixel = picture.pixel(outside.0, outside.1);
            report.check("beside the marker", close_to(pixel, FILL), format!("the pixel at {outside:?} is {pixel:?}"));
        }

        #[derive(Clone, Debug, PartialEq)]
        enum Recorded {
            Pointer(RawPointerEventType, Point),
            Wheel(Vector),
            Key(RawKeyEventType, Key, PhysicalKey, Option<String>),
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
                return Some(Recorded::Key(key.type_(), key.key(), key.physical_key(), key.key_symbol()));
            }
            if let Some(text) = args.downcast_ref::<RawTextInputEventArgs>() {
                return Some(Recorded::Text(text.text().to_string()));
            }
            None
        }

        /// Input from the compositor's side: every step is synthesized through the virtual
        /// pointer and the virtual keyboard and expected at the input callback of the window.
        async fn input_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>) {
            let Some(window_impl) = window.platform_impl() else {
                report.check("window", false, "the window has no platform implementation".to_string());
                return;
            };
            let client_size = window.client_size();
            let extent = (client_size.width as u32, client_size.height as u32);

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
            let has = |expected: &dyn Fn(&Recorded) -> bool| log.borrow().iter().any(expected);
            let tail = || {
                let log = log.borrow();
                format!("{:?}", &log[log.len().saturating_sub(4)..])
            };

            if probe.has_pointer() {
                for (x, y) in [(200u32, 150u32), (320, 240)] {
                    log.borrow_mut().clear();
                    let sent = probe.pointer_move(x, y, extent);
                    let expected = Point::new(f64::from(x), f64::from(y));
                    let arrived = wait_for(STEP_TIMEOUT, || {
                        has(&|event| {
                            matches!(event, Recorded::Pointer(RawPointerEventType::Move, position)
                                if (position.x - expected.x).abs() <= 1.0 && (position.y - expected.y).abs() <= 1.0)
                        })
                    })
                    .await;
                    report.check("pointer move", sent.is_ok() && arrived, format!("to {expected:?}: {sent:?}; last input {}", tail()));
                }

                log.borrow_mut().clear();
                let sent = probe.pointer_button(true).and_then(|()| probe.pointer_button(false));
                let pressed = wait_for(STEP_TIMEOUT, || {
                    has(&|event| matches!(event, Recorded::Pointer(RawPointerEventType::LeftButtonUp, _)))
                })
                .await;
                let down_there = has(&|event| {
                    matches!(event, Recorded::Pointer(RawPointerEventType::LeftButtonDown, position)
                        if (position.x - 320.0).abs() <= 1.0 && (position.y - 240.0).abs() <= 1.0)
                });
                report.check(
                    "pointer button",
                    sent.is_ok() && pressed && down_there,
                    format!("left button down and up at (320, 240): {sent:?}; last input {}", tail()),
                );

                for (steps, expected) in [(1, -1.0), (-1, 1.0)] {
                    log.borrow_mut().clear();
                    let sent = probe.pointer_wheel(steps);
                    let arrived = wait_for(STEP_TIMEOUT, || {
                        has(&|event| matches!(event, Recorded::Wheel(delta) if delta.x == 0.0 && delta.y == expected))
                    })
                    .await;
                    report.check(
                        "wheel",
                        sent.is_ok() && arrived,
                        format!("{steps} notch (positive is down), expected a delta of (0, {expected}): {sent:?}; last input {}", tail()),
                    );
                }
            } else {
                report.check("virtual pointer", false, "the compositor has no zwlr_virtual_pointer_manager_v1".to_string());
            }

            if probe.has_keyboard() {
                const KEY_A: u32 = 30;
                const KEY_B: u32 = 48;
                match default_keymap_text().and_then(|keymap| probe.keyboard_keymap(&keymap)) {
                    Ok(()) => {
                        // The compositor tells the seat about the new keymap before the key.
                        delay(Duration::from_millis(300)).await;
                        log.borrow_mut().clear();
                        let sent = probe.keyboard_key(KEY_A, true).and_then(|()| probe.keyboard_key(KEY_A, false));
                        let released =
                            wait_for(STEP_TIMEOUT, || has(&|event| matches!(event, Recorded::Key(RawKeyEventType::KeyUp, Key::A, _, _))))
                                .await;
                        let down = has(&|event| {
                            matches!(event, Recorded::Key(RawKeyEventType::KeyDown, Key::A, PhysicalKey::A, symbol)
                                if symbol.as_deref() == Some("a"))
                        });
                        report.check(
                            "key",
                            sent.is_ok() && released && down,
                            format!("the key of \"a\" down with its physical key and symbol, and up: {sent:?}; last input {}", tail()),
                        );
                        let text = wait_for(STEP_TIMEOUT, || has(&|event| *event == Recorded::Text("a".to_string()))).await;
                        report.check("text", text, format!("expected the text \"a\"; last input {}", tail()));

                        // A key that is held repeats, and the client does the repeating.
                        log.borrow_mut().clear();
                        let sent = probe.keyboard_key(KEY_B, true);
                        delay(Duration::from_millis(1500)).await;
                        let downs = log
                            .borrow()
                            .iter()
                            .filter(|event| matches!(event, Recorded::Key(RawKeyEventType::KeyDown, Key::B, _, _)))
                            .count();
                        let released = probe.keyboard_key(KEY_B, false);
                        delay(Duration::from_millis(300)).await;
                        let after_release = log
                            .borrow()
                            .iter()
                            .filter(|event| matches!(event, Recorded::Key(RawKeyEventType::KeyDown, Key::B, _, _)))
                            .count();
                        delay(Duration::from_millis(500)).await;
                        let later = log
                            .borrow()
                            .iter()
                            .filter(|event| matches!(event, Recorded::Key(RawKeyEventType::KeyDown, Key::B, _, _)))
                            .count();
                        report.check(
                            "key repeat",
                            sent.is_ok() && released.is_ok() && downs >= 3 && later == after_release,
                            format!("{downs} key-down events while \"b\" was held for a second and a half, {later} in the end"),
                        );
                    }
                    Err(error) => report.check("keymap", false, error),
                }
            } else {
                report.check("virtual keyboard", false, "the compositor has no zwp_virtual_keyboard_manager_v1".to_string());
            }

            window_impl.set_input(framework_input);
        }

        /// The window geometry the worker last gave the compositor for the first top-level.
        fn window_geometry(client: &Rc<WaylandWorkerClient>) -> Option<(i32, i32, i32, i32)> {
            client
                .invoke_oob(|worker| worker.state.top_levels.values().next().and_then(|top_level| top_level.shell().last_window_geometry()))
                .recv_timeout(STEP_TIMEOUT)
                .ok()
                .flatten()
        }

        /// The rectangle of the view of the window in the compositor.
        fn view_rect() -> Option<(i32, i32, i32, i32)> {
            sway::view(TITLE).and_then(|view| sway::rect(&view, "rect"))
        }

        /// A press at a point of the output, a drag by an offset in steps, and a release. The
        /// dispatcher runs between the steps: the press has to reach the window, and the
        /// request of the window the compositor, while the button is still down.
        async fn drag(probe: &mut Probe, from: (i32, i32), by: (i32, i32), extent: (u32, u32)) -> Result<(), String> {
            let clamp = |value: i32, limit: u32| value.clamp(0, limit as i32 - 1) as u32;
            probe.pointer_move(clamp(from.0, extent.0), clamp(from.1, extent.1), extent)?;
            delay(Duration::from_millis(200)).await;
            probe.pointer_button(true)?;
            delay(Duration::from_millis(400)).await;
            for step in 1..=8 {
                let x = from.0 + by.0 * step / 8;
                let y = from.1 + by.1 * step / 8;
                probe.pointer_move(clamp(x, extent.0), clamp(y, extent.1), extent)?;
                delay(Duration::from_millis(50)).await;
            }
            delay(Duration::from_millis(200)).await;
            probe.pointer_button(false)
        }

        /// The data sources the worker has and whether its first data device owns the selection.
        fn data_sources(client: &Rc<WaylandWorkerClient>) -> Option<(usize, bool)> {
            client
                .invoke_oob(|worker| {
                    let globals = worker.state.globals.as_ref()?;
                    let owner = globals.input_dispatcher_has_selection_source();
                    Some((globals.data_sources.len(), owner))
                })
                .recv_timeout(STEP_TIMEOUT)
                .ok()
                .flatten()
        }

        /// Runs a future of the UI thread to its end while the test client answers the
        /// compositor: its events are dispatched only when it is asked to.
        async fn pumped<T: 'static>(probe: &mut Probe, future: impl Future<Output = T> + 'static) -> Option<T> {
            let task = Dispatcher::ui_thread().invoke_async_task_local(move || future);
            let done = wait_for(STEP_TIMEOUT, || {
                let _ = probe.sync();
                task.is_completed()
            })
            .await;
            if done {
                task.result().ok()
            } else {
                None
            }
        }

        /// The data of the selection as the test client reads it.
        async fn read_by_probe(probe: &mut Probe, mime_type: &str) -> Result<Vec<u8>, String> {
            let receiver = probe.read_selection(mime_type)?;
            let mut bytes = None;
            wait_for(STEP_TIMEOUT, || {
                let _ = probe.sync();
                bytes = receiver.try_recv().ok();
                bytes.is_some()
            })
            .await;
            bytes.ok_or_else(|| format!("no data of the type {mime_type} within {} seconds", STEP_TIMEOUT.as_secs()))
        }

        /// Runs a command and gives what it wrote, without holding up the UI thread, which
        /// serves the selection the command may ask for.
        async fn command_output(program: &str, arguments: &[&str], input: Option<Vec<u8>>) -> Result<Vec<u8>, String> {
            use std::io::Write;
            use std::process::{Command, Stdio};
            let mut command = Command::new(program);
            command.args(arguments).stdout(Stdio::piped()).stderr(Stdio::piped());
            command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
            let mut child = command.spawn().map_err(|error| format!("{program} cannot be started: {error}"))?;
            if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
                let _ = stdin.write_all(&input);
            }
            let (sender, receiver) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = sender.send(child.wait_with_output());
            });
            let mut output = None;
            wait_for(STEP_TIMEOUT, || {
                output = receiver.try_recv().ok();
                output.is_some()
            })
            .await;
            match output {
                Some(Ok(output)) if output.status.success() => Ok(output.stdout),
                Some(Ok(output)) => Err(format!("{program} failed: {}", String::from_utf8_lossy(&output.stderr).trim())),
                Some(Err(error)) => Err(format!("{program}: {error}")),
                None => Err(format!("{program} did not end within {} seconds", STEP_TIMEOUT.as_secs())),
            }
        }

        /// The clipboard: the selection of the data device, in both directions, against the
        /// test client (which sees and sets the selection through the data control protocol
        /// of the compositor) and against `wl-paste` and `wl-copy`.
        async fn clipboard_checks(report: &Report, probe: &mut Probe, client: Option<&Rc<WaylandWorkerClient>>, options: &Options) {
            use ferroui_base::input::platform::{ClipboardExtensions, IClipboard, IClipboardImpl};
            use ferroui_base::input::{DataFormat, DataTransfer, DataTransferItem};

            const UTF8: &str = "text/plain;charset=utf-8";
            const BYTES: &str = "application/x-ferroui-smoke-bytes";

            let (Some(clipboard), Some(clipboard_impl)) = (
                FerroLocator::current().get_service::<dyn IClipboard>(),
                FerroLocator::current().get_service::<dyn IClipboardImpl>(),
            ) else {
                report.check("clipboard", false, "the platform has no clipboard".to_string());
                return;
            };
            let Some(client) = client else {
                report.check("clipboard", false, "the platform has no worker client".to_string());
                return;
            };

            let device = client
                .invoke_oob(|worker| {
                    let globals = worker.state.globals.as_ref()?;
                    Some((globals.data_device_manager.is_some(), globals.input_dispatcher_has_data_device()))
                })
                .recv_timeout(STEP_TIMEOUT)
                .ok()
                .flatten();
            report.check(
                "data device",
                device == Some((true, true)),
                format!("the worker has the data device manager and a data device of the seat: {device:?}"),
            );
            if !probe.has_data_control() {
                report.check("test client", false, "the compositor has no data control manager for the test client".to_string());
                return;
            }

            let owner = |clipboard_impl: &Rc<dyn IClipboardImpl>| {
                let clipboard_impl = clipboard_impl.clone();
                async move {
                    match clipboard_impl.as_owned_clipboard_impl() {
                        Some(owned) => owned.is_current_owner_async().await.ok(),
                        None => None,
                    }
                }
            };

            // The application sets a text; the other client reads it.
            let text = "Text of the smoke mode: caf\u{e9} \u{4e16}\u{754c}";
            let before = probe.selections();
            let set = pumped(probe, clipboard.set_text_async(Some(text))).await;
            let announced = wait_for(STEP_TIMEOUT, || probe.selections() > before).await;
            let mime_types = probe.selection_mime_types().unwrap_or_default();
            report.check(
                "text set",
                matches!(set, Some(Ok(()))) && announced && mime_types == [UTF8, "text/plain"],
                format!("the selection the compositor announces to the other client has the types {mime_types:?}"),
            );
            let read = read_by_probe(probe, UTF8).await;
            report.check(
                "text read by the other client",
                read.as_deref() == Ok(text.as_bytes()),
                format!("{:?}", read.as_ref().map(|bytes| String::from_utf8_lossy(bytes).into_owned())),
            );
            let is_owner = pumped(probe, owner(&clipboard_impl)).await.flatten();
            report.check("owner", is_owner == Some(true), format!("the application owns the clipboard: {is_owner:?}"));

            // A text, bytes of a format of the application, and a format of the process.
            let bytes_format = DataFormat::create_bytes_application_format("smoke-bytes");
            let payload: Vec<u8> = (0..=255u8).collect();
            let transfer = DataTransfer::new();
            let item = DataTransferItem::new();
            item.set(&DataFormat::text(), Some("with bytes".to_string()));
            item.set(&bytes_format, Some(Rc::from(payload.as_slice())));
            item.set(&DataFormat::create_in_process_format::<String>("smoke-private"), Some("private".to_string()));
            transfer.add(item);
            let before = probe.selections();
            let set = pumped(probe, clipboard.set_data_async(Some(transfer))).await;
            let announced = wait_for(STEP_TIMEOUT, || probe.selections() > before).await;
            let mime_types = probe.selection_mime_types().unwrap_or_default();
            let read = read_by_probe(probe, BYTES).await;
            report.check(
                "application format",
                matches!(set, Some(Ok(()))) && announced && mime_types == [UTF8, "text/plain", BYTES] && read.as_deref() == Ok(payload.as_slice()),
                format!("types {mime_types:?} (the format of the process is not offered); {} bytes read", read.as_ref().map(Vec::len).unwrap_or(0)),
            );

            // More than a pipe holds: the writer waits for the reader.
            let large: String = (0..(1 << 20)).map(|index| char::from(b'a' + (index % 26) as u8)).collect();
            let before = probe.selections();
            let set = pumped(probe, clipboard.set_text_async(Some(&large))).await;
            let announced = wait_for(STEP_TIMEOUT, || probe.selections() > before).await;
            let read = read_by_probe(probe, "text/plain").await;
            report.check(
                "large text",
                matches!(set, Some(Ok(()))) && announced && read.as_deref() == Ok(large.as_bytes()),
                format!("{} of {} bytes read by the other client", read.as_ref().map(Vec::len).unwrap_or(0), large.len()),
            );

            if options.runs("wl-clipboard") {
                let set = pumped(probe, clipboard.set_text_async(Some(text))).await;
                let pasted = command_output("wl-paste", &["--no-newline"], None).await;
                report.check(
                    "wl-paste",
                    matches!(set, Some(Ok(()))) && pasted.as_deref() == Ok(text.as_bytes()),
                    format!("{:?}", pasted.as_ref().map(|bytes| String::from_utf8_lossy(bytes).into_owned())),
                );
            }

            // The other client sets a text; the application reads it and is not the owner.
            let other = "Text of the other client: gr\u{fc}\u{df}e";
            let sources_before = data_sources(client);
            let set = probe.set_selection(vec![(UTF8, other.as_bytes().to_vec())]);
            let lost = wait_for(STEP_TIMEOUT, || data_sources(client) == Some((0, false))).await;
            let got = pumped(probe, clipboard.try_get_text_async()).await;
            let is_owner = pumped(probe, owner(&clipboard_impl)).await.flatten();
            report.check(
                "text of the other client",
                set.is_ok() && matches!(&got, Some(Ok(Some(got))) if got == other) && is_owner == Some(false),
                format!("read {got:?}; owner: {is_owner:?}; asked for {:?}", probe.sent()),
            );
            report.check(
                "source cancelled",
                lost,
                format!("the worker had {sources_before:?} (sources, owner) and has {:?} after the other client took the selection", data_sources(client)),
            );

            // Bytes of the format of the application, more than a pipe holds.
            let large_bytes: Vec<u8> = (0..(1usize << 20)).map(|index| (index % 251) as u8).collect();
            let set = probe.set_selection(vec![(BYTES, large_bytes.clone())]);
            let got = pumped(probe, clipboard.try_get_value_async(&bytes_format)).await;
            let length = match &got {
                Some(Ok(Some(bytes))) => bytes.len(),
                _ => 0,
            };
            report.check(
                "large data of the other client",
                set.is_ok() && matches!(&got, Some(Ok(Some(bytes))) if **bytes == *large_bytes.as_slice()),
                format!("{length} of {} bytes read by the application", large_bytes.len()),
            );

            // A list of files.
            let file = std::env::current_exe().ok().map(|path| path.display().to_string()).unwrap_or_default();
            let set = probe.set_selection(vec![("text/uri-list", format!("file://{file}\r\n").into_bytes())]);
            let got = pumped(probe, clipboard.try_get_files_async()).await;
            let names: Vec<String> = match &got {
                Some(Ok(Some(files))) => files.iter().map(|file| file.name()).collect(),
                _ => Vec::new(),
            };
            let expected = std::path::Path::new(&file).file_name().map(|name| name.to_string_lossy().into_owned());
            report.check(
                "files of the other client",
                set.is_ok() && names.len() == 1 && names.first() == expected.as_ref(),
                format!("the list of one file gives {names:?}"),
            );

            if options.runs("wl-clipboard") {
                // `wl-copy` stays in the foreground as the owner until the selection is replaced.
                let copied = "Text of wl-copy";
                let before = probe.selections();
                let child = std::process::Command::new("wl-copy").args(["--foreground", copied]).spawn();
                let announced = wait_for(STEP_TIMEOUT, || probe.selections() > before).await;
                let got = pumped(probe, clipboard.try_get_text_async()).await;
                report.check(
                    "wl-copy",
                    child.is_ok() && announced && matches!(&got, Some(Ok(Some(got))) if got == copied),
                    format!("read {got:?}{}", child.as_ref().err().map(|error| format!("; wl-copy cannot be started: {error}")).unwrap_or_default()),
                );
                if let Ok(mut child) = child {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }

            // The application clears the clipboard: a selection without a type.
            let before = probe.selections();
            let cleared = pumped(probe, clipboard.clear_async()).await;
            let announced = wait_for(STEP_TIMEOUT, || probe.selections() > before).await;
            let mime_types = probe.selection_mime_types();
            let replaced = probe.source_cancelled();
            report.check(
                "clear",
                matches!(cleared, Some(Ok(()))) && announced && mime_types.clone().unwrap_or_default().is_empty() && replaced,
                format!("the selection after the clear has the types {mime_types:?}; the source of the other client was cancelled: {replaced}"),
            );
            report.check("connection", probe.alive(), "the compositor ended neither connection".to_string());
        }

        /// Drag and drop: a press on the marker starts a drag of a text, the compositor leads
        /// it over the window, which is the drop target as well, and the release drops it;
        /// then a drag the window refuses.
        async fn drag_drop_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>, client: Option<&Rc<WaylandWorkerClient>>) {
            use ferroui_base::input::DragDropEffects;

            if !probe.has_pointer() {
                report.check("drag", false, "the compositor has no virtual pointer for the test client".to_string());
                return;
            }
            let Some(client) = client else {
                report.check("drag", false, "the platform has no worker client".to_string());
                return;
            };
            let client_size = window.client_size();
            let extent = (client_size.width as u32, client_size.height as u32);
            let from = ((MARKER_ORIGIN.0 + MARKER_SIZE / 2.0) as i32, (MARKER_ORIGIN.1 + MARKER_SIZE / 2.0) as i32);
            let by = (200, 150);
            let target = Point::new(f64::from(from.0 + by.0), f64::from(from.1 + by.1));

            // The sources the worker has before the drags: the selection, when the application
            // owns the clipboard.
            let sources_before = data_sources(client);
            for refuse in [false, true] {
                DRAG.with(|drag| *drag.borrow_mut() = DragProbe { armed: true, refuse, ..DragProbe::default() });
                let dragged = drag(probe, from, by, extent).await;
                let ended = wait_for(STEP_TIMEOUT, || DRAG.with(|drag| drag.borrow().result.is_some())).await;
                let (events, result) = DRAG.with(|drag| {
                    let mut drag = drag.borrow_mut();
                    drag.armed = false;
                    (std::mem::take(&mut drag.events), drag.result)
                });
                let count = |kind: &str| events.iter().filter(|event| event.0 == kind).count();
                let summary = format!(
                    "{} enter, {} over, {} drop, {} leave; result {result:?}; {dragged:?}",
                    count("enter"),
                    count("over"),
                    count("drop"),
                    count("leave")
                );
                if refuse {
                    report.check(
                        "refused drag",
                        dragged.is_ok() && ended && result == Some(DragDropEffects::NONE) && count("enter") >= 1 && count("drop") == 0,
                        summary,
                    );
                } else {
                    let dropped = events.iter().find(|event| event.0 == "drop");
                    let at_target = dropped.is_some_and(|event| (event.1.x - target.x).abs() <= 2.0 && (event.1.y - target.y).abs() <= 2.0);
                    report.check(
                        "drag",
                        // The marker and the window around it are entered one after the other.
                        dragged.is_ok() && ended && count("enter") >= 1 && count("over") >= 1 && count("drop") == 1,
                        summary,
                    );
                    report.check(
                        "drop",
                        at_target && dropped.is_some_and(|event| event.2.as_deref() == Some(DRAG_TEXT)),
                        format!("the drop at {:?} (expected {target:?}) has the text {:?}", dropped.map(|event| event.1), dropped.and_then(|event| event.2.clone())),
                    );
                    report.check("drag result", result == Some(DragDropEffects::COPY), format!("the source is told {result:?}"));
                }
                let sources = wait_for(STEP_TIMEOUT, || sources_before.is_some() && data_sources(client) == sources_before).await;
                report.check(
                    if refuse { "source after the refused drag" } else { "source after the drop" },
                    sources,
                    format!("the worker has {:?} (sources, owner), as before the drag ({sources_before:?}): the source of the drag is gone", data_sources(client)),
                );
                // The pointer is given back to the window before the next drag.
                delay(Duration::from_millis(300)).await;
            }
            DRAG.with(|drag| *drag.borrow_mut() = DragProbe::default());
            report.check("connection", probe.alive(), "the compositor ended neither connection".to_string());
        }

        /// Decorations: what the compositor answered about them; then the application asks
        /// for less than full decorations once, after which the framework draws them for
        /// good (title bar, border, resize grips, shadow) and the window geometry leaves the
        /// shadow out; a press on the drawn title bar moves the floating window and a press on
        /// a drawn resize grip resizes it, both done by the compositor.
        async fn decorations_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>, client: Option<&Rc<WaylandWorkerClient>>) {
            let (Some(client), Some(window_impl)) = (client, window.platform_impl()) else {
                report.check("decorations", false, "the worker client or the platform window is missing".to_string());
                return;
            };
            let all = PlatformRequestedDrawnDecoration::TITLE_BAR
                | PlatformRequestedDrawnDecoration::BORDER
                | PlatformRequestedDrawnDecoration::RESIZE_GRIPS
                | PlatformRequestedDrawnDecoration::SHADOW;

            let has_manager = client
                .invoke_oob(|worker| worker.state.globals.as_ref().is_some_and(|globals| globals.xdg_decoration_manager.is_some()))
                .recv_timeout(STEP_TIMEOUT)
                .unwrap_or(false);
            let drawn_before = window_impl.requested_drawn_decorations();
            report.check(
                "decoration mode",
                if has_manager {
                    !window_impl.needs_managed_decorations() && drawn_before == PlatformRequestedDrawnDecoration::NONE
                } else {
                    window_impl.needs_managed_decorations() && drawn_before == all
                },
                format!(
                    "the compositor has {} decoration manager; the framework is asked to draw {drawn_before:?}",
                    if has_manager { "a" } else { "no" }
                ),
            );

            // Less than full decorations once: the decoration object is destroyed for good and
            // the framework draws from then on, also when full decorations are asked for again.
            window.set_window_decorations(WindowDecorations::BorderOnly);
            delay(Duration::from_millis(200)).await;
            window.set_window_decorations(WindowDecorations::Full);
            let scaling = window.render_scaling();
            let column = ((MARKER_ORIGIN.0 + MARKER_SIZE / 2.0) * scaling) as u32;
            // The first row of the marker in the composed output: it moves down by the title bar.
            let marker_row = |picture: &Picture| (0..picture.height).find(|y| close_to(picture.pixel(column, *y), MARKER_FILL));
            let started = std::time::Instant::now();
            let mut title_bar = None;
            let mut top_pixel = None;
            while started.elapsed() < STEP_TIMEOUT {
                if let Ok(picture) = probe.capture() {
                    let moved = marker_row(&picture).map(|row| f64::from(row) / scaling - MARKER_ORIGIN.1);
                    if moved.is_some_and(|moved| moved >= 8.0) {
                        title_bar = moved;
                        top_pixel = picture.pixel(picture.width / 2, 4);
                        break;
                    }
                }
                delay(Duration::from_millis(200)).await;
            }
            let geometry = window_geometry(client);
            let view = view_rect();
            let drawn = window_impl.requested_drawn_decorations();
            report.check(
                "drawn decorations",
                window_impl.needs_managed_decorations() && drawn == all,
                format!("after the application asked for less than full decorations the framework is asked to draw {drawn:?}"),
            );
            report.check(
                "title bar",
                title_bar.is_some_and(|height| (8.0..=80.0).contains(&height)) && !close_to(top_pixel, FILL),
                format!("the content moved down by {title_bar:?} in the composed output, and the top of the window is {top_pixel:?}"),
            );
            println!("  tiled: the window geometry is {geometry:?} (left, top, width, height), the view of the compositor {view:?}");
            let (Some(title_bar), Some(_)) = (title_bar, geometry) else {
                return;
            };

            // A floating window with its top left corner at a place that leaves room on the
            // output, which the compositor lets the pointer move and resize. Its window geometry
            // is the view of the compositor: the shadow lies outside it.
            let floated =
                sway::message(&["floating", "enable"]).is_some() && sway::message(&["move", "position", "40", "30"]).is_some();
            let settled = wait_for(STEP_TIMEOUT, || {
                matches!((view_rect(), window_geometry(client)), (Some(view), Some(geometry))
                    if (view.0, view.1) == (40, 30) && (geometry.2, geometry.3) == (view.2, view.3))
            })
            .await;
            delay(Duration::from_millis(300)).await;
            let (Some(view), Some(geometry), Some(((output_width, output_height), _, _))) =
                (view_rect(), window_geometry(client), output_geometry())
            else {
                report.check("floating", false, format!("the compositor did not float the window (asked: {floated})"));
                return;
            };
            report.check(
                "window geometry",
                floated && settled && geometry.0 > 0 && geometry.1 > 0 && (geometry.2, geometry.3) == (view.2, view.3),
                format!(
                    "floating, the window geometry sent with the frame is {geometry:?} (left, top, width, height): the shadow is outside it, and its size is the view of the compositor, {view:?}"
                ),
            );
            let extent = (output_width as u32, output_height as u32);
            // A point of the client area in the coordinates of the output.
            let to_output = |x: f64, y: f64| (view.0 + x as i32 - geometry.0, view.1 + y as i32 - geometry.1);
            let input_root = window.input_root();

            let title_point = (f64::from(geometry.0) + f64::from(geometry.2) / 2.0, f64::from(geometry.1) + title_bar / 2.0);
            let role = input_root.hit_test_chrome_element(Point::new(title_point.0, title_point.1));
            let from = to_output(title_point.0, title_point.1);
            let sent = drag(probe, from, (40, 20), extent).await;
            let moved = wait_for(STEP_TIMEOUT, || view_rect().is_some_and(|now| now.0 - view.0 >= 25 && now.1 - view.1 >= 10)).await;
            let after_move = view_rect();
            report.check(
                "interactive move",
                role == Some(WindowDecorationsElementRole::TitleBar) && sent.is_ok() && moved,
                format!(
                    "a drag of the drawn title bar (role {role:?}) from {from:?} by (40, 20): the view went from {view:?} to {after_move:?}: {sent:?}"
                ),
            );

            // The grip of the top left corner, which is on the output whatever size the
            // compositor gave the floating window: the first point along the diagonal through
            // the corner of the window geometry that the decorations name so.
            let (Some(view), Some(geometry)) = (view_rect(), window_geometry(client)) else {
                report.check("interactive resize", false, "the view or the window geometry is gone".to_string());
                return;
            };
            let to_output = |x: f64, y: f64| (view.0 + x as i32 - geometry.0, view.1 + y as i32 - geometry.1);
            let corner = (f64::from(geometry.0), f64::from(geometry.1));
            let grip = (-6..=12)
                .map(|d| (corner.0 - f64::from(d), corner.1 - f64::from(d)))
                .filter(|point| point.0 >= 0.0 && point.1 >= 0.0)
                .find(|point| {
                    input_root.hit_test_chrome_element(Point::new(point.0, point.1)) == Some(WindowDecorationsElementRole::ResizeNW)
                });
            let Some(grip) = grip else {
                report.check(
                    "interactive resize",
                    false,
                    format!("no point around the corner {corner:?} of the window geometry {geometry:?} is the top left resize grip"),
                );
                return;
            };
            let from = to_output(grip.0, grip.1);
            let size_before = window.client_size();
            let sent = drag(probe, from, (60, 40), extent).await;
            let resized = wait_for(STEP_TIMEOUT, || {
                view_rect().is_some_and(|now| view.2 - now.2 >= 30 && view.3 - now.3 >= 20)
                    && size_before.width - window.client_size().width >= 30.0
            })
            .await;
            let after_resize = view_rect();
            report.check(
                "interactive resize",
                sent.is_ok() && resized,
                format!(
                    "a drag of the drawn grip at {grip:?} of the client area ({from:?} of the output) by (60, 40): the view went from {view:?} to {after_resize:?}, the client size from {size_before:?} to {:?}: {sent:?}",
                    window.client_size()
                ),
            );
            // A popup of the decorated window: its anchor is given in the coordinates of the
            // buffer, and the compositor places it against the window geometry, which starts
            // after the shadow.
            if let (Some(marker), Some(geometry)) = (MARKER.with(|cell| cell.borrow().clone()), window_geometry(client)) {
                // The root visual is the whole buffer: the shadow, the border and the title bar
                // lie between it and the content of the window.
                let root_visual = window.presentation_source().root_visual();
                let origin = root_visual
                    .and_then(|root_visual| {
                        let root_visual: &ferroui_base::Visual = &root_visual;
                        marker.translate_point(Point::new(0.0, 0.0), root_visual)
                    })
                    .unwrap_or_default();
                let child = Border::new();
                child.set_background(Some(brush(POPUP_FILL)));
                child.set_width(POPUP_SIZE.0);
                child.set_height(POPUP_SIZE.1);
                let popup = Popup::new();
                popup.set_child(&child);
                popup.set_placement_target(&marker);
                popup.set_placement(PlacementMode::BottomEdgeAlignedLeft);
                popup.set_is_open(true);
                let expected = (
                    origin.x.round() as i32 - geometry.0,
                    (origin.y + MARKER_SIZE).round() as i32 - geometry.1,
                    POPUP_SIZE.0 as i32,
                    POPUP_SIZE.1 as i32,
                );
                // Where the output leaves no room below the marker the compositor flips the popup
                // to the other side of it.
                let flipped = (expected.0, origin.y.round() as i32 - geometry.1 - POPUP_SIZE.1 as i32, expected.2, expected.3);
                let placed =
                    wait_for(STEP_TIMEOUT, || [vec![expected], vec![flipped]].contains(&popup_configures(client))).await;
                let configured = popup_configures(client);
                report.check(
                    "popup of the decorated window",
                    placed,
                    format!(
                        "the marker is at {origin:?} of the buffer and the window geometry starts at ({}, {}): the compositor configured the popup at {configured:?}; expected {expected:?}, or {flipped:?} above the marker",
                        geometry.0, geometry.1
                    ),
                );
                popup.set_is_open(false);
                delay(Duration::from_millis(300)).await;
            }
            let alive = probe.alive() && sway::view(TITLE).is_some();
            report.check("after the decorations", alive, "the connection and the window are still there".to_string());
        }

        /// What the worker has of popups: how many are registered, how many have their role
        /// object, how many are mapped, how many have a popup as their parent, and the version
        /// of the shell.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        struct PopupCounts {
            registered: usize,
            attached: usize,
            mapped: usize,
            nested: usize,
            shell_version: u32,
        }

        fn popup_counts(client: &Rc<WaylandWorkerClient>) -> Option<PopupCounts> {
            client
                .invoke_oob(|worker| {
                    let state = &worker.state;
                    PopupCounts {
                        registered: state.popups.len(),
                        attached: state.popups.values().filter(|popup| popup.is_attached()).count(),
                        mapped: state.popups.values().filter(|popup| popup.shell().is_mapped()).count(),
                        nested: state.popups.values().filter(|popup| state.popups.contains_key(&popup.parent())).count(),
                        shell_version: state.globals.as_ref().map_or(0, |globals| globals.xdg_wm_base.version()),
                    }
                })
                .recv_timeout(STEP_TIMEOUT)
                .ok()
        }

        /// Where the compositor configured the popups of the worker: position (relative to the
        /// window geometry of the parent) and size, in the order the popups were made.
        fn popup_configures(client: &Rc<WaylandWorkerClient>) -> Vec<(i32, i32, i32, i32)> {
            client
                .invoke_oob(|worker| {
                    let mut popups: Vec<_> = worker.state.popups.iter().collect();
                    popups.sort_by_key(|(id, _)| **id);
                    popups
                        .into_iter()
                        .filter_map(|(_, popup)| popup.placement().configure.and_then(|configure| configure.geometry))
                        .map(|geometry| (geometry.x, geometry.y, geometry.width, geometry.height))
                        .collect::<Vec<_>>()
                })
                .recv_timeout(STEP_TIMEOUT)
                .unwrap_or_default()
        }

        /// A popup of the framework over `xdg_popup`: placed by the compositor from the
        /// positioner, moved, given a popup of its own, and dismissed by a press beside it.
        async fn popup_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>, client: Option<&Rc<WaylandWorkerClient>>) {
            let (Some(client), Some(marker)) = (client, MARKER.with(|cell| cell.borrow().clone())) else {
                report.check("popup", false, "the worker client or the marker of the window is missing".to_string());
                return;
            };
            if !probe.has_screencopy() || !probe.has_pointer() {
                report.check("popup", false, "the compositor has no screen copy or no virtual pointer".to_string());
                return;
            }
            let scaling = window.render_scaling();
            let client_size = window.client_size();
            let extent = (client_size.width as u32, client_size.height as u32);
            let at = |x: f64, y: f64| ((x * scaling) as u32, (y * scaling) as u32);
            // The pointer away from where the popups will be.
            let _ = probe.pointer_move(20, 20, extent);

            let nested_child = Border::new();
            nested_child.set_background(Some(brush(NESTED_FILL)));
            nested_child.set_width(NESTED_SIZE.0);
            nested_child.set_height(NESTED_SIZE.1);
            let nested = Popup::new();
            nested.set_child(&nested_child);
            nested.set_placement(PlacementMode::Right);

            let popup_child = Border::new();
            popup_child.set_background(Some(brush(POPUP_FILL)));
            popup_child.set_width(POPUP_SIZE.0);
            popup_child.set_height(POPUP_SIZE.1);
            nested.set_placement_target(&popup_child);
            let popup = Popup::new();
            popup.set_child(&popup_child);
            popup.set_placement_target(&marker);
            // The left edge of the popup at the left edge of the marker, below it.
            popup.set_placement(PlacementMode::BottomEdgeAlignedLeft);
            popup.set_is_light_dismiss_enabled(true);

            let before = popup_counts(client).unwrap_or_default();
            popup.set_is_open(true);

            // Where the compositor has to put it: under the marker.
            let left = MARKER_ORIGIN.0;
            let top = MARKER_ORIGIN.1 + MARKER_SIZE;
            let centre = at(left + POPUP_SIZE.0 / 2.0, top + POPUP_SIZE.1 / 2.0);
            let picture = picture_with(probe, centre, POPUP_FILL).await;
            let counts = popup_counts(client).unwrap_or_default();
            report.check(
                "popup created",
                !popup.is_using_overlay_layer() && before.registered == 0 && counts.registered == 1 && counts.attached == 1 && counts.mapped == 1,
                format!(
                    "the popup of the framework is a surface of the worker with its xdg_popup, mapped: {counts:?} (xdg_wm_base version {})",
                    counts.shell_version
                ),
            );
            // The place the compositor reported, relative to the window: under the marker, the
            // left edges aligned.
            let expected = (left as i32, top as i32, POPUP_SIZE.0 as i32, POPUP_SIZE.1 as i32);
            let configured = popup_configures(client);
            report.check(
                "popup configure",
                configured == vec![expected],
                format!("the compositor configured the popup at {configured:?} (x, y, width, height); expected {expected:?}"),
            );
            match &picture {
                Ok(picture) => {
                    let inside = picture.pixel(centre.0, centre.1);
                    let right = at(left + POPUP_SIZE.0 + 8.0, top + POPUP_SIZE.1 / 2.0);
                    let below = at(left + POPUP_SIZE.0 / 2.0, top + POPUP_SIZE.1 + 8.0);
                    let first = at(left + 3.0, top + 3.0);
                    let last = at(left + POPUP_SIZE.0 - 3.0, top + POPUP_SIZE.1 - 3.0);
                    let marker_pixel = at(MARKER_ORIGIN.0 + MARKER_SIZE / 2.0, MARKER_ORIGIN.1 + MARKER_SIZE / 2.0);
                    report.check(
                        "popup placed",
                        close_to(inside, POPUP_FILL)
                            && close_to(picture.pixel(first.0, first.1), POPUP_FILL)
                            && close_to(picture.pixel(last.0, last.1), POPUP_FILL)
                            && close_to(picture.pixel(right.0, right.1), FILL)
                            && close_to(picture.pixel(below.0, below.1), FILL)
                            && close_to(picture.pixel(marker_pixel.0, marker_pixel.1), MARKER_FILL),
                        format!(
                            "the composed output has the popup below the marker, its left edge at the marker's: centre {centre:?} is {inside:?}, its corners {:?} and {:?}, beside it {:?} and {:?}",
                            picture.pixel(first.0, first.1),
                            picture.pixel(last.0, last.1),
                            picture.pixel(right.0, right.1),
                            picture.pixel(below.0, below.1)
                        ),
                    );
                }
                Err(error) => report.check("popup placed", false, error.clone()),
            }

            // The pointer over the popup: its events arrive at the popup, not at the window.
            let over = (left + POPUP_SIZE.0 / 2.0, top + POPUP_SIZE.1 / 2.0);
            let sent = probe.pointer_move(over.0 as u32, over.1 as u32, extent);
            let entered = wait_for(STEP_TIMEOUT, || popup_child.is_pointer_over()).await;
            report.check(
                "popup input",
                sent.is_ok() && entered && !marker.is_pointer_over(),
                format!("the pointer at {over:?} of the output is over the content of the popup: {sent:?}"),
            );
            let _ = probe.pointer_move(20, 20, extent);

            // Moved: a reposition from version 3 of the shell, the popup created again before it.
            const SHIFT: f64 = 60.0;
            popup.set_horizontal_offset(SHIFT);
            let moved_in = at(left + POPUP_SIZE.0 + SHIFT - 10.0, top + POPUP_SIZE.1 / 2.0);
            let vacated = at(left + SHIFT / 2.0, top + POPUP_SIZE.1 / 2.0);
            match picture_with(probe, moved_in, POPUP_FILL).await {
                Ok(_) => {
                    // The place the popup left shows the window again.
                    let picture = picture_with(probe, vacated, FILL).await;
                    let counts = popup_counts(client).unwrap_or_default();
                    let expected = (left as i32 + SHIFT as i32, top as i32, POPUP_SIZE.0 as i32, POPUP_SIZE.1 as i32);
                    let configured = popup_configures(client);
                    report.check(
                        "moved popup configure",
                        configured == vec![expected],
                        format!("the compositor configured the moved popup at {configured:?}; expected {expected:?}"),
                    );
                    let (new, old) = match &picture {
                        Ok(picture) => (picture.pixel(moved_in.0, moved_in.1), picture.pixel(vacated.0, vacated.1)),
                        Err(_) => (None, None),
                    };
                    report.check(
                        "popup moved",
                        close_to(new, POPUP_FILL) && close_to(old, FILL) && counts.attached == 1 && counts.mapped == 1,
                        format!(
                            "an offset of {SHIFT} moves the popup ({}): {moved_in:?} is {new:?}, {vacated:?} is {old:?}; {counts:?}",
                            if counts.shell_version >= 3 { "xdg_popup.reposition" } else { "created again: the shell is older than version 3" }
                        ),
                    );
                }
                Err(error) => report.check("popup moved", false, error),
            }

            // Moved again and again at intervals of nothing to a few frames, so that some of
            // the moves are asked for while the answer to the configure of the move before is
            // on its way. On a shell before version 3 the popup then has another xdg_surface by
            // the time the serial of that configure arrives at the worker, which must not
            // acknowledge it there (the catalog ended with "wrong configure serial" on a popup
            // that was laid out twice while it opened).
            const MOVES: u32 = 24;
            for step in 1..=MOVES {
                popup.set_horizontal_offset(SHIFT + f64::from(step % 5) * 4.0 - 8.0);
                delay(Duration::from_millis(u64::from(step % 8) * 4)).await;
            }
            popup.set_horizontal_offset(SHIFT);
            let expected = (left as i32 + SHIFT as i32, top as i32, POPUP_SIZE.0 as i32, POPUP_SIZE.1 as i32);
            let settled = wait_for(STEP_TIMEOUT, || popup_configures(client) == vec![expected]).await;
            let picture = picture_with(probe, moved_in, POPUP_FILL).await;
            let pixel = picture.as_ref().ok().and_then(|picture| picture.pixel(moved_in.0, moved_in.1));
            let counts = popup_counts(client).unwrap_or_default();
            report.check(
                "popup moved in a hurry",
                settled && close_to(pixel, POPUP_FILL) && counts.mapped == 1 && probe.alive() && sway::view(TITLE).is_some(),
                format!(
                    "{MOVES} moves at intervals of 0 to 28 milliseconds: the popup ends at {:?} (expected {expected:?}), mapped ({counts:?}), {moved_in:?} is {pixel:?}, and the connection is still there",
                    popup_configures(client)
                ),
            );

            // A popup of the popup: its parent on the compositor is the popup.
            nested.set_is_open(true);
            let nested_centre = at(left + SHIFT + POPUP_SIZE.0 + NESTED_SIZE.0 / 2.0, top + POPUP_SIZE.1 / 2.0);
            let picture = picture_with(probe, nested_centre, NESTED_FILL).await;
            let counts = popup_counts(client).unwrap_or_default();
            let pixel = picture.as_ref().ok().and_then(|picture| picture.pixel(nested_centre.0, nested_centre.1));
            // The nested popup is placed against its parent popup: at its right edge, centred.
            let expected = (POPUP_SIZE.0 as i32, ((POPUP_SIZE.1 - NESTED_SIZE.1) / 2.0) as i32, NESTED_SIZE.0 as i32, NESTED_SIZE.1 as i32);
            let configured = popup_configures(client);
            report.check(
                "nested popup configure",
                configured.last() == Some(&expected) && configured.len() == 2,
                format!("the compositor configured the popups at {configured:?}; expected the second at {expected:?} of the first"),
            );
            report.check(
                "nested popup",
                close_to(pixel, NESTED_FILL) && counts.attached == 2 && counts.mapped == 2 && counts.nested == 1,
                format!("a popup to the right of the popup, whose parent is the popup: {nested_centre:?} is {pixel:?}; {counts:?}"),
            );
            nested.set_is_open(false);
            let closed = wait_for(STEP_TIMEOUT, || popup_counts(client).is_some_and(|counts| counts.registered == 1)).await;
            let picture = picture_with(probe, nested_centre, FILL).await;
            let pixel = picture.as_ref().ok().and_then(|picture| picture.pixel(nested_centre.0, nested_centre.1));
            report.check(
                "nested popup closed",
                closed && close_to(pixel, FILL) && popup.is_open(),
                format!("the popup of the popup is gone and its parent stays: {nested_centre:?} is {pixel:?}"),
            );

            // A press on the window beside the popup dismisses it.
            let beside = (left + POPUP_SIZE.0 + SHIFT + 150.0, top + POPUP_SIZE.1 + 120.0);
            let sent = probe
                .pointer_move(beside.0 as u32, beside.1 as u32, extent)
                .and_then(|()| probe.pointer_button(true))
                .and_then(|()| probe.pointer_button(false));
            let dismissed = wait_for(STEP_TIMEOUT, || !popup.is_open()).await;
            let gone = wait_for(STEP_TIMEOUT, || popup_counts(client).is_some_and(|counts| counts.registered == 0)).await;
            let picture = picture_with(probe, moved_in, FILL).await;
            let pixel = picture.as_ref().ok().and_then(|picture| picture.pixel(moved_in.0, moved_in.1));
            report.check(
                "popup dismissed",
                sent.is_ok() && dismissed && gone && close_to(pixel, FILL),
                format!("a press at {beside:?} closes the popup and its surface is destroyed: {sent:?}; {moved_in:?} is {pixel:?}"),
            );
            let alive = probe.alive() && sway::view(TITLE).is_some();
            report.check("after the popups", alive, "the connection and the window are still there".to_string());
            if popup.is_open() {
                popup.set_is_open(false);
            }
        }

        /// A themed cursor and a bitmap cursor: the worker makes both and the compositor takes
        /// the requests.
        async fn cursor_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>, client: Option<&Rc<WaylandWorkerClient>>) {
            let (Some(window_impl), Some(client), Some(factory)) =
                (window.platform_impl(), client, FerroLocator::current().get_service::<dyn ICursorFactory>())
            else {
                report.check("cursor factory", false, "the window, the worker client or the cursor factory is missing".to_string());
                return;
            };
            // The pointer is over the window (the input phase left it there, or it is moved now).
            let client_size = window.client_size();
            let _ = probe.pointer_move(50, 50, (client_size.width as u32, client_size.height as u32));

            let themed = factory.get_cursor(StandardCursorType::Ibeam);
            window_impl.set_cursor(Some(themed.clone()));
            // The command of the window goes with the next commit of the compositor of the
            // framework; the count is asked for after it.
            delay(Duration::from_millis(500)).await;
            let counts = client
                .invoke_oob(|worker| {
                    let themed_images = worker.state.globals.as_ref().map(|globals| {
                        worker.state.cursors.values().filter(|cursor| cursor.resolve(&globals.cursor_manager).is_some()).count()
                    });
                    (worker.state.cursors.len(), themed_images)
                })
                .recv_timeout(STEP_TIMEOUT);
            report.check(
                "themed cursor",
                matches!(counts, Ok((count, Some(_))) if count >= 1),
                format!("the worker has (cursors, cursors with an image): {counts:?} (an image needs a cursor theme on the system)"),
            );

            // A bitmap cursor of 16 by 16 opaque pixels.
            let size = PixelSize::new(16, 16);
            let pixels = vec![0xffu8; 16 * 16 * 4];
            let bitmap = ferroui_base::media::imaging::Bitmap::from_pixels(
                ferroui_base::platform::PixelFormat::BGRA8888,
                ferroui_base::platform::AlphaFormat::Premul,
                &pixels,
                size,
                Vector::new(96.0, 96.0),
                16 * 4,
            );
            let custom = factory.create_cursor(&bitmap, PixelPoint::new(8, 8));
            window_impl.set_cursor(Some(custom.clone()));
            delay(Duration::from_millis(500)).await;
            let resolved = client
                .invoke_oob(|worker| {
                    worker
                        .state
                        .cursors
                        .values()
                        .filter(|cursor| matches!(cursor, ferroui_wayland::server::persistent::wayland_cursor::WaylandCursor::Bitmap(_)))
                        .filter(|cursor| {
                            worker.state.globals.as_ref().is_some_and(|globals| cursor.resolve(&globals.cursor_manager).is_some())
                        })
                        .count()
                })
                .recv_timeout(STEP_TIMEOUT);
            report.check(
                "bitmap cursor",
                resolved == Ok(1),
                format!("bitmap cursors of the worker with a surface: {resolved:?}"),
            );

            window_impl.set_cursor(None);
            custom.dispose();
            themed.dispose();
            delay(Duration::from_millis(500)).await;
            let left = client.invoke_oob(|worker| worker.state.cursors.len()).recv_timeout(STEP_TIMEOUT);
            report.check("cursors released", left == Ok(0), format!("cursors of the worker after both were disposed: {left:?}"));
            report.check(
                "connection",
                probe.alive() && window.platform_impl().is_some(),
                "the compositor ended no connection after the cursor requests".to_string(),
            );
        }

        /// Another mode of the output: the compositor resizes the view, and the window follows.
        async fn resize_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>) {
            let Some((_, scale, output_name)) = output_geometry() else {
                report.check("output", false, "the compositor did not report its output".to_string());
                return;
            };
            let mode = format!("{}x{}", NEW_MODE.0, NEW_MODE.1);
            let changed = sway::message(&["output", &output_name, "resolution", &mode]).is_some();
            let expected = Size::new(f64::from(NEW_MODE.0) / scale, f64::from(NEW_MODE.1) / scale);
            let followed = wait_for(STEP_TIMEOUT, || window.client_size() == expected).await;
            report.check(
                "resized",
                changed && followed,
                format!("the output got the mode {mode} ({changed}); the framework has {:?}, expected {expected:?}", window.client_size()),
            );
            let rect = sway::view(TITLE).and_then(|view| sway::rect(&view, "rect"));
            report.check(
                "geometry",
                rect == Some((0, 0, expected.width as i32, expected.height as i32)),
                format!("the view is at {rect:?}"),
            );

            if probe.has_screencopy() {
                let corner = (NEW_MODE.0 - 6, NEW_MODE.1 - 6);
                match picture_with(probe, corner, FILL).await {
                    Ok(picture) => {
                        report.check(
                            "size",
                            (picture.width, picture.height) == NEW_MODE,
                            format!("the output has {} by {} pixels", picture.width, picture.height),
                        );
                        let pixel = picture.pixel(corner.0, corner.1);
                        report.check(
                            "bottom right",
                            close_to(pixel, FILL),
                            format!("the pixel at {corner:?} is {pixel:?}, the fill is {FILL:?}"),
                        );
                    }
                    Err(error) => report.check("screen copy", false, error),
                }
            }
        }

        /// Fullscreen and back: the request goes to the compositor, whose configure is the answer.
        async fn state_checks(report: &Report, window: &Ref<Window>) {
            let Some(window_impl) = window.platform_impl() else {
                report.check("window", false, "the window has no platform implementation".to_string());
                return;
            };
            let fullscreen_of_view = || sway::view(TITLE).and_then(|view| view.get("fullscreen_mode").and_then(json::Json::as_f64));

            window.set_window_state(WindowState::FullScreen);
            let entered = wait_for(STEP_TIMEOUT, || window_impl.window_state() == WindowState::FullScreen).await;
            let mode = fullscreen_of_view();
            report.check(
                "fullscreen",
                entered && mode == Some(1.0),
                format!("the platform has {:?}; the compositor says fullscreen_mode {mode:?}", window_impl.window_state()),
            );

            window.set_window_state(WindowState::Normal);
            let left = wait_for(STEP_TIMEOUT, || window_impl.window_state() == WindowState::Normal).await;
            let mode = fullscreen_of_view();
            report.check(
                "normal",
                left && mode == Some(0.0),
                format!("the platform has {:?}; the compositor says fullscreen_mode {mode:?}", window_impl.window_state()),
            );
        }

        /// A new title and size limits: the compositor shows the title and takes the limits.
        async fn title_checks(report: &Report, probe: &mut Probe, window: &Ref<Window>) {
            let new_title = format!("{TITLE} (renamed)");
            window.set_title(Some(new_title.clone()));
            let renamed = wait_for(STEP_TIMEOUT, || sway::view(&new_title).is_some()).await;
            report.check("title", renamed, format!("the compositor has a view with the title \"{new_title}\": {renamed}"));
            window.set_title(Some(TITLE.to_string()));
            let restored = wait_for(STEP_TIMEOUT, || sway::view(TITLE).is_some()).await;
            report.check("title restored", restored, format!("the view has the title \"{TITLE}\" again: {restored}"));

            if let Some(window_impl) = window.platform_impl() {
                // A minimum above the maximum is a protocol error that ends the connection: the
                // worker has to clamp it.
                window_impl.set_min_max_size(Size::new(400.0, 300.0), Size::new(200.0, 100.0));
                delay(Duration::from_millis(500)).await;
                window_impl.set_min_max_size(Size::new(0.0, 0.0), Size::new(f64::INFINITY, f64::INFINITY));
                delay(Duration::from_millis(500)).await;
                report.check(
                    "size limits",
                    probe.alive() && sway::view(TITLE).is_some(),
                    "the view is still there after limits whose minimum exceeded the maximum".to_string(),
                );
            }
        }

        fn request_close(window: &Ref<Window>) {
            let closed = Rc::new(Cell::new(false));
            {
                let closed = closed.clone();
                let _subscription = window.closed(move || closed.set(true));
            }
            println!("Asking the compositor to close the window (swaymsg kill)");
            if sway::message(&["kill"]).is_none() {
                println!("  [FAILED] close: the compositor did not take the request");
                SMOKE_PASSED.store(false, Ordering::SeqCst);
                window.close();
                return;
            }

            // The timer lives until the process ends.
            DispatcherTimer::run_once(
                move || {
                    if !closed.get() {
                        println!("  [FAILED] close: the window did not close after the compositor asked it to");
                        println!("SMOKE FAILED");
                        std::process::exit(1);
                    }
                },
                CLOSE_TIMEOUT,
                DispatcherPriority::BACKGROUND,
            )
            .detach();
        }
    }
}
