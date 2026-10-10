//! The real application of a sample, headless: its windows render through the compositor of
//! the test into memory, and take the input of a mouse.

use crate::{services, Frame, FrameSurface, Report, TestGlobalClock};
use ferroui_base::animation::TimeSpan;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{
    IInputDevice, IKeyboardDevice, IKeyboardNavigationHandler, KeyboardDevice, KeyboardNavigationHandler, MouseDevice, Pointer,
    PointerType, RawInputModifiers,
};
use ferroui_base::logging::{LogArea, LogEventLevel};
use ferroui_base::platform::surfaces::IPlatformRenderSurface;
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::composition::Compositor;
use ferroui_base::rendering::testing::ManualRenderLoop;
use ferroui_base::styling::IThemeVariantHost;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroLocator, Point, Rect, Ref, Visual};
use ferroui_controls::platform::{IPopupImpl, ITopLevelImpl, IWindowImpl};
use ferroui_controls::presentation_source::IRendererFactory;
use ferroui_controls::testing::{
    CompositorTestServices, MockWindowImpl, MockWindowingPlatform, TestLogSink, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{Application, TopLevel};
use sample_support::Sample;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

/// The frames after an input or a change: what the change started ends within them.
const SETTLE_FRAMES: usize = 8;

/// The time of the global clock between two frames, in milliseconds, unless the test sets
/// another ([`Shell::set_frame_time`]).
const FRAME_TIME_MS: f64 = 100.0;

type Windows = Rc<RefCell<Vec<(Rc<MockWindowImpl>, Arc<FrameSurface>)>>>;

/// The application of a sample under the test services ([`services`]), with the input
/// services an application has. Every window the application creates is a window of the
/// given size that renders through the compositor of the test into a [`FrameSurface`].
///
/// The windows of the test are closed before the shell is dropped.
pub struct Shell {
    windows: Windows,
    clock: Rc<TestGlobalClock>,
    time: Cell<f64>,
    frame_time: Cell<f64>,
    mouse: Rc<MouseDevice>,
    timestamp: Cell<u64>,
    /// What the framework logged at the level of a warning or above.
    log: Rc<RefCell<Vec<String>>>,
    /// The errors the bindings reported.
    binding_reports: Rc<RefCell<Vec<Report>>>,
    _log_sink: Rc<dyn IDisposable>,
    // Declared before the application scope: dropped before it.
    _compositor: Rc<Compositor>,
    render_loop: Arc<ManualRenderLoop>,
    _app: UnitTestApplicationScope,
}

impl Shell {
    /// Starts the application `create_application` creates as the application of the test.
    pub fn start(sample: &'static Sample, width: f64, height: f64, create_application: impl FnOnce() -> Ref<Application>) -> Shell {
        (sample.register_types)();
        let log = Rc::new(RefCell::new(Vec::new()));
        let binding_reports = Rc::new(RefCell::new(Vec::new()));
        let log_sink = {
            let log = log.clone();
            let binding_reports = binding_reports.clone();
            TestLogSink::start(move |level, area, source, template, values| {
                if level >= LogEventLevel::Warning {
                    if area == LogArea::BINDING {
                        binding_reports.borrow_mut().push(Report::of(source, template, values));
                    }
                    let values: Vec<String> = values.iter().map(|value| value.to_string()).collect();
                    log.borrow_mut().push(format!("[{area}] {template} {values:?}"));
                }
            })
        };

        let windows: Windows = Rc::new(RefCell::new(Vec::new()));
        let compositor_of_windows: Rc<RefCell<Option<Rc<Compositor>>>> = Rc::new(RefCell::new(None));
        let windowing_platform = {
            let windows = windows.clone();
            let compositor = compositor_of_windows.clone();
            MockWindowingPlatform::with_window_impl(move || {
                let window_impl = MockWindowingPlatform::create_window_mock_with_size(width, height);
                window_impl.setup_compositor(compositor.borrow().clone());
                let surface = FrameSurface::new();
                window_impl.setup_surfaces(vec![surface.clone() as Arc<dyn IPlatformRenderSurface>]);
                windows.borrow_mut().push((window_impl.clone(), surface));
                // The popups of the window render through the compositor too (a popup the
                // mock creates on its own has none, and a control that opens one could not
                // create its renderer).
                let compositor = compositor.clone();
                let weak_window_impl = Rc::downgrade(&window_impl);
                window_impl.setup_create_popup(move |_| {
                    let parent: Rc<dyn ITopLevelImpl> = weak_window_impl.upgrade()?;
                    let popup = MockWindowingPlatform::create_popup_mock(parent);
                    popup.setup_compositor(compositor.borrow().clone());
                    popup.setup_surfaces(vec![FrameSurface::new() as Arc<dyn IPlatformRenderSurface>]);
                    Some(popup as Rc<dyn IPopupImpl>)
                });
                window_impl as Rc<dyn IWindowImpl>
            })
        };

        let clock = Rc::new(TestGlobalClock::default());
        let keyboard = KeyboardDevice::new();
        // The keyboard device and the keyboard navigation are the ones an application
        // registers, and the application is the host of the theme variant of its windows (the
        // unit test application does not bind it).
        let services = services(clock.clone())
            .with_windowing_platform(windowing_platform)
            .with_input_manager(Rc::new(ferroui_base::input::InputManager::new()))
            .with_keyboard_device({
                let keyboard = keyboard.clone();
                move || Some(keyboard.clone() as Rc<dyn IKeyboardDevice>)
            })
            .with_keyboard_navigation(|| Some(KeyboardNavigationHandler::new() as Rc<dyn IKeyboardNavigationHandler>));
        let app = UnitTestApplication::start_with(services, create_application);
        FerroLocator::current_mutable()
            .bind::<dyn IThemeVariantHost>()
            .to_constant(Application::current().expect("the application").as_theme_variant_host());
        FerroLocator::current_mutable().bind::<dyn IRendererFactory>().to_func(|| None);
        let render_loop = ManualRenderLoop::new();
        let compositor = CompositorTestServices::create_dummy_compositor(Some(render_loop.clone()));
        *compositor_of_windows.borrow_mut() = Some(compositor.clone());

        Shell {
            windows,
            clock,
            time: Cell::new(0.0),
            frame_time: Cell::new(FRAME_TIME_MS),
            mouse: MouseDevice::with_pointer(Pointer::new(0, PointerType::Mouse, true)),
            timestamp: Cell::new(0),
            log,
            binding_reports,
            _log_sink: log_sink,
            _compositor: compositor,
            render_loop,
            _app: app,
        }
    }

    /// The global clock of the application.
    pub fn clock(&self) -> &Rc<TestGlobalClock> {
        &self.clock
    }

    /// The time of the global clock, in milliseconds.
    pub fn time(&self) -> f64 {
        self.time.get()
    }

    /// Sets the time of the global clock between two frames, in milliseconds (100 at the
    /// start).
    pub fn set_frame_time(&self, milliseconds: f64) {
        self.frame_time.set(milliseconds);
    }

    /// One frame: the jobs of the dispatcher (layout, the renderer update and the commit), a
    /// tick of the global clock one frame time on, and a frame of the compositor.
    pub fn frame(&self) {
        Dispatcher::ui_thread().run_jobs(None);
        let time = self.time.get() + self.frame_time.get();
        self.time.set(time);
        self.clock.pulse(TimeSpan::from_milliseconds(time));
        Dispatcher::ui_thread().run_jobs(None);
        self.render_loop.tick();
        Dispatcher::ui_thread().run_jobs(None);
    }

    /// The frames in which what an input or a change started ends.
    pub fn settle(&self) {
        for _ in 0..SETTLE_FRAMES {
            self.frame();
        }
    }

    /// The number of windows the application created.
    pub fn window_count(&self) -> usize {
        self.windows.borrow().len()
    }

    fn window_impl(&self, window: usize) -> Rc<MockWindowImpl> {
        self.windows.borrow().get(window).map(|(window_impl, _)| window_impl.clone()).expect("a window the application created")
    }

    /// The number of frames the compositor drew for the window `window` (in the order the
    /// application created its windows).
    pub fn frames(&self, window: usize) -> u32 {
        self.windows.borrow().get(window).map_or(0, |(_, surface)| surface.frames())
    }

    /// The last frame the compositor drew for the window `window`.
    pub fn last_frame(&self, window: usize) -> Frame {
        let surface = self.windows.borrow().get(window).map(|(_, surface)| surface.clone()).expect("a window the application created");
        surface.last_frame().expect("the compositor drew a frame of the window")
    }

    /// The bounds of a visual in the coordinates of its top-level, as a rectangle of a frame
    /// `(left, top, right, bottom)`.
    pub fn frame_rect_of(top_level: &TopLevel, visual: &Visual) -> (f64, f64, f64, f64) {
        let bounds = Self::bounds_of(top_level, visual);
        (bounds.x, bounds.y, bounds.x + bounds.width, bounds.y + bounds.height)
    }

    /// The bounds of a visual in the coordinates of its top-level.
    pub fn bounds_of(top_level: &TopLevel, visual: &Visual) -> Rect {
        let origin = visual.translate_point(Point::new(0.0, 0.0), top_level).expect("the visual is in the tree of the top-level");
        let size = visual.bounds().size();
        Rect::new(origin.x, origin.y, size.width, size.height)
    }

    fn pointer(&self, window: usize, type_: RawPointerEventType, position: Point, modifiers: RawInputModifiers) {
        self.timestamp.set(self.timestamp.get() + 16);
        let window_impl = self.window_impl(window);
        let input = ITopLevelImpl::input(&*window_impl).expect("the window handles input");
        input(Rc::new(RawPointerEventArgs::new(
            self.mouse.clone() as Rc<dyn IInputDevice>,
            self.timestamp.get(),
            window_impl.input_root().expect("the input root of the window"),
            type_,
            position,
            modifiers,
        )));
    }

    /// Moves the mouse to `position` of the window `window`.
    pub fn pointer_move(&self, window: usize, position: Point) {
        self.pointer(window, RawPointerEventType::Move, position, RawInputModifiers::NONE);
        self.frame();
    }

    /// Presses the left button of the mouse at `position` of the window `window`.
    pub fn pointer_down(&self, window: usize, position: Point) {
        self.pointer(window, RawPointerEventType::LeftButtonDown, position, RawInputModifiers::LEFT_MOUSE_BUTTON);
        self.frame();
    }

    /// Releases the left button of the mouse at `position` of the window `window`.
    pub fn pointer_up(&self, window: usize, position: Point) {
        self.pointer(window, RawPointerEventType::LeftButtonUp, position, RawInputModifiers::NONE);
        self.frame();
    }

    /// Moves the mouse out of the window `window`.
    pub fn pointer_leave(&self, window: usize) {
        self.pointer(window, RawPointerEventType::LeaveWindow, Point::new(-1.0, -1.0), RawInputModifiers::NONE);
        self.frame();
    }

    /// Moves the mouse to `position` of the window `window`, presses and releases the left
    /// button, and runs the frames of what the click started.
    pub fn click(&self, window: usize, position: Point) {
        self.pointer_move(window, position);
        self.pointer_down(window, position);
        self.pointer_up(window, position);
        self.settle();
    }

    /// The errors the bindings reported since the last call (the area `Binding`, from the
    /// level the desktop hosts log at), each with its count.
    pub fn take_binding_reports(&self) -> BTreeMap<Report, usize> {
        let mut counted = BTreeMap::new();
        for report in self.binding_reports.borrow_mut().drain(..) {
            *counted.entry(report).or_insert(0) += 1;
        }
        counted
    }

    /// What the framework logged at the level of a warning or above.
    pub fn log(&self) -> Vec<String> {
        self.log.borrow().clone()
    }
}
