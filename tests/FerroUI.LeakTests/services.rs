//! The application of the tests: the services of a styled window of the
//! reference test suite (the mock windowing platform and the Simple theme),
//! and what stands for a collection of the reference.

use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::{
    AccessKeyHandler, IAccessKeyHandler, IKeyboardDevice, InputManager, KeyModifiers, KeyboardDevice, NavigationMethod,
};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_themes_simple::SimpleTheme;
use std::cell::Cell;
use std::rc::Rc;

/// The global clock of a test: it pulses when the test tells it to.
pub struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    pub fn new() -> Rc<MockGlobalClock> {
        Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) })
    }

    /// Tells the subscribers of the clock that the time is `time`.
    pub fn pulse(&self, time: TimeSpan) {
        self.subject.on_next(time);
    }
}

impl IObservable<TimeSpan> for MockGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl IClock for MockGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }
}

impl IGlobalClock for MockGlobalClock {}

/// The styled window services of the reference test suite: the mock
/// windowing platform, the mock render interface and text services, and the
/// Simple theme as the theme of the application.
///
/// The control themes of the Simple theme have transitions, which need a
/// global clock: the services have `clock`. The reference has one only in
/// the transition tests; a clock that is never pulsed changes nothing else.
pub fn styled_window(clock: Rc<MockGlobalClock>) -> TestServices {
    TestServices::styled_window().with_theme(|| SimpleTheme::new().as_style()).with_global_clock(clock)
}

/// Starts the unit test application with the styled window services.
pub fn start_styled_window() -> UnitTestApplicationScope {
    UnitTestApplication::start(styled_window(MockGlobalClock::new()))
}

/// The application of the control tests.
///
/// Ending it (dropping the scope) does what the reference does when a
/// control test ends: the keyboard device lets go of the focused element,
/// the dispatcher runs its jobs, and the application ends.
pub struct ControlTestScope {
    // Dropped after `drop` ran the clean-up.
    _app: UnitTestApplicationScope,
}

impl Drop for ControlTestScope {
    fn drop(&mut self) {
        // No user code while a failed test unwinds.
        if std::thread::panicking() {
            return;
        }

        // The keyboard device holds a reference to the focused element.
        if let Some(keyboard_device) = KeyboardDevice::instance() {
            keyboard_device.set_focused_element(None, NavigationMethod::Unspecified, KeyModifiers::NONE);
        }

        // Empty the dispatcher queue.
        Dispatcher::ui_thread().run_jobs(None);
    }
}

/// Starts the application of the control tests: the styled window services
/// with a keyboard device, an input manager and an access key handler.
pub fn start_control_test() -> ControlTestScope {
    let services = styled_window(MockGlobalClock::new())
        .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>))
        .with_input_manager(Rc::new(InputManager::new()))
        .with_access_key_handler(|| Some(AccessKeyHandler::new() as Rc<dyn IAccessKeyHandler>));
    ControlTestScope { _app: UnitTestApplication::start(services) }
}

/// What the control tests of the reference do to collect: the jobs of the
/// dispatcher down to the priority of the loaded notifications (they hold
/// the controls they will notify), then every job. There is nothing to
/// collect here: an object is freed when its last strong reference goes.
pub fn collect_garbage() {
    let dispatcher = Dispatcher::ui_thread();
    dispatcher.run_jobs(Some(DispatcherPriority::LOADED));
    dispatcher.run_jobs(None);
}

/// What the data context and transition tests of the reference do to
/// collect: the jobs of the dispatcher down to the priority of the loaded
/// notifications.
pub fn collect_garbage_loaded() {
    Dispatcher::ui_thread().run_jobs(Some(DispatcherPriority::LOADED));
}
