//! The smoke application of the Android platform: a shared library that
//! `scripts/android/apk.sh` packages and `scripts/android/emu-smoke.sh` runs
//! on the emulator (`docs/porting/android-platform.md`, sections 9 and 10).
//!
//! ```text
//! scripts/android/apk.sh smoke
//! scripts/android/emu-smoke.sh
//! ```
//!
//! The application shows a view with a stripe, a line of text and an
//! ellipse, drawn by the compositor with Skia on the render thread, and
//! checks itself: the activity, the view and its surface; the size and the
//! scaling against the screen; the insets; the dispatcher (a timer, and a
//! job posted from another thread); frames, by reading pixels back from the
//! surface that is being drawn, inside the frame; touch input the script
//! injects; and the end of the activity. Every check is a line `[ ok ]` or
//! `[FAIL]` in the log of the system under the tag `ferroui-smoke` and in
//! the file `smoke-report.txt` of the files directory of the application;
//! the last line is `RESULT: PASS` or `RESULT: FAIL`.
//!
//! The file `smoke.properties` of the files directory, when the script has
//! written it, selects what is run: `mode=software` renders through the
//! native window instead of EGL, `input=0` leaves out the checks that need
//! the script to act (touch, keys, the night mode).
//!
//! A line `SCRIPT <command> <arguments>` asks the script that runs the
//! application to do something on the device (`scripts/android/emu-smoke.sh`
//! lists the commands).
//!
//! On another system than Android the library is empty.

#[cfg(target_os = "android")]
ferroui_android::android_application!(app::build);

#[cfg(target_os = "android")]
mod app {
    use ferroui_android::interop::java::{call_int, call_object, string_of};
    use ferroui_android::log::{self, LogPriority};
    use ferroui_android::{
        AndroidApplicationExtensions, AndroidPlatform, AndroidPlatformOptions, AndroidRenderingMode,
        AndroidViewControlHandle, FerroActivity, FerroAndroidApplication,
    };
    use ferroui_base::input::platform::ClipboardExtensions;
    use ferroui_base::platform::storage::WellKnownFolder;
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::utilities::Uri;
    use ferroui_controls::platform::{
        FeedbackAction, FeedbackType, INativeControlHostControlTopLevelAttachment,
        INativeControlHostDestroyableControlHandle, INativeControlHostImpl, IPlatformFeedback, IPlatformHandle,
    };
    use ferroui_base::input::text_input::{
        TextInputMethodClient, TextInputMethodClientEvents, TextInputMethodClientRequestedEventArgs, TextSelection,
    };
    use ferroui_base::input::{
        InputElement, InputElementImpl, Key, KeyEventArgs, PointerEventArgs, PointerPressedEventArgs,
        PointerReleasedEventArgs, PointerType, TextInputEventArgs,
    };
    use ferroui_base::interactivity::{Interactive, InteractiveImpl};
    use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl};
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Color, DrawingContext, IBrush, ImmediateDrawingContext};
    use ferroui_base::platform::{IPlatformSettings, PlatformColorValues, PlatformThemeVariant};
    use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
    use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
    use ferroui_base::{
        ferro_class, ferro_impl_classes, instantiate, FerroLocator, FerroObjectImpl, LocatorExtensions, Point, Rect,
        Ref, StyledElementImpl, Thickness, Visual, VisualImpl,
    };
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::application_lifetimes::{ActivatedEventArgs, ActivationKind, IActivatableLifetime};
    use ferroui_controls::platform::{InputPaneState, InputPaneStateEventArgs};
    use ferroui_controls::shapes::Ellipse;
    use ferroui_controls::{
        AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Border, Control, ControlImpl, NewApplication,
        Panel, StackPanel, TextBlock, TopLevel,
    };
    use ferroui_skia::ISkiaApiLeaseFeature;
    use std::any::TypeId;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::{Duration, Instant};

    /// The tag of the lines of the smoke run.
    const TAG: &str = "ferroui-smoke";

    /// The colours drawn.
    const FILL: [u8; 3] = [0x33, 0x66, 0x99];
    const STRIPE: [u8; 3] = [0xcc, 0x33, 0x33];
    const ELLIPSE: [u8; 3] = [0xff, 0xcc, 0x00];
    const STRIPE_HEIGHT: f64 = 96.0;
    const ELLIPSE_SIZE: f64 = 120.0;

    fn brush(color: [u8; 3]) -> Option<Rc<dyn IBrush>> {
        Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(color[0], color[1], color[2]))))
    }

    // ---- the options of the run -----------------------------------------------------------

    #[derive(Clone, Copy)]
    struct SmokeOptions {
        software: bool,
        input: bool,
    }

    fn files_dir() -> Option<String> {
        let context = FerroAndroidApplication::context();
        let directory = call_object(&context, "getFilesDir", "()Ljava/io/File;", &[])?;
        let path = call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])?;
        Some(string_of(&path))
    }

    fn read_options() -> SmokeOptions {
        let mut options = SmokeOptions { software: false, input: true };
        let text =
            files_dir().and_then(|directory| std::fs::read_to_string(format!("{directory}/smoke.properties")).ok());
        for line in text.as_deref().unwrap_or_default().lines() {
            match line.trim().split_once('=') {
                Some(("mode", value)) => options.software = value.trim() == "software",
                Some(("input", value)) => options.input = value.trim() != "0",
                _ => {}
            }
        }
        options
    }

    thread_local! {
        static OPTIONS: Cell<SmokeOptions> = const { Cell::new(SmokeOptions { software: false, input: true }) };
        static REPORT: Report = Report::default();
    }

    // ---- the report -----------------------------------------------------------------------

    #[derive(Default)]
    struct Report {
        lines: RefCell<Vec<String>>,
        failed: Cell<bool>,
    }

    fn check(name: &str, ok: bool, detail: impl AsRef<str>) {
        let line = format!("{} {name}: {}", if ok { "[ ok ]" } else { "[FAIL]" }, detail.as_ref());
        log::write(if ok { LogPriority::Info } else { LogPriority::Error }, TAG, &line);
        REPORT.with(|report| {
            if !ok {
                report.failed.set(true);
            }
            report.lines.borrow_mut().push(line);
        });
    }

    fn note(text: impl AsRef<str>) {
        log::write(LogPriority::Info, TAG, text.as_ref());
        REPORT.with(|report| report.lines.borrow_mut().push(text.as_ref().to_string()));
    }

    /// Writes the result line and the report file.
    fn finish_report() {
        let passed = REPORT.with(|report| !report.failed.get());
        note(if passed { "RESULT: PASS" } else { "RESULT: FAIL" });
        let lines = REPORT.with(|report| report.lines.borrow().join("\n"));
        match files_dir() {
            Some(directory) => {
                if let Err(error) = std::fs::write(format!("{directory}/smoke-report.txt"), lines + "\n") {
                    log::write(LogPriority::Error, TAG, &format!("the report file could not be written: {error}"));
                }
            }
            None => log::write(LogPriority::Error, TAG, "the application has no files directory"),
        }
        log::write(LogPriority::Info, TAG, "REPORT WRITTEN");
    }

    // ---- the application ------------------------------------------------------------------

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
            this.set_name(Some("FerroUI Android smoke".to_string()));
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            match lifetime.as_ref().and_then(|lifetime| lifetime.as_activity_application_lifetime()) {
                Some(lifetime) => {
                    lifetime.set_main_view_factory(Some(Rc::new(create_main_view)));
                    check("lifetime", true, "the application has the activity lifetime of the platform");
                }
                None => check("lifetime", false, "the application has no activity lifetime"),
            }

            Self::parent_on_framework_initialization_completed(this);
        }
    }

    /// The application builder: what `android_application!` is given.
    pub fn build() -> AppBuilder {
        let smoke = read_options();
        OPTIONS.with(|options| options.set(smoke));

        let options = AndroidPlatformOptions {
            rendering_mode: if smoke.software {
                vec![AndroidRenderingMode::Software]
            } else {
                AndroidPlatformOptions::default().rendering_mode
            },
        };
        note(format!(
            "The library is loaded. Rendering modes asked for: {:?}; touch checks: {}",
            options.rendering_mode, smoke.input
        ));

        AppBuilder::configure::<App>().with(Rc::new(options)).use_android().after_setup(|_| {
            check(
                "setup",
                Dispatcher::ui_thread().check_access() && AndroidPlatform::options().is_some(),
                "the platform is initialized and the dispatcher belongs to the main thread",
            );
        })
    }

    // ---- the view -------------------------------------------------------------------------

    /// What the render thread reads of a frame, and what the UI thread asks
    /// it to read.
    #[derive(Default)]
    struct Readback {
        /// The points to read, in pixels of the surface, with their names.
        points: Vec<(&'static str, i32, i32)>,
        /// The rectangle of the line of text, in pixels of the surface.
        text: Option<(i32, i32, i32, i32)>,
        /// The frames read so far.
        frames: u32,
        thread: String,
        surface: (i32, i32),
        gpu: bool,
        colors: Vec<(&'static str, [u8; 4])>,
        /// The pixels of the rectangle of the text that are nearly white.
        text_pixels: usize,
        error: Option<String>,
    }

    /// A control over the whole view that draws nothing and reads the frame
    /// back: its custom drawing operation runs on the thread that renders,
    /// after everything under it was drawn, with the Skia surface of the
    /// frame.
    #[repr(C)]
    struct Probe {
        base: Control,
        readback: Arc<Mutex<Readback>>,
    }

    ferro_class!(Probe: Control);
    ferro_impl_classes!(Probe: FerroObjectImpl, StyledElementImpl, InteractiveImpl, ControlImpl);

    impl InputElementImpl for Probe {}
    impl LayoutableImpl for Probe {}

    impl VisualImpl for Probe {
        fn render(this: &Self, context: &mut DrawingContext) {
            let bounds = this.bounds();
            let op: Arc<dyn ICustomDrawOperation> = Arc::new(ProbeOp {
                bounds: Rect::new(0.0, 0.0, bounds.width, bounds.height),
                readback: this.readback.clone(),
            });
            context.custom(&op);
        }
    }

    impl Probe {
        fn new(readback: Arc<Mutex<Readback>>) -> Ref<Self> {
            let this = instantiate(Self { base: Control::construct(), readback });
            this.set_is_hit_test_visible(false);
            this
        }
    }

    struct ProbeOp {
        bounds: Rect,
        readback: Arc<Mutex<Readback>>,
    }

    impl ICustomDrawOperation for ProbeOp {
        fn bounds(&self) -> Rect {
            self.bounds
        }

        fn hit_test(&self, _p: Point) -> bool {
            false
        }

        fn equals(&self, _other: &dyn ICustomDrawOperation) -> bool {
            false
        }

        fn dispose(&self) {}

        fn render(&self, context: &mut ImmediateDrawingContext<'_>) {
            let mut readback = self.readback.lock().unwrap_or_else(PoisonError::into_inner);
            if readback.frames >= 12 || readback.points.is_empty() {
                return;
            }

            let lease = context
                .try_get_feature(TypeId::of::<dyn ISkiaApiLeaseFeature>())
                .and_then(|feature| feature.downcast_ref::<Rc<dyn ISkiaApiLeaseFeature>>().cloned());
            let Some(lease) = lease else {
                readback.error =
                    Some("the drawing context of the frame gives no access to its Skia surface".to_string());
                return;
            };
            let lease = lease.lease();
            readback.gpu = lease.gr_context().is_some();
            let Some(mut surface) = lease.sk_surface() else {
                readback.error = Some("the frame has no Skia surface".to_string());
                lease.dispose();
                return;
            };

            readback.surface = (surface.width(), surface.height());
            readback.thread = std::thread::current().name().unwrap_or("(unnamed)").to_string();

            let read = |surface: &mut skia_safe::Surface, x: i32, y: i32, width: i32, height: i32| -> Option<Vec<u8>> {
                let info = skia_safe::ImageInfo::new(
                    (width, height),
                    skia_safe::ColorType::RGBA8888,
                    skia_safe::AlphaType::Premul,
                    None,
                );
                let mut pixels = vec![0u8; width as usize * height as usize * 4];
                surface.read_pixels(&info, &mut pixels, width as usize * 4, (x, y)).then_some(pixels)
            };

            let points = readback.points.clone();
            let mut colors = Vec::new();
            for (name, x, y) in points {
                match read(&mut surface, x, y, 1, 1) {
                    Some(pixel) => colors.push((name, [pixel[0], pixel[1], pixel[2], pixel[3]])),
                    None => readback.error = Some(format!("the pixel at {x},{y} ({name}) could not be read back")),
                }
            }
            readback.colors = colors;

            if let Some((x, y, width, height)) = readback.text {
                let (width, height) = (width.min(surface.width() - x), height.min(surface.height() - y));
                if width > 0 && height > 0 {
                    match read(&mut surface, x, y, width, height) {
                        Some(pixels) => {
                            readback.text_pixels =
                                pixels.chunks_exact(4).filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200).count();
                        }
                        None => readback.error = Some("the rectangle of the text could not be read back".to_string()),
                    }
                }
            }

            readback.frames += 1;
            lease.dispose();
        }
    }

    /// What the pointer handlers of the view saw.
    #[derive(Default)]
    struct Pointers {
        pressed: RefCell<Vec<Point>>,
        moved: Cell<u32>,
        released: RefCell<Vec<Point>>,
    }

    /// The client of the input method of the smoke run: a text and a
    /// selection in UTF-16 code units, edited as a text box edits its own
    /// by the text input and the delete keys the editor of the view gets.
    struct EditorClient {
        events: TextInputMethodClientEvents,
        visual: Ref<Visual>,
        text: RefCell<Vec<u16>>,
        selection: Cell<TextSelection>,
        /// The text inputs the editor got while it was the client.
        inputs: Cell<u32>,
    }

    impl EditorClient {
        fn text(&self) -> String {
            String::from_utf16_lossy(&self.text.borrow())
        }

        fn ordered(&self) -> (usize, usize) {
            let selection = self.selection.get();
            let length = self.text.borrow().len() as i32;
            let start = selection.start.min(selection.end).clamp(0, length);
            let end = selection.start.max(selection.end).clamp(0, length);
            (start as usize, end as usize)
        }

        fn replace(&self, start: usize, end: usize, with: &str) {
            let units: Vec<u16> = with.encode_utf16().collect();
            let caret = (start + units.len()) as i32;
            self.text.borrow_mut().splice(start..end, units);
            self.selection.set(TextSelection::new(caret, caret));
            self.raise_surrounding_text_changed();
            self.raise_selection_changed();
        }

        fn input(&self, text: &str) {
            self.inputs.set(self.inputs.get() + 1);
            let (start, end) = self.ordered();
            self.replace(start, end, text);
        }

        fn delete(&self, forward: bool) {
            let (start, end) = self.ordered();
            let length = self.text.borrow().len();
            if end > start {
                self.replace(start, end, "");
            } else if forward && start < length {
                self.replace(start, start + 1, "");
            } else if !forward && start > 0 {
                self.replace(start - 1, start, "");
            }
        }
    }

    impl TextInputMethodClient for EditorClient {
        fn events(&self) -> &TextInputMethodClientEvents {
            &self.events
        }

        fn text_view_visual(&self) -> Ref<Visual> {
            self.visual.clone()
        }

        fn supports_preedit(&self) -> bool {
            false
        }

        fn supports_surrounding_text(&self) -> bool {
            true
        }

        fn surrounding_text(&self) -> String {
            self.text()
        }

        fn cursor_rectangle(&self) -> Rect {
            Rect::default()
        }

        fn selection(&self) -> TextSelection {
            self.selection.get()
        }

        fn set_selection(&self, value: TextSelection) {
            self.selection.set(value);
        }
    }

    struct MainView {
        editor: Ref<Border>,
        /// Something else that takes the focus, and has no text to edit.
        other: Ref<Border>,
        client: Rc<EditorClient>,
        root: Ref<Panel>,
        probe: Ref<Probe>,
        text: Ref<TextBlock>,
        ellipse: Ref<Ellipse>,
        readback: Arc<Mutex<Readback>>,
        pointers: Rc<Pointers>,
    }

    fn create_main_view() -> Ref<Control> {
        let stripe = Border::new();
        stripe.set_height(STRIPE_HEIGHT);
        stripe.set_background(brush(STRIPE));

        let text = TextBlock::new();
        text.set_text(Some("FerroUI on Android"));
        text.set_font_size(28.0);
        text.set_foreground(brush([0xff, 0xff, 0xff]));
        text.set_margin(Thickness::uniform(16.0));
        text.set_horizontal_alignment(HorizontalAlignment::Center);

        let ellipse = Ellipse::new();
        ellipse.set_width(ELLIPSE_SIZE);
        ellipse.set_height(ELLIPSE_SIZE);
        ellipse.set_fill(brush(ELLIPSE));
        ellipse.set_horizontal_alignment(HorizontalAlignment::Center);

        let stack = StackPanel::new();
        stack.children().add(stripe);
        stack.children().add(text.clone());
        stack.children().add(ellipse.clone());

        let background = Border::new();
        background.set_background(brush(FILL));
        background.set_child(stack);

        let readback = Arc::new(Mutex::new(Readback::default()));
        let probe = Probe::new(readback.clone());

        // The editor of the input method checks, and something else to move the focus to:
        // two elements of one pixel that draw nothing.
        let focusable = || {
            let border = Border::new();
            border.set_width(1.0);
            border.set_height(1.0);
            border.set_focusable(true);
            border
        };
        let (editor, other) = (focusable(), focusable());
        let client = Rc::new(EditorClient {
            events: TextInputMethodClientEvents::new(),
            visual: editor.clone().upcast(),
            text: RefCell::new(Vec::new()),
            selection: Cell::new(TextSelection::default()),
            inputs: Cell::new(0),
        });
        editor.add_handler(InputElement::text_input_method_client_requested_event(), {
            let client = client.clone();
            move |_: &Interactive, e: &TextInputMethodClientRequestedEventArgs| {
                let client: Rc<dyn TextInputMethodClient> = client.clone();
                e.set_client(Some(client));
            }
        });
        editor.add_handler(InputElement::text_input_event(), {
            let client = client.clone();
            move |_: &Interactive, e: &TextInputEventArgs| client.input(e.text.as_deref().unwrap_or_default())
        });
        editor.add_handler(InputElement::key_down_event(), {
            let client = client.clone();
            move |_: &Interactive, e: &KeyEventArgs| match e.key {
                Key::Delete => client.delete(true),
                Key::Back => client.delete(false),
                _ => {}
            }
        });

        let root = Panel::new();
        root.children().add(background);
        root.children().add(editor.clone());
        root.children().add(other.clone());
        root.children().add(probe.clone());

        let pointers = Rc::new(Pointers::default());
        root.add_handler(InputElement::pointer_pressed_event(), {
            let (pointers, root) = (pointers.clone(), root.clone());
            move |_: &Interactive, e: &PointerPressedEventArgs| {
                pointers.pressed.borrow_mut().push(e.get_position(Some(&root)));
            }
        });
        root.add_handler(InputElement::pointer_moved_event(), {
            let pointers = pointers.clone();
            move |_: &Interactive, _: &PointerEventArgs| pointers.moved.set(pointers.moved.get() + 1)
        });
        root.add_handler(InputElement::pointer_released_event(), {
            let (pointers, root) = (pointers.clone(), root.clone());
            move |_: &Interactive, e: &PointerReleasedEventArgs| {
                pointers.released.borrow_mut().push(e.get_position(Some(&root)));
            }
        });

        smoke::start(Rc::new(MainView { editor, other, client, root: root.clone(), probe, text, ellipse, readback, pointers }));

        root.upcast()
    }

    // ---- the checks -----------------------------------------------------------------------

    mod smoke {
        use super::*;

        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum Stage {
            WaitForSurface,
            Dispatcher,
            Frames,
            Input,
            Keys,
            InputPane,
            InputMethod,
            InputPaneClosed,
            NativeControl,
            Settings,
            Background,
            Foreground,
            Rotated,
            RotatedBack,
            Protocol,
            BackHandled,
            BackNotHandled,
            BackResumed,
            Finish,
            WaitForDestroy,
            Done,
        }

        struct State {
            view: Rc<MainView>,
            stage: Cell<Stage>,
            /// When the stage began.
            since: Cell<Instant>,
            timer_fired: Rc<Cell<bool>>,
            posted: Arc<AtomicBool>,
            posted_on_main: Arc<AtomicBool>,
            /// The pixel the script is asked to tap, and the logical point
            /// it is.
            tap: Cell<(i32, i32, Point)>,
            scaling: Cell<f64>,
            /// A step of a stage that is taken once.
            aimed: Cell<bool>,
            keys: Rc<Keys>,
            /// The colour values the platform settings raised as changed.
            color_changes: Rc<RefCell<Vec<PlatformColorValues>>>,
            /// The theme of the system when the settings stage began, and how
            /// far the stage is: 0 asked for the other theme, 1 asked for the
            /// first one again.
            first_theme: Cell<PlatformThemeVariant>,
            settings_step: Cell<u32>,
            /// The changes of the state of the input pane.
            pane_changes: Rc<RefCell<Vec<(InputPaneState, Rect, Duration, bool)>>>,
            lifecycle: Rc<Lifecycle>,
            /// The native control of the native control host check, and its attachment.
            native_control: RefCell<Option<NativeControl>>,
            /// Whether a back request is handled, and how many arrived.
            handle_back: Rc<Cell<bool>>,
            back_requests: Rc<Cell<u32>>,
        }

        type NativeControl =
            (Rc<dyn INativeControlHostDestroyableControlHandle>, Rc<dyn INativeControlHostControlTopLevelAttachment>);

        /// The value of a future that is complete when it is first asked.
        fn now<T>(future: impl std::future::Future<Output = T>) -> Option<T> {
            let future = std::pin::pin!(future);
            match future.poll(&mut std::task::Context::from_waker(std::task::Waker::noop())) {
                std::task::Poll::Ready(value) => Some(value),
                std::task::Poll::Pending => None,
            }
        }

        /// A feature of the platform implementation of the top-level.
        fn feature<T: ?Sized + 'static>(state: &State) -> Option<Rc<T>> {
            let platform_impl = top_level(state)?.platform_impl()?;
            let feature = platform_impl.try_get_feature(TypeId::of::<T>())?;
            feature.downcast_ref::<Rc<T>>().cloned()
        }

        /// What the activatable lifetime of the application raised.
        #[derive(Default)]
        struct Lifecycle {
            activated: RefCell<Vec<ActivationKind>>,
            deactivated: RefCell<Vec<ActivationKind>>,
            protocols: RefCell<Vec<String>>,
        }

        /// The key and text events the top-level raised.
        #[derive(Default)]
        struct Keys {
            down: RefCell<Vec<(Key, Option<String>)>>,
            up: RefCell<Vec<Key>>,
            text: RefCell<String>,
        }

        thread_local! {
            /// How many times the main view was made: once per start of the activity.
            static STARTS: Cell<u32> = const { Cell::new(0) };
        }

        fn second_start() -> bool {
            STARTS.with(Cell::get) >= 2
        }

        pub fn start(view: Rc<MainView>) {
            STARTS.with(|starts| starts.set(starts.get() + 1));
            if second_start() {
                note("The activity was started again in the process: the checks of the surface and of the frames follow.");
            }
            let state = Rc::new(State {
                view,
                stage: Cell::new(Stage::WaitForSurface),
                since: Cell::new(Instant::now()),
                timer_fired: Rc::new(Cell::new(false)),
                posted: Arc::new(AtomicBool::new(false)),
                posted_on_main: Arc::new(AtomicBool::new(false)),
                tap: Cell::new((0, 0, Point::default())),
                scaling: Cell::new(1.0),
                aimed: Cell::new(false),
                keys: Rc::new(Keys::default()),
                color_changes: Rc::new(RefCell::new(Vec::new())),
                first_theme: Cell::new(PlatformThemeVariant::Light),
                settings_step: Cell::new(0),
                pane_changes: Rc::new(RefCell::new(Vec::new())),
                lifecycle: Rc::new(Lifecycle::default()),
                native_control: RefCell::new(None),
                handle_back: Rc::new(Cell::new(true)),
                back_requests: Rc::new(Cell::new(0)),
            });

            let timer = DispatcherTimer::new();
            timer.set_interval(Duration::from_millis(100));
            let _ = timer.tick({
                let state = state.clone();
                move |timer: &DispatcherTimer| {
                    step(&state);
                    if state.stage.get() == Stage::Done {
                        timer.stop();
                    }
                }
            });
            timer.start();
            // The timer lives as long as the application.
            std::mem::forget(timer);
        }

        fn enter(state: &State, stage: Stage) {
            state.stage.set(stage);
            state.since.set(Instant::now());
        }

        fn elapsed(state: &State) -> Duration {
            state.since.get().elapsed()
        }

        fn top_level(state: &State) -> Option<Ref<TopLevel>> {
            TopLevel::get_top_level(Some(&state.view.root))
        }

        fn near(pixel: [u8; 4], color: [u8; 3]) -> bool {
            (0..3).all(|i| (i32::from(pixel[i]) - i32::from(color[i])).abs() <= 3)
        }

        fn step(state: &Rc<State>) {
            match state.stage.get() {
                Stage::WaitForSurface => wait_for_surface(state),
                Stage::Dispatcher => dispatcher(state),
                Stage::Frames => frames(state),
                Stage::Input => input(state),
                Stage::Keys => keys(state),
                Stage::InputPane => input_pane(state),
                Stage::InputMethod => input_method(state),
                Stage::InputPaneClosed => input_pane_closed(state),
                Stage::NativeControl => native_control(state),
                Stage::Settings => settings(state),
                Stage::Background => background(state),
                Stage::Foreground => foreground(state),
                Stage::Rotated => rotated(state),
                Stage::RotatedBack => rotated_back(state),
                Stage::Protocol => protocol(state),
                Stage::BackHandled => back_handled(state),
                Stage::BackNotHandled => back_not_handled(state),
                Stage::BackResumed => back_resumed(state),
                Stage::Finish => finish(state),
                Stage::WaitForDestroy => wait_for_destroy(state),
                Stage::Done => {}
            }
        }

        fn wait_for_surface(state: &Rc<State>) {
            let activity = FerroActivity::current_main_activity();
            let top_level = top_level(state);
            let size = top_level.as_ref().map(|top_level| top_level.client_size());
            let ready = activity.is_some() && size.is_some_and(|size| size.width > 1.0 && size.height > 1.0);
            if !ready {
                if elapsed(state) > Duration::from_secs(20) {
                    check(
                        "surface",
                        false,
                        format!(
                            "no surface after 20 s (main activity resumed: {}, top-level: {}, client size: {size:?})",
                            activity.is_some(),
                            top_level.is_some()
                        ),
                    );
                    enter(state, Stage::Finish);
                }
                return;
            }
            let (Some(activity), Some(top_level), Some(size)) = (activity, top_level, size) else {
                return;
            };

            check(
                "activity",
                activity.is_main_activity() && activity.view().is_some(),
                "the main activity was created and resumed, and has its view",
            );

            let scaling = top_level.render_scaling();
            state.scaling.set(scaling);
            let pixels = (size.width * scaling, size.height * scaling);
            check(
                "surface",
                scaling >= 1.0
                    && (pixels.0 - pixels.0.round()).abs() < 1e-6
                    && (pixels.1 - pixels.1.round()).abs() < 1e-6,
                format!(
                    "the client size is {}x{} at scaling {scaling}: a surface of {}x{} px",
                    size.width,
                    size.height,
                    pixels.0.round(),
                    pixels.1.round()
                ),
            );

            // The screen.
            match top_level.screens() {
                Some(screens) => {
                    let all = screens.all();
                    let primary = screens.primary();
                    let own = screens.screen_from_top_level(&top_level);
                    let described = own.as_ref().map(|screen| {
                        format!(
                            "'{}', bounds {:?}, working area {:?}, scaling {}, orientation {:?}",
                            screen.display_name().unwrap_or_default(),
                            screen.bounds(),
                            screen.working_area(),
                            screen.scaling(),
                            screen.current_orientation()
                        )
                    });
                    let ok = !all.is_empty()
                        && primary.is_some()
                        && own.as_ref().is_some_and(|screen| {
                            f64::from(screen.bounds().width) >= pixels.0
                                && f64::from(screen.bounds().height) >= pixels.1
                                && (screen.scaling() - scaling).abs() < 1e-6
                        });
                    check(
                        "screen",
                        ok,
                        format!(
                            "{} screen(s), a primary one: {}; the screen of the view: {}",
                            all.len(),
                            primary.is_some(),
                            described.unwrap_or_else(|| "none".to_string())
                        ),
                    );
                }
                None => check("screen", false, "the top-level has no screens"),
            }

            // The insets.
            match top_level.insets_manager() {
                Some(insets) => {
                    let padding = insets.safe_area_padding();
                    let edge_to_edge = insets.displays_edge_to_edge();
                    // A window that is displayed edge to edge has the status bar over it.
                    let ok = if edge_to_edge { padding.top > 0.0 } else { padding == Thickness::default() };
                    check(
                        "insets",
                        ok,
                        format!(
                            "displayed edge to edge: {edge_to_edge}; safe area padding {padding:?}; system bars visible: {:?}",
                            insets.is_system_bar_visible()
                        ),
                    );
                }
                None => check("insets", false, "the top-level has no insets manager"),
            }

            // The key and text events of the top-level, for the keys the script injects.
            top_level.add_handler(InputElement::key_down_event(), {
                let keys = state.keys.clone();
                move |_: &Interactive, e: &KeyEventArgs| keys.down.borrow_mut().push((e.key, e.key_symbol.clone()))
            });
            top_level.add_handler(InputElement::key_up_event(), {
                let keys = state.keys.clone();
                move |_: &Interactive, e: &KeyEventArgs| keys.up.borrow_mut().push(e.key)
            });
            top_level.add_handler(InputElement::text_input_event(), {
                let keys = state.keys.clone();
                move |_: &Interactive, e: &TextInputEventArgs| {
                    keys.text.borrow_mut().push_str(e.text.as_deref().unwrap_or_default());
                }
            });

            // The back requests of the top-level, and the activations of the application.
            top_level.add_handler(TopLevel::back_requested_event(), {
                let (handle_back, back_requests) = (state.handle_back.clone(), state.back_requests.clone());
                move |_: &Interactive, e: &RoutedEventArgs| {
                    back_requests.set(back_requests.get() + 1);
                    e.set_handled(handle_back.get());
                }
            });
            if let Some(lifetime) = FerroLocator::current().get_service::<dyn IActivatableLifetime>() {
                // The subscriptions live as long as the application.
                std::mem::forget(lifetime.activated(Rc::new({
                    let lifecycle = state.lifecycle.clone();
                    move |e: &ActivatedEventArgs| {
                        lifecycle.activated.borrow_mut().push(e.kind());
                        if let Some(protocol) = e.as_protocol_activated() {
                            lifecycle.protocols.borrow_mut().push(protocol.uri().original_string().to_string());
                        }
                    }
                })));
                std::mem::forget(lifetime.deactivated(Rc::new({
                    let lifecycle = state.lifecycle.clone();
                    move |e: &ActivatedEventArgs| lifecycle.deactivated.borrow_mut().push(e.kind())
                })));
            }

            // The dispatcher: a timer, and a job from another thread.
            let fired = state.timer_fired.clone();
            let _ = DispatcherTimer::run_once(
                move || fired.set(true),
                Duration::from_millis(50),
                DispatcherPriority::NORMAL,
            );
            let (posted, posted_on_main) = (state.posted.clone(), state.posted_on_main.clone());
            std::thread::Builder::new()
                .name("smoke-poster".to_string())
                .spawn(move || {
                    Dispatcher::ui_thread().post(
                        move || {
                            posted_on_main.store(Dispatcher::ui_thread().check_access(), Ordering::SeqCst);
                            posted.store(true, Ordering::SeqCst);
                        },
                        DispatcherPriority::NORMAL,
                    );
                })
                .expect("a thread can be started");
            enter(state, Stage::Dispatcher);
        }

        fn dispatcher(state: &Rc<State>) {
            let (fired, posted) = (state.timer_fired.get(), state.posted.load(Ordering::SeqCst));
            if !(fired && posted) && elapsed(state) < Duration::from_secs(5) {
                return;
            }
            check("dispatcher timer", fired, "a timer of 50 ms fired on the main looper");
            check(
                "dispatcher signal",
                posted && state.posted_on_main.load(Ordering::SeqCst),
                "a job posted from a thread the virtual machine did not start ran on the main thread",
            );

            aim_readback(state);
            enter(state, Stage::Frames);
        }

        /// Tells the render thread what to read back, for the layout as it is now, and
        /// forgets the frames read so far.
        fn aim_readback(state: &Rc<State>) {
            // What the render thread is asked to read back: the middle of the stripe, the
            // centre of the ellipse, a point of the background below both, and the rectangle
            // of the text.
            let view = &state.view;
            let scaling = state.scaling.get();
            let size = view.root.bounds();
            let to_pixel = |point: Point| ((point.x * scaling) as i32, (point.y * scaling) as i32);
            let ellipse_center = view
                .ellipse
                .translate_point(Point::new(ELLIPSE_SIZE / 2.0, ELLIPSE_SIZE / 2.0), &view.root)
                .unwrap_or_default();
            let text_origin = view.text.translate_point(Point::default(), &view.root).unwrap_or_default();
            let text_bounds = view.text.bounds();
            let stripe = to_pixel(Point::new(size.width / 2.0, STRIPE_HEIGHT / 2.0));
            let ellipse = to_pixel(ellipse_center);
            let background = to_pixel(Point::new(size.width / 2.0, size.height - 40.0));
            let text_at = to_pixel(text_origin);
            {
                let mut readback = view.readback.lock().unwrap_or_else(PoisonError::into_inner);
                readback.frames = 0;
                readback.colors.clear();
                readback.points = vec![
                    ("stripe", stripe.0, stripe.1),
                    ("ellipse", ellipse.0, ellipse.1),
                    ("background", background.0, background.1),
                ];
                readback.text = Some((
                    text_at.0,
                    text_at.1,
                    (text_bounds.width * scaling) as i32,
                    (text_bounds.height * scaling) as i32,
                ));
            }
            state.tap.set((ellipse.0, ellipse.1, ellipse_center));
        }

        /// The frames read back since the readback was aimed, the size of the surface
        /// they were read from, and whether the three points have the colours drawn.
        fn frames_read(state: &Rc<State>) -> (u32, (i32, i32), bool) {
            // Something changes every tick, so that frames keep coming.
            state.view.probe.invalidate_visual();

            let readback = state.view.readback.lock().unwrap_or_else(PoisonError::into_inner);
            let colors = [("stripe", STRIPE), ("ellipse", ELLIPSE), ("background", FILL)].iter().all(|(name, expected)| {
                readback.colors.iter().any(|(read_name, read)| read_name == name && near(*read, *expected))
            });
            (readback.frames, readback.surface, colors)
        }

        fn frames(state: &Rc<State>) {
            // Something changes every tick, so that frames keep coming.
            state.view.probe.invalidate_visual();

            let readback = state.view.readback.lock().unwrap_or_else(PoisonError::into_inner);
            if readback.frames < 3 && readback.error.is_none() && elapsed(state) < Duration::from_secs(15) {
                return;
            }

            check(
                "frames",
                readback.frames >= 3,
                format!(
                    "{} frame(s) drawn and read back on the thread '{}', from a Skia surface of {}x{} px ({})",
                    readback.frames,
                    readback.thread,
                    readback.surface.0,
                    readback.surface.1,
                    if readback.gpu { "GPU: OpenGL ES through EGL" } else { "raster: the buffer of the native window" }
                ),
            );
            check(
                "render thread",
                readback.thread == "Render Thread",
                format!("the frames were drawn on the thread '{}'", readback.thread),
            );
            let software = OPTIONS.with(Cell::get).software;
            check(
                "rendering mode",
                readback.frames == 0 || readback.gpu != software,
                if software {
                    "software was asked for and the frames are raster"
                } else {
                    "the default order gave a GPU surface"
                },
            );
            if let Some(error) = &readback.error {
                check("readback", false, error);
            }
            let color = |name: &str| readback.colors.iter().find(|(n, _)| *n == name).map(|(_, color)| *color);
            for (name, expected) in [("stripe", STRIPE), ("ellipse", ELLIPSE), ("background", FILL)] {
                let read = color(name);
                check(
                    &format!("pixel of the {name}"),
                    read.is_some_and(|read| near(read, expected)),
                    format!("read back {read:?}, drawn {expected:?}"),
                );
            }
            check(
                "text",
                readback.text_pixels > 0,
                format!(
                    "{} nearly white pixel(s) in the rectangle of the line of text {:?}",
                    readback.text_pixels, readback.text
                ),
            );
            drop(readback);

            if second_start() {
                enter(state, Stage::Finish);
                return;
            }

            // The script reads the lines that begin with SCRIPT and does what each says, in
            // order: here a picture of the screen, a tap at the pixel and a swipe from it.
            note("SCRIPT picture smoke");
            if OPTIONS.with(Cell::get).input {
                let (x, y, _) = state.tap.get();
                note(format!("SCRIPT tap {x} {y}"));
                note(format!("SCRIPT swipe {x} {y} {x} {} 400", y + 300));
                enter(state, Stage::Input);
            } else {
                enter(state, Stage::Finish);
            }
        }

        fn input(state: &Rc<State>) {
            let pointers = &state.view.pointers;
            let (pressed, released, moved) =
                (pointers.pressed.borrow().len(), pointers.released.borrow().len(), pointers.moved.get());
            // A tap, then a swipe: two presses, two releases, and moves between the second pair.
            let complete = pressed >= 2 && released >= 2 && moved >= 1;
            if !complete && elapsed(state) < Duration::from_secs(25) {
                return;
            }

            let (_, _, expected) = state.tap.get();
            let first = pointers.pressed.borrow().first().copied();
            check(
                "touch pressed",
                first.is_some_and(|point| (point.x - expected.x).abs() <= 1.5 && (point.y - expected.y).abs() <= 1.5),
                format!(
                    "the tap arrived as a pointer pressed at {first:?}; injected at the logical point {expected:?}"
                ),
            );
            check(
                "touch moved and released",
                complete,
                format!("{pressed} pressed, {moved} moved and {released} released event(s) for a tap and a swipe"),
            );

            // The key A, the text "ferro" (which the shell injects as the keys of a virtual
            // keyboard) and the enter key.
            note("SCRIPT keyevent 29");
            note("SCRIPT text ferro");
            note("SCRIPT keyevent 66");
            enter(state, Stage::Keys);
        }

        fn keys(state: &Rc<State>) {
            let keys = &state.keys;
            let complete = keys.up.borrow().contains(&Key::Enter);
            if !complete && elapsed(state) < Duration::from_secs(40) {
                return;
            }

            let down = keys.down.borrow();
            let up = keys.up.borrow();
            let text = keys.text.borrow();
            check(
                "key down and up",
                down.first() == Some(&(Key::A, Some("a".to_string()))) && up.first() == Some(&Key::A),
                format!(
                    "the key A arrived as key down {:?} and key up {:?}; {} key down and {} key up event(s) in all",
                    down.first(),
                    up.first(),
                    down.len(),
                    up.len()
                ),
            );
            let down_keys: Vec<Key> = down.iter().map(|(key, _)| *key).collect();
            check(
                "key text",
                *text == "aferro" && down_keys == [Key::A, Key::F, Key::E, Key::R, Key::R, Key::O, Key::Enter],
                format!("the text input of the keys is {:?}; the keys that went down: {down_keys:?}", *text),
            );
            check(
                "key without text",
                // A key the shell injects has no scan code, so no physical key, and the line feed
                // it produces is not a key symbol: the enter key of a keyboard has "\r".
                down.last() == Some(&(Key::Enter, None)) && !text.contains(['\r', '\n']),
                format!("the enter key went down as {:?} and raised no text", down.last()),
            );
            drop((down, up, text));

            // The input method: the editor takes the focus, which makes it the client of the
            // input method and asks for the soft keyboard.
            match top_level(state).and_then(|top_level| top_level.input_pane()) {
                Some(input_pane) => {
                    let changes = state.pane_changes.clone();
                    // The subscription lives as long as the application.
                    std::mem::forget(input_pane.state_changed(Rc::new(move |e: &InputPaneStateEventArgs| {
                        changes.borrow_mut().push((
                            e.new_state(),
                            e.end_rect(),
                            e.animation_duration(),
                            e.easing().is_some(),
                        ));
                    })));
                    let focused = state.view.editor.focus();
                    note(format!("The editor took the focus: {focused}"));
                    enter(state, Stage::InputPane);
                }
                None => {
                    check("input pane", false, "the top-level has no input pane");
                    begin_settings(state);
                }
            }
        }

        fn input_pane(state: &Rc<State>) {
            let Some(input_pane) = top_level(state).and_then(|top_level| top_level.input_pane()) else {
                return;
            };
            let rect = input_pane.occluded_rect();
            let open = input_pane.state() == InputPaneState::Open && rect.height > 0.0;
            // The animation of the insets reports the change; it starts after the state.
            let animated = state.pane_changes.borrow().iter().any(|(new_state, ..)| *new_state == InputPaneState::Open);
            if !(open && animated) && elapsed(state) < Duration::from_secs(40) {
                return;
            }
            check(
                "input pane",
                open && animated,
                format!(
                    "with the editor focused the input pane is {:?} and covers {rect:?}; changes reported (state, \
                     end, duration, easing): {:?}",
                    input_pane.state(),
                    state.pane_changes.borrow()
                ),
            );
            if !open {
                state.view.other.focus();
                begin_settings(state);
                return;
            }

            // Two taps on letters of the soft keyboard: in its second and third row of keys
            // (the pane has a strip of suggestions, three rows of letters and a bottom row).
            let scaling = state.scaling.get();
            let at = |x: f64, y: f64| {
                format!(
                    "SCRIPT tap {} {}",
                    ((rect.x + rect.width * x) * scaling).round(),
                    ((rect.y + rect.height * y) * scaling).round()
                )
            };
            note("SCRIPT picture ime");
            note(at(0.45, 0.42));
            note(at(0.65, 0.58));
            enter(state, Stage::InputMethod);
        }

        fn input_method(state: &Rc<State>) {
            let client = &state.view.client;
            let text = client.text();
            let complete = text.encode_utf16().count() >= 2;
            if !complete && elapsed(state) < Duration::from_secs(30) {
                return;
            }
            check(
                "input method",
                complete,
                format!(
                    "two keys of the soft keyboard made the text of the editor {text:?}, selection {:?}, through {} \
                     text input(s) of the input connection",
                    client.selection(),
                    client.inputs.get()
                ),
            );

            // Another element takes the focus: no client, and the soft keyboard goes.
            state.pane_changes.borrow_mut().clear();
            state.view.other.focus();
            enter(state, Stage::InputPaneClosed);
        }

        fn input_pane_closed(state: &Rc<State>) {
            let Some(input_pane) = top_level(state).and_then(|top_level| top_level.input_pane()) else {
                return;
            };
            let closed = input_pane.state() == InputPaneState::Closed && input_pane.occluded_rect().height == 0.0;
            if !closed && elapsed(state) < Duration::from_secs(30) {
                return;
            }
            check(
                "input pane closed",
                closed,
                format!(
                    "without a client the input pane is {:?} and covers {:?}; changes reported: {:?}",
                    input_pane.state(),
                    input_pane.occluded_rect(),
                    state.pane_changes.borrow()
                ),
            );
            services(state);
        }

        /// The services of the top-level that answer at once: the clipboard, the
        /// feedback, the launcher, the storage provider; then the native control host.
        fn services(state: &Rc<State>) {
            match top_level(state).and_then(|top_level| top_level.clipboard()) {
                Some(clipboard) => {
                    let text = "FerroUI \u{17c}\u{f3}\u{142}w \u{1f600}";
                    let set = now(clipboard.set_text_async(Some(text)));
                    let got = now(clipboard.try_get_text_async());
                    check(
                        "clipboard",
                        matches!(set, Some(Ok(()))) && matches!(&got, Some(Ok(Some(got))) if got == text),
                        format!("text set on the clipboard of the system ({set:?}) reads back as {got:?}"),
                    );
                    let cleared = now(clipboard.clear_async());
                    let after = now(clipboard.try_get_text_async());
                    check(
                        "clipboard cleared",
                        matches!(cleared, Some(Ok(()))) && matches!(&after, Some(Ok(None))),
                        format!("cleared ({cleared:?}) the clipboard has the text {after:?}"),
                    );
                }
                None => check("clipboard", false, "the top-level has no clipboard"),
            }

            match feature::<dyn IPlatformFeedback>(state) {
                Some(feedback) => {
                    let hold = feedback.perform(FeedbackAction::hold(), FeedbackType::Haptic);
                    let hold_sound = feedback.perform(FeedbackAction::hold(), FeedbackType::Sound);
                    let click = feedback.perform(FeedbackAction::click(), FeedbackType::Sound);
                    check(
                        "platform feedback",
                        hold && !hold_sound && click,
                        format!(
                            "a haptic hold was performed: {hold}; a hold as a sound: {hold_sound}; a click sound: {click}"
                        ),
                    );
                }
                None => check("platform feedback", false, "the top-level has no platform feedback"),
            }

            match top_level(state) {
                Some(top_level) => {
                    let uri = Uri::absolute("ferroui-smoke-nobody://nothing").expect("a URI");
                    let launched = now(top_level.launcher().launch_uri_async(&uri));
                    check(
                        "launcher",
                        launched == Some(false),
                        format!("a URI no application of the device handles was launched: {launched:?}"),
                    );

                    let provider = top_level.storage_provider();
                    let documents = now(provider.try_get_well_known_folder_async(WellKnownFolder::Documents));
                    let folder = documents.as_ref().and_then(|folder| folder.as_ref().map(|folder| folder.path().original_string().to_string()));
                    check(
                        "storage provider",
                        provider.can_open() && provider.can_save() && provider.can_pick_folder() && folder.is_some(),
                        format!(
                            "the provider can open: {}, save: {}, pick a folder: {}; the documents folder is {folder:?}",
                            provider.can_open(),
                            provider.can_save(),
                            provider.can_pick_folder()
                        ),
                    );
                }
                None => check("launcher", false, "there is no top-level"),
            }

            // The native control host: a default child, attached and shown in a rectangle.
            let host = feature::<dyn INativeControlHostImpl>(state);
            let parent = top_level(state).and_then(|top_level| top_level.platform_impl()).and_then(|i| i.handle());
            match (host, parent) {
                (Some(host), Some(parent)) => {
                    let child = host.create_default_child(parent);
                    let handle: Rc<dyn IPlatformHandle> = child.clone();
                    let compatible = host.is_compatible_with(&*handle);
                    let attachment = host.create_new_attachment(handle);
                    attachment.show_in_bounds(Rect::new(20.0, 30.0, 100.0, 50.0));
                    note(format!("A native control was attached (its handle is compatible with the host: {compatible})"));
                    *state.native_control.borrow_mut() = Some((child, attachment));
                    enter(state, Stage::NativeControl);
                }
                _ => {
                    check("native control host", false, "the top-level has no native control host");
                    begin_settings(state);
                }
            }
        }

        fn native_control(state: &Rc<State>) {
            // A layout pass of the system places the view.
            if elapsed(state) < Duration::from_millis(1500) {
                return;
            }
            let Some((child, attachment)) = state.native_control.borrow_mut().take() else {
                begin_settings(state);
                return;
            };
            let view = child.as_any().downcast_ref::<AndroidViewControlHandle>().and_then(AndroidViewControlHandle::view);
            match view {
                Some(view) => {
                    let int = |name: &str| call_int(&view, name, "()I", &[]);
                    let has_parent = || call_object(&view, "getParent", "()Landroid/view/ViewParent;", &[]).is_some();
                    let scaling = state.scaling.get();
                    let expected =
                        ((20.0 * scaling) as i32, (30.0 * scaling) as i32, (100.0 * scaling) as i32, (50.0 * scaling) as i32);
                    let placed = (int("getLeft"), int("getTop"), int("getWidth"), int("getHeight"));
                    let shown = has_parent() && int("getVisibility") == 0;
                    attachment.hide_with_size(ferroui_base::Size::new(100.0, 50.0));
                    let hidden = int("getVisibility") == 8;
                    attachment.dispose();
                    let removed = !has_parent();
                    check(
                        "native control host",
                        shown && placed == expected && hidden && removed,
                        format!(
                            "a view of the system shown in (20, 30, 100, 50) is a visible child of the view: {shown}, \
                             placed at (left, top, width, height) {placed:?} px, expected {expected:?}; hidden it is \
                             gone: {hidden}; disposed it left the view: {removed}"
                        ),
                    );
                }
                None => check("native control host", false, "the default child is not a view"),
            }
            child.destroy();
            begin_settings(state);
        }

        /// The settings of the platform, and the night mode the script switches.
        fn begin_settings(state: &Rc<State>) {
            match FerroLocator::current().get_service::<dyn IPlatformSettings>() {
                Some(settings) => {
                    let colors = settings.get_color_values();
                    let (tap, double_tap) =
                        (settings.get_tap_size(PointerType::Touch), settings.get_double_tap_size(PointerType::Touch));
                    check(
                        "platform settings",
                        colors.accent_color1().a == 0xff
                            && tap.width > 0.0
                            && double_tap.width > tap.width
                            && !settings.preferred_application_language().is_empty(),
                        format!(
                            "theme {:?}, contrast {:?}, accents {:?} {:?} {:?}; tap size {tap:?}, double tap size \
                             {double_tap:?} within {:?}, hold after {:?}; language {:?}",
                            colors.theme_variant(),
                            colors.contrast_preference(),
                            colors.accent_color1(),
                            colors.accent_color2(),
                            colors.accent_color3(),
                            settings.get_double_tap_time(PointerType::Touch),
                            settings.hold_wait_duration(),
                            settings.preferred_application_language()
                        ),
                    );
                    let changes = state.color_changes.clone();
                    // The subscription lives as long as the application.
                    std::mem::forget(settings.color_values_changed(Rc::new(move |values: &PlatformColorValues| {
                        changes.borrow_mut().push(*values);
                    })));
                    state.first_theme.set(colors.theme_variant());
                    note(format!("SCRIPT night {}", if colors.theme_variant() == PlatformThemeVariant::Dark { "no" } else { "yes" }));
                    enter(state, Stage::Settings);
                }
                None => {
                    check("platform settings", false, "no platform settings are registered");
                    enter(state, Stage::Finish);
                }
            }
        }

        fn settings(state: &Rc<State>) {
            let first = state.first_theme.get();
            let other =
                if first == PlatformThemeVariant::Dark { PlatformThemeVariant::Light } else { PlatformThemeVariant::Dark };
            let wanted = if state.settings_step.get() == 0 { other } else { first };
            let last = state.color_changes.borrow().last().map(PlatformColorValues::theme_variant);
            let arrived = last == Some(wanted);
            // The broadcast of a configuration change is slow on a device that has just booted.
            if !arrived && elapsed(state) < Duration::from_secs(60) {
                return;
            }

            let current = FerroLocator::current()
                .get_service::<dyn IPlatformSettings>()
                .map(|settings| settings.get_color_values().theme_variant());
            if state.settings_step.get() == 0 {
                check(
                    "night mode",
                    arrived && current == Some(wanted),
                    format!(
                        "the system went from {first:?} to {wanted:?}: the settings raised {} change(s), the last to \
                         {last:?}, and now answer {current:?}",
                        state.color_changes.borrow().len()
                    ),
                );
                note(format!("SCRIPT night {}", if first == PlatformThemeVariant::Dark { "yes" } else { "no" }));
                state.settings_step.set(1);
                enter(state, Stage::Settings);
            } else {
                check(
                    "night mode back",
                    arrived && current == Some(wanted),
                    format!("the system went back to {wanted:?}: the last change was to {last:?}, the settings answer {current:?}"),
                );

                // The lifecycle: the home button sends the application to the background.
                state.lifecycle.deactivated.borrow_mut().clear();
                note("SCRIPT home");
                enter(state, Stage::Background);
            }
        }

        fn background(state: &Rc<State>) {
            let deactivated = state.lifecycle.deactivated.borrow().contains(&ActivationKind::Background);
            if !deactivated && elapsed(state) < Duration::from_secs(30) {
                return;
            }
            check(
                "background",
                deactivated,
                format!(
                    "after the home button the application was deactivated: {:?}",
                    state.lifecycle.deactivated.borrow()
                ),
            );
            // The surface of a stopped activity is destroyed; the frames that are read back
            // after the activity is back are drawn to a new one.
            state.lifecycle.activated.borrow_mut().clear();
            aim_readback(state);
            note("SCRIPT resume");
            enter(state, Stage::Foreground);
        }

        fn foreground(state: &Rc<State>) {
            let activated = state.lifecycle.activated.borrow().contains(&ActivationKind::Background);
            let (frames, surface, colors) = frames_read(state);
            if !(activated && frames >= 3 && colors) && elapsed(state) < Duration::from_secs(40) {
                return;
            }
            check(
                "foreground",
                activated && frames >= 3 && colors,
                format!(
                    "back in the foreground the application was activated ({:?}) and {frames} frame(s) were drawn \
                     to the new surface of {}x{} px, with the colours drawn: {colors}",
                    state.lifecycle.activated.borrow(),
                    surface.0,
                    surface.1
                ),
            );

            // Rotation: the surface changes its size and the layout follows.
            note("SCRIPT rotate 1");
            enter(state, Stage::Rotated);
        }

        /// The client size of the top-level and the orientation of its screen.
        fn orientation(state: &Rc<State>) -> (ferroui_base::Size, String) {
            let top_level = top_level(state);
            let size = top_level.as_ref().map(|top_level| top_level.client_size()).unwrap_or_default();
            let orientation = top_level
                .as_ref()
                .and_then(|top_level| top_level.screens())
                .and_then(|screens| top_level.as_ref().and_then(|top_level| screens.screen_from_top_level(top_level)))
                .map(|screen| format!("{:?}", screen.current_orientation()))
                .unwrap_or_default();
            (size, orientation)
        }

        fn rotated(state: &Rc<State>) {
            let (size, orientation) = orientation(state);
            let landscape = size.width > size.height;
            // The readback is aimed once the layout has the new size.
            let root = state.view.root.bounds();
            if landscape && root.width > root.height && !state.aimed.replace(true) {
                aim_readback(state);
            }
            let (frames, surface, colors) = frames_read(state);
            let drawn = state.aimed.get() && frames >= 3 && colors && surface.0 > surface.1;
            if !(landscape && drawn) && elapsed(state) < Duration::from_secs(40) {
                return;
            }
            check(
                "rotation",
                landscape && drawn && orientation.contains("Landscape"),
                format!(
                    "rotated by a quarter the client size is {}x{}, the screen is {orientation}, and {frames} \
                     frame(s) were drawn to a surface of {}x{} px with the colours drawn: {colors}",
                    size.width, size.height, surface.0, surface.1
                ),
            );
            state.aimed.set(false);
            note("SCRIPT rotate 0");
            enter(state, Stage::RotatedBack);
        }

        fn rotated_back(state: &Rc<State>) {
            let (size, orientation) = orientation(state);
            let portrait = size.height > size.width;
            let root = state.view.root.bounds();
            if portrait && root.height > root.width && !state.aimed.replace(true) {
                aim_readback(state);
            }
            let (frames, surface, colors) = frames_read(state);
            let drawn = state.aimed.get() && frames >= 3 && colors && surface.1 > surface.0;
            if !(portrait && drawn) && elapsed(state) < Duration::from_secs(40) {
                return;
            }
            check(
                "rotation back",
                portrait && drawn && orientation.contains("Portrait"),
                format!(
                    "rotated back the client size is {}x{}, the screen is {orientation}, and {frames} frame(s) \
                     were drawn to a surface of {}x{} px with the colours drawn: {colors}",
                    size.width, size.height, surface.0, surface.1
                ),
            );

            // An intent with a URI for the activity that runs: a protocol activation.
            note("SCRIPT view ferroui-smoke://hello/world?answer=42");
            enter(state, Stage::Protocol);
        }

        fn protocol(state: &Rc<State>) {
            let arrived = !state.lifecycle.protocols.borrow().is_empty();
            if !arrived && elapsed(state) < Duration::from_secs(30) {
                return;
            }
            check(
                "protocol activation",
                state.lifecycle.protocols.borrow().first().map(String::as_str)
                    == Some("ferroui-smoke://hello/world?answer=42"),
                format!(
                    "a new intent with a URI activated the application with {:?} (activations: {:?})",
                    state.lifecycle.protocols.borrow(),
                    state.lifecycle.activated.borrow()
                ),
            );

            // The back button, handled by the application.
            state.handle_back.set(true);
            state.back_requests.set(0);
            state.lifecycle.deactivated.borrow_mut().clear();
            note("SCRIPT back");
            enter(state, Stage::BackHandled);
        }

        fn back_handled(state: &Rc<State>) {
            let requested = state.back_requests.get() > 0;
            // After the request, a moment in which a default action of the system would show.
            if !requested && elapsed(state) < Duration::from_secs(30) {
                return;
            }
            if requested && !state.aimed.replace(true) {
                state.since.set(Instant::now());
                return;
            }
            if requested && elapsed(state) < Duration::from_secs(3) {
                return;
            }
            state.aimed.set(false);
            let stayed =
                state.lifecycle.deactivated.borrow().is_empty() && FerroActivity::current_main_activity().is_some();
            check(
                "back handled",
                requested && stayed,
                format!(
                    "the back button raised {} back request(s) of the top-level, which were handled; the activity \
                     stayed in the foreground: {stayed}",
                    state.back_requests.get()
                ),
            );

            // The back button, not handled: the system does its default (the task of a main
            // activity goes to the background).
            state.handle_back.set(false);
            state.back_requests.set(0);
            note("SCRIPT back");
            enter(state, Stage::BackNotHandled);
        }

        fn back_not_handled(state: &Rc<State>) {
            let requested = state.back_requests.get() > 0;
            let left = !state.lifecycle.deactivated.borrow().is_empty()
                || FerroActivity::current_main_activity().is_none();
            if !(requested && left) && elapsed(state) < Duration::from_secs(30) {
                return;
            }
            check(
                "back not handled",
                requested && left,
                format!(
                    "the back button raised {} back request(s), which nobody handled; the default action of the \
                     system followed (deactivated: {:?}, main activity alive: {})",
                    state.back_requests.get(),
                    state.lifecycle.deactivated.borrow(),
                    FerroActivity::current_main_activity().is_some()
                ),
            );
            if FerroActivity::current_main_activity().is_none() {
                // The default action finished the activity: the script starts it again, and
                // the main view that is made then runs the checks of a second start.
                note("SCRIPT resume");
                enter(state, Stage::Done);
                return;
            }
            state.lifecycle.activated.borrow_mut().clear();
            aim_readback(state);
            note("SCRIPT resume");
            enter(state, Stage::BackResumed);
        }

        fn back_resumed(state: &Rc<State>) {
            let activity = FerroActivity::current_main_activity().is_some();
            let (frames, _, colors) = frames_read(state);
            if !(activity && frames >= 3 && colors) && elapsed(state) < Duration::from_secs(40) {
                return;
            }
            note(format!(
                "Back in the foreground after the back button: main activity {activity}, {frames} frame(s), colours {colors}"
            ));
            enter(state, Stage::Finish);
        }

        fn finish(state: &Rc<State>) {
            match FerroActivity::current_main_activity() {
                Some(activity) => {
                    note("Finishing the activity");
                    activity.finish();
                    enter(state, Stage::WaitForDestroy);
                }
                None => {
                    check("finish", false, "there is no main activity to finish");
                    finish_report();
                    enter(state, Stage::Done);
                }
            }
        }

        fn wait_for_destroy(state: &Rc<State>) {
            let destroyed = FerroActivity::current_main_activity().is_none();
            // The close transition of the system comes first; on an emulator it takes seconds.
            if !destroyed && elapsed(state) < Duration::from_secs(40) {
                return;
            }
            check(
                "finish",
                destroyed && top_level(state).is_none(),
                format!(
                    "the activity was destroyed: {destroyed}; the view left its top-level: {}",
                    top_level(state).is_none()
                ),
            );
            if !second_start() && OPTIONS.with(Cell::get).input {
                // The process lives on: the script starts the activity again, and the main
                // view that is made then runs the checks of a second start and the report.
                note("SCRIPT resume");
                enter(state, Stage::Done);
                return;
            }
            finish_report();
            enter(state, Stage::Done);
        }
    }
}
