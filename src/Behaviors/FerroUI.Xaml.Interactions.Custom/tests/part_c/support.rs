//! The test application of the tests of part C (`TestAppBuilder.cs`, the application document
//! and `Core/Command.cs` of the reference test project).
//!
//! The reference runs its tests on the headless platform with the Fluent theme, and the platform
//! ticks the global clock. Here the tests run in the unit test application of the framework with
//! the services of a styled window, the Fluent theme and a manual global clock
//! ([`MockGlobalClock`]), which a test pulses where the platform of the reference ticks. The tests
//! that read the composition visuals of a shown window start a [`CompositionApp`] (a compositing
//! renderer over the test compositor of the framework, driven one frame per
//! [`CompositionApp::run_jobs`]), as the tests of the animations crate do.

use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::input::ICommand;
use ferroui_base::reactive::{Disposable, IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, FerroObject, ObjectType, Ref, Upcast};
use ferroui_controls::testing::{
    CompositorTestServices, MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{Control, Window};
use ferroui_xaml_interactivity::Interaction;
use std::cell::Cell;
use std::rc::Rc;

/// The global clock of the themed application: the transitions of the Fluent control themes and
/// the animations of the behaviors need one. It pulses when a test tells it to.
pub(crate) struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    /// Ticks the clock: its subscribers are told that the time is `time`.
    pub(crate) fn pulse(&self, time: TimeSpan) {
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

/// The services of the reference's test application: the Fluent theme and a global clock.
fn fluent_services(clock: Rc<MockGlobalClock>) -> TestServices {
    TestServices::styled_window()
        .with_global_clock(clock)
        .with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style())
}

/// A started test application and its global clock.
pub(crate) struct App {
    pub(crate) clock: Rc<MockGlobalClock>,
    _scope: UnitTestApplicationScope,
}

/// Starts the application of a test (the headless test attribute of the reference).
pub(crate) fn start_app() -> App {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    let clock = Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
    let scope = UnitTestApplication::start(fluent_services(clock.clone()));
    App { clock, _scope: scope }
}

/// A test application whose windows render through a compositing renderer, so that their visuals
/// have composition visuals.
pub(crate) struct CompositionApp {
    #[allow(dead_code)]
    pub(crate) clock: Rc<MockGlobalClock>,
    services: CompositorTestServices,
}

impl CompositionApp {
    /// Starts the application (the headless test attribute of the reference).
    pub(crate) fn start() -> CompositionApp {
        Control::reset_loaded_queue_for_unit_tests();
        crate::register_types();
        let clock =
            Rc::new(MockGlobalClock { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) });
        let services = CompositorTestServices::start(fluent_services(clock.clone()));
        CompositionApp { clock, services }
    }

    /// `new Window { Content = content }`, rendered by the compositor of the test (not shown yet).
    pub(crate) fn window_with<T: ObjectType + Upcast<Control>>(&self, content: &Ref<T>) -> Ref<Window> {
        let window_impl = MockWindowingPlatform::create_window_mock();
        self.services.setup(&window_impl);
        let window = Window::with_impl(window_impl);
        let content: Ref<Control> = content.clone().upcast();
        window.set_content(Some(Control::boxed(content)));
        window
    }

    /// `Dispatcher.UIThread.RunJobs()`: runs the dispatcher jobs, renders one frame and runs the
    /// jobs the frame posted.
    pub(crate) fn run_jobs(&self) {
        self.services.run_jobs();
    }
}

/// `Dispatcher.UIThread.RunJobs()`.
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// `new Window { Content = content }`.
pub(crate) fn window_with<T: ObjectType + Upcast<Control>>(content: &Ref<T>) -> Ref<Window> {
    let content: Ref<Control> = content.clone().upcast();
    let window = Window::new();
    window.set_content(Some(Control::boxed(content)));
    window
}

/// The object as the root class.
pub(crate) fn object<T: ObjectType + Upcast<FerroObject>>(value: &Ref<T>) -> Ref<FerroObject> {
    value.clone().upcast()
}

/// The object as an untyped value (an `object` argument of the reference).
pub(crate) fn boxed<T: ObjectType + Upcast<FerroObject>>(value: &Ref<T>) -> BoxedValue {
    Rc::new(object(value))
}

/// Adds `behavior` to the behaviors of `host` (`Interaction.GetBehaviors(host).Add(behavior)`).
pub(crate) fn add_behavior<T: ObjectType + Upcast<FerroObject>>(host: &FerroObject, behavior: &Ref<T>) {
    Interaction::get_behaviors(host).add(object(behavior));
}

/// The boolean an untyped result holds (`Assert.IsType<bool>(result)`).
pub(crate) fn as_bool(value: Option<BoxedValue>) -> bool {
    value.and_then(|value| value.downcast_ref::<bool>().copied()).expect("a boolean result")
}

/// A command of the tests (`Core/Command.cs` of the reference test project): runs `execute`, and
/// can always execute. It never raises its can-execute change.
pub(crate) struct Command {
    execute: Box<dyn Fn(Option<&BoxedValue>)>,
}

impl Command {
    /// `new Command(execute)`.
    pub(crate) fn new(execute: impl Fn(Option<&BoxedValue>) + 'static) -> Rc<dyn ICommand> {
        Rc::new(Self { execute: Box::new(execute) })
    }
}

impl ICommand for Command {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        true
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        (self.execute)(parameter);
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
