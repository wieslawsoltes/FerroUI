//! The test support of the ported tests, in place of `TestAppBuilder.cs` of the reference.
//!
//! The reference runs its tests on the headless platform with a real renderer and a global
//! clock that the platform ticks. Here they run in the unit test application of the framework
//! with the services of a styled window: [`start_app`] for the tests that need no composition,
//! [`CompositionApp`] for the tests that read the composition visuals of a shown window (a
//! compositing renderer over the test compositor of the framework, driven one frame per
//! [`CompositionApp::run_jobs`]). Both register a manual global clock ([`MockGlobalClock`]),
//! which a test pulses where the reference lets the platform tick.

use crate::register_types;
use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::data::TemplateBinding;
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{FerroProperty, ObjectType, Ref, Upcast};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use ferroui_controls::testing::{
    CompositorTestServices, MockWindowingPlatform, TestServices, UnitTestApplication, UnitTestApplicationScope,
};
use ferroui_controls::{ContentControl, Control, Window};
use std::cell::Cell;
use std::rc::Rc;

/// The global clock of the test application. It pulses when a test tells it to
/// ([`pulse`](Self::pulse)).
pub(crate) struct MockGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl MockGlobalClock {
    pub(crate) fn new() -> Self {
        Self { subject: LightweightSubject::new(), play_state: Cell::new(PlayState::Run) }
    }

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

/// A started unit test application and its global clock.
pub(crate) struct App {
    pub(crate) clock: Rc<MockGlobalClock>,
    _scope: UnitTestApplicationScope,
}

/// Starts the unit test application of a test that needs no composition (a plain test of the reference).
pub(crate) fn start_app() -> App {
    register_types();
    let clock = Rc::new(MockGlobalClock::new());
    let scope = UnitTestApplication::start(TestServices::styled_window().with_global_clock(clock.clone()));
    App { clock, _scope: scope }
}

/// `Dispatcher.UIThread.RunJobs()`.
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// A unit test application whose windows render through a compositing renderer, so that their
/// visuals have composition visuals.
pub(crate) struct CompositionApp {
    pub(crate) clock: Rc<MockGlobalClock>,
    services: CompositorTestServices,
}

impl CompositionApp {
    /// Starts the application.
    pub(crate) fn start() -> CompositionApp {
        register_types();
        let clock = Rc::new(MockGlobalClock::new());
        let services = CompositorTestServices::start(TestServices::styled_window().with_global_clock(clock.clone()));
        CompositionApp { clock, services }
    }

    /// `new Window { Content = content }`, rendered by the compositor of the test (not shown
    /// yet).
    pub(crate) fn window(&self, content: &Ref<Control>) -> Ref<Window> {
        let window_impl = MockWindowingPlatform::create_window_mock();
        self.services.setup(&window_impl);
        let window = Window::with_impl(window_impl);
        window.set_content(Some(Control::boxed(content.clone())));
        window
    }

    /// `Dispatcher.UIThread.RunJobs()`: runs the dispatcher jobs, renders one frame and runs the
    /// jobs the frame posted.
    pub(crate) fn run_jobs(&self) {
        self.services.run_jobs();
    }
}

/// The element as a control.
pub(crate) fn control<T: ObjectType + Upcast<Control>>(element: &Ref<T>) -> &Control {
    Upcast::<Control>::upcast(&**element)
}

/// The control template of a content control: a content presenter showing its content.
///
/// Adapted: the reference's tests run without a theme and find the content of their content
/// controls among their visual descendants; the test theme of the framework has no template for
/// content controls, so the tests that need one give it this one.
pub(crate) fn content_control_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            presenter.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        bind(ContentPresenter::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        presenter.register_in_name_scope(&**ns).upcast()
    })
}

/// Whether `f` panics: `Assert.Throws<ArgumentOutOfRangeException>` of the reference, whose
/// exception is a panic here.
pub(crate) fn panics(f: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_err()
}
