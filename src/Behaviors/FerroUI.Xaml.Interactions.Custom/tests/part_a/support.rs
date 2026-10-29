//! The test application of the tests of part A (`TestAppBuilder.cs`, the application document
//! and `HeadlessWindowExtensions.cs` of the reference test project).
//!
//! The reference runs its tests on the headless platform with the Fluent theme; here they run in
//! the unit test application of the framework, with the services of a styled window that moves
//! the keyboard focus, the Fluent theme and a global clock (which the transitions of the Fluent
//! control themes need). The windows of the tests are documents loaded with the run-time loader.

use crate::ASSEMBLY;
use ferroui_base::controls::NameScope;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{ObjectType, Ref, StyledElement};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Control;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::Cell;
use std::rc::Rc;

/// Starts the application of a test (the headless test attribute of the reference).
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    // The documents of the tests use the actions of the interactions crate.
    ferroui_xaml_interactions::register_types();
    let scope = UnitTestApplication::start(fluent_services());
    FerroRuntimeXamlLoader::register();
    scope
}

/// `Dispatcher.UIThread.RunJobs()`.
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

/// Loads a document of the tests with the run-time loader and returns its root object.
pub(crate) fn load<T: ObjectType>(name: &str, xaml: &str) -> Ref<T> {
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| match error.inner_exception() {
            Some(inner) => panic!("{name} failed to load: {}\n ---> {inner}", error.message()),
            None => panic!("{name} failed to load: {}", error.message()),
        });
    from_markup_value::<Ref<T>>(&Some(loaded)).unwrap_or_else(|| panic!("{name} is not a {}", T::TYPE.name()))
}

/// A named element of the name scope of `root` (the fields the compiler of the reference generates
/// for the named elements of a window).
pub(crate) fn find<T: ObjectType>(root: &StyledElement, name: &str) -> Ref<T> {
    NameScope::get_name_scope(root)
        .and_then(|scope| scope.find(name))
        .and_then(|object| object.cast::<T>())
        .unwrap_or_else(|| panic!("the named element {name}"))
}

/// The global clock of the themed application: the transitions of the Fluent control themes need
/// one. It never pulses.
struct MockGlobalClock {
    subject: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::TimeSpan>,
    play_state: Cell<ferroui_base::animation::PlayState>,
}

impl ferroui_base::reactive::IObservable<ferroui_base::animation::TimeSpan> for MockGlobalClock {
    fn subscribe(
        &self,
        observer: Rc<dyn ferroui_base::reactive::IObserver<ferroui_base::animation::TimeSpan>>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl ferroui_base::animation::IClock for MockGlobalClock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        self.play_state.get()
    }

    fn set_play_state(&self, value: ferroui_base::animation::PlayState) {
        self.play_state.set(value)
    }
}

impl ferroui_base::animation::IGlobalClock for MockGlobalClock {}

/// The services of the reference's test application: the Fluent theme and a global clock, with
/// the keyboard services that move the focus.
fn fluent_services() -> TestServices {
    let clock = Rc::new(MockGlobalClock {
        subject: ferroui_base::reactive::LightweightSubject::new(),
        play_state: Cell::new(ferroui_base::animation::PlayState::Run),
    });
    let focus = TestServices::focusable_window();
    let mut services = TestServices::styled_window()
        .with_global_clock(clock)
        .with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style());
    services.keyboard_device = focus.keyboard_device;
    services.keyboard_navigation = focus.keyboard_navigation;
    services.input_manager = focus.input_manager;
    services
}
