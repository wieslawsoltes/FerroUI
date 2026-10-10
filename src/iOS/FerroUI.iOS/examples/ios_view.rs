//! A view on iOS: an application with the iOS platform, the Skia renderer
//! on Metal and the Simple theme, whose main view shows a few shapes and a
//! line of text.
//!
//! ```text
//! scripts/ios/bundle.sh                         # the application bundle for the simulator
//! scripts/ios/sim-smoke.sh "iPhone 17"          # builds, installs, runs the smoke mode, reports
//! ```
//!
//! With `--smoke` the example checks itself and exits: the size and the
//! scaling of the view against what UIKit says of the view, of its window
//! and of the screen; the safe area against the layout guide of the view;
//! the screens; that frames are presented on the Metal layer; and the
//! pixels of a frame, read back from the texture it was drawn to (the
//! colours of the three shapes where the layout puts them, and that the
//! band of the text is not empty). Every check prints a line, to the
//! standard output and to `Documents/ios_view_smoke.txt` in the data
//! container of the application; the exit code is 0 only when all of them
//! passed.
//!
//! The checks of stage 2 (the module `stage2`) follow: the settings of
//! the platform against the traits and the locale UIKit reports, a change
//! of the traits (the window is given the other user interface style, and
//! the settings have to raise their change), the scroll gesture of the
//! view, the launcher and the feedback of the top-level.
//!
//! What the smoke mode cannot check: input. An application cannot
//! synthesize a touch, a key press or a scroll event for itself without
//! private interfaces, so their translation is covered by the tests of the
//! crate, and their delivery by trying the application by hand.

#[cfg(not(target_os = "ios"))]
fn main() {
    eprintln!("ios_view: this example needs UIKit, which this system does not have");
}

#[cfg(target_os = "ios")]
fn main() {
    ferroui_ios::run_application::<app::App>()
}

#[cfg(target_os = "ios")]
mod app {
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::media::immutable::ImmutableSolidColorBrush;
    use ferroui_base::media::{Color, IBrush};
    use ferroui_base::styling::Styles;
    use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Thickness};
    use ferroui_controls::shapes::Ellipse;
    use ferroui_controls::{
        Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication, Panel, TextBlock,
    };
    use ferroui_themes_simple::SimpleTheme;
    use std::rc::Rc;

    /// The colour the view is filled with.
    pub const FILL: (u8, u8, u8) = (0x33, 0x66, 0x99);
    /// The colour of the square in the middle of the view.
    pub const SQUARE: (u8, u8, u8) = (0xCC, 0x33, 0x33);
    /// The colour of the circle above the bottom edge.
    pub const CIRCLE: (u8, u8, u8) = (0x33, 0xAA, 0x55);
    /// The side of the square, in points.
    pub const SQUARE_SIDE: f64 = 120.0;
    /// The diameter of the circle and its distance from the bottom edge.
    pub const CIRCLE_DIAMETER: f64 = 80.0;
    pub const CIRCLE_BOTTOM: f64 = 150.0;
    /// The distance of the text from the top edge and its font size.
    pub const TEXT_TOP: f64 = 150.0;
    pub const TEXT_SIZE: f64 = 28.0;
    /// The two colours and the side of the marker in the bottom right corner.
    pub const MARKER: [(u8, u8, u8); 2] = [(0xFF, 0xCC, 0x00), (0x00, 0xCC, 0xFF)];
    pub const MARKER_SIDE: f64 = 16.0;

    /// Gives the marker the colour of the given step.
    pub fn set_marker(marker: &Ref<Border>, step: u32) {
        marker.set_background(brush(MARKER[(step % 2) as usize]));
    }

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
            this.set_name(Some("FerroUI iOS view".to_string()));
            this.styles().add(SimpleTheme::new().upcast::<Styles>());
        }

        fn on_framework_initialization_completed(this: &Self) {
            let lifetime = this.application_lifetime();
            if let Some(single_view) =
                lifetime.as_ref().and_then(|lifetime| lifetime.as_single_view_application_lifetime())
            {
                let (main_view, marker) = create_main_view();
                single_view.set_main_view(Some(main_view));

                if std::env::args().any(|arg| arg == "--smoke") {
                    super::smoke::start(marker);
                }
            }

            Self::parent_on_framework_initialization_completed(this);
        }
    }

    fn brush(color: (u8, u8, u8)) -> Option<Rc<dyn IBrush>> {
        Some(Rc::new(ImmutableSolidColorBrush::new(Color::from_rgb(color.0, color.1, color.2))))
    }

    /// The main view, and the marker in its corner: a small square the
    /// smoke mode changes the colour of, so that frames are drawn.
    fn create_main_view() -> (Ref<Control>, Ref<Border>) {
        let background = Border::new();
        background.set_background(brush(FILL));

        let square = Border::new();
        square.set_background(brush(SQUARE));
        square.set_width(SQUARE_SIDE);
        square.set_height(SQUARE_SIDE);
        square.set_horizontal_alignment(HorizontalAlignment::Center);
        square.set_vertical_alignment(VerticalAlignment::Center);

        let circle = Ellipse::new();
        circle.set_fill(brush(CIRCLE));
        circle.set_width(CIRCLE_DIAMETER);
        circle.set_height(CIRCLE_DIAMETER);
        circle.set_horizontal_alignment(HorizontalAlignment::Center);
        circle.set_vertical_alignment(VerticalAlignment::Bottom);
        circle.set_margin(Thickness::new(0.0, 0.0, 0.0, CIRCLE_BOTTOM));

        let text = TextBlock::new();
        text.set_text(Some("FerroUI on iOS"));
        text.set_font_size(TEXT_SIZE);
        text.set_foreground(brush((0xFF, 0xFF, 0xFF)));
        text.set_horizontal_alignment(HorizontalAlignment::Center);
        text.set_vertical_alignment(VerticalAlignment::Top);
        text.set_margin(Thickness::new(0.0, TEXT_TOP, 0.0, 0.0));

        let panel = Panel::new();
        panel.children().add(background);
        panel.children().add(square);
        panel.children().add(circle);
        let marker = Border::new();
        marker.set_background(brush(MARKER[0]));
        marker.set_width(MARKER_SIDE);
        marker.set_height(MARKER_SIDE);
        marker.set_horizontal_alignment(HorizontalAlignment::Right);
        marker.set_vertical_alignment(VerticalAlignment::Bottom);

        panel.children().add(text);
        panel.children().add(marker.clone());
        (panel.upcast(), marker)
    }
}

/// The checks of the smoke mode.
#[cfg(target_os = "ios")]
mod smoke {
    use super::app::{CIRCLE, CIRCLE_BOTTOM, CIRCLE_DIAMETER, FILL, SQUARE, TEXT_SIZE, TEXT_TOP};
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    use ferroui_base::{Rect, Ref, Size, Thickness};
    use ferroui_controls::{Application, Border, TopLevel};
    use ferroui_ios::metal::FrameCapture;
    use ferroui_ios::single_view_lifetime::SingleViewLifetime;
    use ferroui_ios::view_controller::safe_area_padding_of;
    use ferroui_ios::{FerroView, Platform};
    use objc2::rc::Retained;
    use std::cell::Cell;
    use std::io::Write;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Duration;

    /// How often the checks are tried before they count as failed: the
    /// scene, the first layout and the first frames take a moment.
    const ATTEMPTS: u32 = 40;
    const INTERVAL: Duration = Duration::from_millis(500);
    /// The frames that have to be presented.
    const FRAMES: u64 = 3;
    /// How far a channel of a pixel may be from the colour that was drawn.
    const TOLERANCE: i32 = 4;

    struct Check {
        name: &'static str,
        passed: bool,
        detail: String,
    }

    fn check(checks: &mut Vec<Check>, name: &'static str, passed: bool, detail: String) {
        checks.push(Check { name, passed, detail });
    }

    /// Starts the checks: they are tried until all of them pass or the
    /// attempts are used up, and then the process exits.
    pub fn start(marker: Ref<Border>) {
        let attempt = Rc::new(Cell::new(0u32));
        let capture: Arc<Mutex<Option<FrameCapture>>> = Arc::new(Mutex::new(None));
        let capture_requested = Rc::new(Cell::new(false));
        let stage2 = super::stage2::State::new();

        let _timer = DispatcherTimer::run(
            move || {
                attempt.set(attempt.get() + 1);

                // The pixels of a frame are asked for once the view
                // exists; a frame is only drawn when something changed.
                if let Some(view) = view() {
                    if !capture_requested.replace(true) {
                        let capture = capture.clone();
                        view.capture_next_frame(move |frame| {
                            *capture.lock().unwrap_or_else(PoisonError::into_inner) = Some(frame);
                        });
                    }
                }
                // Something has to change for a frame to be drawn.
                super::app::set_marker(&marker, attempt.get());
                trace(attempt.get(), &capture);

                let frame = capture.lock().unwrap_or_else(PoisonError::into_inner).clone();
                let mut checks = run_checks(frame.as_ref());
                if let Some(view) = view() {
                    for (name, passed, detail) in stage2.run(&view) {
                        check(&mut checks, name, passed, detail);
                    }
                }
                let passed = checks.iter().all(|check| check.passed);
                if !passed && attempt.get() < ATTEMPTS {
                    return true;
                }

                let mut lines = vec![format!("Checks, attempt {} of {ATTEMPTS}:", attempt.get())];
                for check in &checks {
                    lines.push(format!(
                        "  [{}] {}: {}",
                        if check.passed { " ok " } else { "FAIL" },
                        check.name,
                        check.detail
                    ));
                }
                lines.push(if passed { "SMOKE PASSED".to_string() } else { "SMOKE FAILED".to_string() });
                report(&lines);

                // `FERROUI_SMOKE_HOLD=<seconds>` keeps the view on the
                // screen after the report, for a screenshot of the
                // simulator.
                let code = if passed { 0 } else { 1 };
                let hold = std::env::var("FERROUI_SMOKE_HOLD").ok().and_then(|hold| hold.parse::<u64>().ok());
                match hold {
                    Some(hold) if hold > 0 => {
                        let _exit = DispatcherTimer::run_once(
                            move || std::process::exit(code),
                            Duration::from_secs(hold),
                            DispatcherPriority::BACKGROUND,
                        );
                        false
                    }
                    _ => std::process::exit(code),
                }
            },
            INTERVAL,
            DispatcherPriority::BACKGROUND,
        );
    }

    /// One line per attempt: what the display link, the render target and
    /// the layer did so far.
    fn trace(attempt: u32, capture: &Arc<Mutex<Option<FrameCapture>>>) {
        let timer = Platform::timer();
        let (ticks, has_tick) = timer.map_or((0, false), |timer| (timer.ticks(), timer.has_tick()));
        let captured = capture.lock().unwrap_or_else(PoisonError::into_inner).is_some();
        match view() {
            Some(view) => println!(
                "trace: attempt {attempt}: display link ticks {ticks} (render loop attached: {has_tick}), frames begun {}, \
                 presented {}, layout {:?}, frame read back: {captured}",
                view.frames_begun(),
                view.frames_presented(),
                view.pending_layout(),
            ),
            None => println!(
                "trace: attempt {attempt}: display link ticks {ticks} (render loop attached: {has_tick}), no view yet"
            ),
        }
    }

    /// Prints the lines and writes them to the data container.
    fn report(lines: &[String]) {
        for line in lines {
            println!("{line}");
        }
        let _ = std::io::stdout().flush();

        if let Some(home) = std::env::var_os("HOME") {
            let directory = std::path::Path::new(&home).join("Documents");
            let _ = std::fs::create_dir_all(&directory);
            let _ = std::fs::write(directory.join("ios_view_smoke.txt"), lines.join("\n") + "\n");
        }
    }

    fn view() -> Option<Retained<FerroView>> {
        let lifetime = Application::current()?.application_lifetime()?;
        lifetime.as_any().downcast_ref::<SingleViewLifetime>()?.view()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    fn run_checks(frame: Option<&FrameCapture>) -> Vec<Check> {
        let mut checks = Vec::new();

        check(
            &mut checks,
            "platform",
            Platform::graphics().is_some() && Platform::timer().is_some(),
            "the platform has its graphics (Metal) and its render timer".to_string(),
        );

        let Some(view) = view() else {
            check(&mut checks, "view", false, "the lifetime has no view: no scene was connected".to_string());
            return checks;
        };
        let top_level: Ref<TopLevel> = view.top_level();
        let bounds = view.bounds();
        let Some(window) = view.window() else {
            check(&mut checks, "view", false, "the view is in no window".to_string());
            return checks;
        };
        let window_bounds = window.bounds();
        check(
            &mut checks,
            "view",
            bounds.size.width > 0.0 && bounds.size.height > 0.0,
            format!(
                "the view is in a window, {}x{} points (the window: {}x{})",
                bounds.size.width, bounds.size.height, window_bounds.size.width, window_bounds.size.height
            ),
        );

        // The size: what the framework lays out is what UIKit gave the
        // view, and the root view of a window fills the window.
        let client_size = top_level.client_size();
        check(
            &mut checks,
            "size",
            close(client_size.width, bounds.size.width)
                && close(client_size.height, bounds.size.height)
                && close(bounds.size.width, window_bounds.size.width)
                && close(bounds.size.height, window_bounds.size.height),
            format!("the top-level is {}x{}", client_size.width, client_size.height),
        );

        // The scaling: the one of the view and of the screen it is on.
        let scaling = top_level.render_scaling();
        let screen_scale = window.screen().scale();
        check(
            &mut checks,
            "scale",
            scaling >= 1.0 && close(scaling, view.contentScaleFactor()) && close(scaling, screen_scale),
            format!(
                "the top-level renders at {scaling} (the view: {}, the screen: {screen_scale})",
                view.contentScaleFactor()
            ),
        );

        // The screens.
        let screens = top_level.screens();
        let all = screens.as_ref().map(|screens| screens.all()).unwrap_or_default();
        let primary = screens.as_ref().and_then(|screens| screens.primary());
        let native_bounds = window.screen().nativeBounds();
        check(
            &mut checks,
            "screen",
            !all.is_empty()
                && primary.as_ref().is_some_and(|primary| {
                    close(primary.scaling(), window.screen().nativeScale())
                        && primary.bounds().width.max(primary.bounds().height)
                            == native_bounds.size.width.max(native_bounds.size.height) as i32
                }),
            match &primary {
                Some(primary) => format!(
                    "{} screen(s); the primary one: {:?} at {}, {:?}",
                    all.len(),
                    primary.bounds(),
                    primary.scaling(),
                    primary.current_orientation()
                ),
                None => format!("{} screen(s) and no primary one", all.len()),
            },
        );

        // The safe area: what the insets manager reports is what the
        // layout guide of the view leaves free, and the insets of the view.
        let frame_size = view.frame().size;
        let guide = view.safeAreaLayoutGuide().layoutFrame();
        let expected = safe_area_padding_of(
            Size::new(frame_size.width, frame_size.height),
            Rect::new(guide.origin.x, guide.origin.y, guide.size.width, guide.size.height),
        );
        let insets = view.safeAreaInsets();
        let padding = top_level.insets_manager().map(|insets_manager| insets_manager.safe_area_padding());
        check(
            &mut checks,
            "safe area",
            padding == Some(expected)
                && expected == Thickness::new(insets.left, insets.top, insets.right, insets.bottom),
            format!("the insets manager reports {padding:?} (the layout guide: {expected:?})"),
        );

        // Frames.
        let frames = view.frames_presented();
        check(&mut checks, "frames", frames >= FRAMES, format!("{frames} frame(s) presented on the Metal layer"));

        // The pixels of a frame.
        match frame {
            None => check(&mut checks, "pixels", false, "no frame was read back yet".to_string()),
            Some(frame) => check_pixels(&mut checks, frame, bounds.size.width, bounds.size.height, scaling),
        }

        checks
    }

    /// The channels of a pixel of a frame in BGRA order as red, green and
    /// blue.
    fn rgb(frame: &FrameCapture, x: f64, y: f64) -> Option<(u8, u8, u8)> {
        let [b, g, r, _] = frame.pixel(x as usize, y as usize)?;
        Some((r, g, b))
    }

    fn is_color(pixel: Option<(u8, u8, u8)>, color: (u8, u8, u8)) -> bool {
        let near = |a: u8, b: u8| (i32::from(a) - i32::from(b)).abs() <= TOLERANCE;
        pixel.is_some_and(|pixel| near(pixel.0, color.0) && near(pixel.1, color.1) && near(pixel.2, color.2))
    }

    fn check_pixels(checks: &mut Vec<Check>, frame: &FrameCapture, width: f64, height: f64, scaling: f64) {
        let expected_width = (width * scaling) as usize;
        let expected_height = (height * scaling) as usize;
        check(
            checks,
            "frame size",
            frame.width == expected_width && frame.height == expected_height,
            format!(
                "the frame is {}x{} pixels, format {} (the view: {expected_width}x{expected_height})",
                frame.width, frame.height, frame.pixel_format
            ),
        );

        let fill = rgb(frame, width * 0.1 * scaling, height * 0.5 * scaling);
        let square = rgb(frame, width * 0.5 * scaling, height * 0.5 * scaling);
        let circle =
            rgb(frame, width * 0.5 * scaling, (height - CIRCLE_BOTTOM - CIRCLE_DIAMETER / 2.0) * scaling);
        check(
            checks,
            "pixels",
            is_color(fill, FILL) && is_color(square, SQUARE) && is_color(circle, CIRCLE),
            format!("the fill is {fill:?}, the square {square:?}, the circle {circle:?} (red, green, blue)"),
        );

        // The text: in the band where the line of text is laid out there
        // are pixels that are not the fill.
        let mut drawn = 0usize;
        let (top, bottom) = ((TEXT_TOP * scaling) as usize, ((TEXT_TOP + TEXT_SIZE * 1.5) * scaling) as usize);
        let (left, right) = (((width / 2.0 - 100.0) * scaling) as usize, ((width / 2.0 + 100.0) * scaling) as usize);
        for y in top..bottom {
            for x in left..right {
                if !is_color(rgb(frame, x as f64, y as f64), FILL) {
                    drawn += 1;
                }
            }
        }
        check(checks, "text", drawn > 50, format!("{drawn} pixels of the text band are not the fill"));
    }
}

/// The checks of stage 2 of the platform (`docs/porting/ios-platform.md`,
/// section 12).
#[cfg(target_os = "ios")]
mod stage2 {
    use ferroui_base::platform::storage::ILauncher;
    use ferroui_base::platform::{IOptionalFeatureProvider, IPlatformSettings, PlatformColorValues, PlatformThemeVariant};
    use ferroui_base::reactive::IDisposable;
    use ferroui_base::utilities::Uri;
    use ferroui_base::{FerroLocator, LocatorExtensions};
    use ferroui_controls::platform::{FeedbackAction, FeedbackType, IPlatformFeedback};
    use ferroui_ios::FerroView;
    use objc2_foundation::NSLocale;
    use objc2_ui_kit::{UIPanGestureRecognizer, UIScrollTypeMask, UITraitEnvironment, UIUserInterfaceStyle};
    use std::cell::{Cell, RefCell};
    use std::future::Future;
    use std::pin::Pin;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};

    type Checks = Vec<(&'static str, bool, String)>;

    struct NoopWaker;

    impl Wake for NoopWaker {
        fn wake(self: Arc<Self>) {}
    }

    /// Polls a future once: the value of one that is ready.
    fn poll_once<T>(future: &mut Pin<Box<dyn Future<Output = T>>>) -> Option<T> {
        let waker = Waker::from(Arc::new(NoopWaker));
        match future.as_mut().poll(&mut Context::from_waker(&waker)) {
            Poll::Ready(value) => Some(value),
            Poll::Pending => None,
        }
    }

    fn is_dark(view: &FerroView) -> bool {
        // SAFETY: a trait of the collection of a view, read on the main
        // thread.
        unsafe { view.traitCollection().userInterfaceStyle() == UIUserInterfaceStyle::Dark }
    }

    fn variant_of(dark: bool) -> PlatformThemeVariant {
        if dark {
            PlatformThemeVariant::Dark
        } else {
            PlatformThemeVariant::Light
        }
    }

    /// What the checks keep between the attempts.
    pub struct State {
        /// The checks that are made once, when the view is in its window.
        once: RefCell<Option<Checks>>,
        /// The style the view had before the window was given the other.
        initial_dark: Cell<bool>,
        color_events: Rc<RefCell<Vec<PlatformColorValues>>>,
        subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    }

    impl State {
        pub fn new() -> Rc<Self> {
            Rc::new(Self {
                once: RefCell::new(None),
                initial_dark: Cell::new(false),
                color_events: Rc::new(RefCell::new(Vec::new())),
                subscription: RefCell::new(None),
            })
        }

        /// The checks of an attempt.
        pub fn run(&self, view: &FerroView) -> Checks {
            let Some(window) = view.window() else {
                return Vec::new();
            };
            let Some(settings) = FerroLocator::current().get_service::<dyn IPlatformSettings>() else {
                return vec![("settings", false, "the platform has no settings".to_string())];
            };

            if self.once.borrow().is_none() {
                let checks = self.run_once(view, &settings);
                *self.once.borrow_mut() = Some(checks);

                // The change of the traits: the window gets the other
                // style, and the settings have to say so.
                let events = self.color_events.clone();
                *self.subscription.borrow_mut() = Some(settings.color_values_changed(Rc::new(
                    move |values: &PlatformColorValues| events.borrow_mut().push(*values),
                )));
                window.setOverrideUserInterfaceStyle(if self.initial_dark.get() {
                    UIUserInterfaceStyle::Light
                } else {
                    UIUserInterfaceStyle::Dark
                });
            }

            let mut checks = self.once.borrow().clone().unwrap_or_default();

            let expected = variant_of(!self.initial_dark.get());
            let events = self.color_events.borrow();
            checks.push((
                "trait change",
                is_dark(view) != self.initial_dark.get()
                    && events.last().is_some_and(|values| values.theme_variant() == expected)
                    && settings.get_color_values().theme_variant() == expected,
                format!(
                    "the window was given the other style: the view is {:?}, the settings raised {} change(s) and say {:?}",
                    variant_of(is_dark(view)),
                    events.len(),
                    settings.get_color_values().theme_variant()
                ),
            ));

            checks
        }

        fn run_once(&self, view: &FerroView, settings: &Rc<dyn IPlatformSettings>) -> Checks {
            let mut checks = Checks::new();

            // The settings: the variant is the style of the traits of
            // the view, the language the first preferred language.
            self.initial_dark.set(is_dark(view));
            let values = settings.get_color_values();
            let language = settings.preferred_application_language();
            let preferred = NSLocale::preferredLanguages().iter().next().map(|language| language.to_string());
            checks.push((
                "settings",
                values.theme_variant() == variant_of(self.initial_dark.get())
                    && preferred.as_deref() == Some(language.as_str()),
                format!(
                    "the theme variant is {:?}, the contrast {:?}, the accent {:?}, the language {language:?} \
                     (UIKit: dark {}, the first preferred language {preferred:?})",
                    values.theme_variant(),
                    values.contrast_preference(),
                    values.accent_color1(),
                    self.initial_dark.get()
                ),
            ));

            // The scroll gesture: a pan gesture that takes no touches and
            // both kinds of scroll events.
            let recognizers = view.gestureRecognizers();
            let pans: Vec<_> = recognizers
                .iter()
                .flat_map(|recognizers| recognizers.iter())
                .filter_map(|recognizer| recognizer.downcast::<UIPanGestureRecognizer>().ok())
                .collect();
            checks.push((
                "scroll gesture",
                pans.len() == 1
                    && pans[0].maximumNumberOfTouches() == 0
                    && pans[0].allowedScrollTypesMask() == UIScrollTypeMask::Discrete | UIScrollTypeMask::Continuous,
                format!(
                    "the view has {} pan gesture recognizer(s); it takes at most {:?} touches",
                    pans.len(),
                    pans.first().map(|pan| pan.maximumNumberOfTouches())
                ),
            ));

            let top_level = view.top_level();
            let platform_impl = top_level.platform_impl();
            let provider = platform_impl.as_ref().map(|platform_impl| -> &dyn IOptionalFeatureProvider { &**platform_impl });

            // The launcher: a URI of a scheme no application has is not
            // launched, and the answer is there at once.
            let launcher = provider.and_then(|provider| provider.try_get::<dyn ILauncher>());
            let launched = launcher.as_ref().and_then(|launcher| {
                let uri = Uri::absolute("ferroui-no-such-scheme://nothing").ok()?;
                poll_once(&mut launcher.launch_uri_async(&uri))
            });
            checks.push((
                "launcher",
                launched == Some(false),
                format!(
                    "the top-level has a launcher: {}; a URI of an unknown scheme was launched: {launched:?}",
                    launcher.is_some()
                ),
            ));

            // The feedback: the sound of a click is played (the haptic
            // engine of a simulator does nothing), holding has no sound.
            let feedback = provider.and_then(|provider| provider.try_get::<dyn IPlatformFeedback>());
            let click = feedback.as_ref().map(|feedback| feedback.perform(FeedbackAction::click(), FeedbackType::Sound));
            let hold = feedback.as_ref().map(|feedback| feedback.perform(FeedbackAction::hold(), FeedbackType::Sound));
            let tap = feedback.as_ref().map(|feedback| feedback.perform(FeedbackAction::hold(), FeedbackType::Haptic));
            checks.push((
                "feedback",
                click == Some(true) && hold == Some(false) && tap == Some(true),
                format!("the sound of a click: {click:?}, of holding: {hold:?}; the tap of holding: {tap:?}"),
            ));

            checks
        }
    }
}
