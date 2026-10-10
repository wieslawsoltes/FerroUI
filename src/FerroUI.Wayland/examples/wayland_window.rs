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
//!   backend.
//! - `--expect-scale=N`: the scale of the output, which the window has to
//!   take.
//! - `--skip=a,b`: phases to leave out (`screens`, `frames`, `input`, `popup`, `decorations`,
//!   `cursor`, `resize`, `state`, `title`).
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

    thread_local! {
        /// The marker of the main window: what the popup of the smoke mode is placed at.
        static MARKER: RefCell<Option<Ref<Border>>> = const { RefCell::new(None) };
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

    pub fn run() -> ExitCode {
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
        if mode() != "egl" {
            // No profile to make a context with: the display of EGL is not created, and the
            // worker renders through shared memory buffers.
            options.gl_profiles = Vec::new();
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
        use ferroui_wayland::server::interop::unsafe_native_methods::{memfd_of_length, MemoryMapping};
        use std::os::fd::AsFd;
        use wayland_client::protocol::wl_buffer::WlBuffer;
        use wayland_client::protocol::wl_output::WlOutput;
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

        #[derive(Default)]
        struct State {
            globals: Vec<(u32, String, u32)>,
            buffer: Option<BufferInfo>,
            y_invert: bool,
            ready: bool,
            failed: bool,
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
        delegate_noop!(State: ignore WlSeat);
        delegate_noop!(State: ignore WlOutput);
        delegate_noop!(State: ignore WlShmPool);
        delegate_noop!(State: ignore WlBuffer);
        delegate_noop!(State: ignore ZwlrScreencopyManagerV1);
        delegate_noop!(State: ignore ZwlrVirtualPointerManagerV1);
        delegate_noop!(State: ignore ZwlrVirtualPointerV1);
        delegate_noop!(State: ignore ZwpVirtualKeyboardManagerV1);
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
                queue.roundtrip(&mut state).map_err(|error| error.to_string())?;

                Ok(Self { connection, queue, state, shm, output, screencopy, pointer, keyboard, time: 1000 })
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
        const RUN_TIMEOUT: Duration = Duration::from_secs(180);
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

        /// A press at a point of the output, a drag by an offset in steps, and a release.
        fn drag(probe: &mut Probe, from: (i32, i32), by: (i32, i32), extent: (u32, u32)) -> Result<(), String> {
            let clamp = |value: i32, limit: u32| value.clamp(0, limit as i32 - 1) as u32;
            probe.pointer_move(clamp(from.0, extent.0), clamp(from.1, extent.1), extent)?;
            probe.pointer_button(true)?;
            for step in 1..=8 {
                let x = from.0 + by.0 * step / 8;
                let y = from.1 + by.1 * step / 8;
                probe.pointer_move(clamp(x, extent.0), clamp(y, extent.1), extent)?;
                std::thread::sleep(Duration::from_millis(30));
            }
            probe.pointer_button(false)
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
            report.check(
                "window geometry",
                match (geometry, view) {
                    (Some((left, top, width, height)), Some((_, _, view_width, view_height))) => {
                        left >= 0 && top >= 0 && (width, height) == (view_width, view_height)
                    }
                    _ => false,
                },
                format!("the window geometry sent with the frame is {geometry:?} (left, top, width, height); the view of the compositor is {view:?}"),
            );
            let (Some(title_bar), Some(_)) = (title_bar, geometry) else {
                return;
            };

            // A floating window, which the compositor lets the pointer move and resize.
            let floated = sway::message(&["floating", "enable"]).is_some();
            delay(Duration::from_millis(800)).await;
            let (Some(view), Some(geometry), Some(((output_width, output_height), _, _))) =
                (view_rect(), window_geometry(client), output_geometry())
            else {
                report.check("floating", false, format!("the compositor did not float the window (asked: {floated})"));
                return;
            };
            let extent = (output_width as u32, output_height as u32);
            // A point of the client area in the coordinates of the output.
            let to_output = |x: f64, y: f64| (view.0 + x as i32 - geometry.0, view.1 + y as i32 - geometry.1);
            let input_root = window.input_root();

            let title_point = (f64::from(geometry.0) + f64::from(geometry.2) / 2.0, f64::from(geometry.1) + title_bar / 2.0);
            let role = input_root.hit_test_chrome_element(Point::new(title_point.0, title_point.1));
            let from = to_output(title_point.0, title_point.1);
            let sent = drag(probe, from, (90, 60), extent);
            let moved = wait_for(STEP_TIMEOUT, || view_rect().is_some_and(|now| (now.0 - view.0).abs() >= 40 || (now.1 - view.1).abs() >= 25)).await;
            let after_move = view_rect();
            report.check(
                "interactive move",
                role == Some(WindowDecorationsElementRole::TitleBar) && sent.is_ok() && moved,
                format!(
                    "a drag of the drawn title bar (role {role:?}) from {from:?} by (90, 60): the view went from {view:?} to {after_move:?}: {sent:?}"
                ),
            );

            // The grip of the bottom right corner: the first point along the diagonal through
            // the corner of the window geometry that the decorations name so.
            let (Some(view), Some(geometry)) = (view_rect(), window_geometry(client)) else {
                report.check("interactive resize", false, "the view or the window geometry is gone".to_string());
                return;
            };
            let to_output = |x: f64, y: f64| (view.0 + x as i32 - geometry.0, view.1 + y as i32 - geometry.1);
            let corner = (f64::from(geometry.0 + geometry.2), f64::from(geometry.1 + geometry.3));
            let grip = (-16..=24).map(|d| (corner.0 + f64::from(d), corner.1 + f64::from(d))).find(|point| {
                input_root.hit_test_chrome_element(Point::new(point.0, point.1)) == Some(WindowDecorationsElementRole::ResizeSE)
            });
            let Some(grip) = grip else {
                report.check(
                    "interactive resize",
                    false,
                    format!("no point around the corner {corner:?} of the window geometry {geometry:?} is the bottom right resize grip"),
                );
                return;
            };
            let from = to_output(grip.0, grip.1);
            let size_before = window.client_size();
            let sent = drag(probe, from, (70, 50), extent);
            let resized = wait_for(STEP_TIMEOUT, || {
                view_rect().is_some_and(|now| now.2 - view.2 >= 30 && now.3 - view.3 >= 20)
                    && window.client_size().width - size_before.width >= 30.0
            })
            .await;
            let after_resize = view_rect();
            report.check(
                "interactive resize",
                sent.is_ok() && resized,
                format!(
                    "a drag of the drawn grip at {grip:?} of the client area ({from:?} of the output) by (70, 50): the view went from {view:?} to {after_resize:?}, the client size from {size_before:?} to {:?}: {sent:?}",
                    window.client_size()
                ),
            );
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

            // A popup of the popup: its parent on the compositor is the popup.
            nested.set_is_open(true);
            let nested_centre = at(left + SHIFT + POPUP_SIZE.0 + NESTED_SIZE.0 / 2.0, top + POPUP_SIZE.1 / 2.0);
            let picture = picture_with(probe, nested_centre, NESTED_FILL).await;
            let counts = popup_counts(client).unwrap_or_default();
            let pixel = picture.as_ref().ok().and_then(|picture| picture.pixel(nested_centre.0, nested_centre.1));
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

            let _watchdog = DispatcherTimer::run_once(
                move || {
                    if !closed.get() {
                        println!("  [FAILED] close: the window did not close after the compositor asked it to");
                        println!("SMOKE FAILED");
                        std::process::exit(1);
                    }
                },
                CLOSE_TIMEOUT,
                DispatcherPriority::BACKGROUND,
            );
            // The timer lives until the process ends.
            std::mem::forget(_watchdog);
        }
    }
}
