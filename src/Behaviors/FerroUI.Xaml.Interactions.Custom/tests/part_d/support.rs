//! The test application of part D (`TestAppBuilder.cs` and the application document of the
//! reference test project).
//!
//! The reference runs its tests on the headless platform with the Fluent theme; here they run in
//! the unit test application of the framework, with the services of a styled window, the Fluent
//! theme and a global clock, and documents load with the run-time loader.

use ferroui_base::threading::Dispatcher;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::Control;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::cell::Cell;
use std::rc::Rc;

/// Starts the application of a test (the headless test attribute of the reference), with the
/// Fluent theme, the theme of the reference's test application, and the run-time loader.
pub(crate) fn start_app() -> UnitTestApplicationScope {
    // The queue of the `Loaded` events belongs to the thread; start from an empty one.
    Control::reset_loaded_queue_for_unit_tests();
    crate::register_types();
    let scope = UnitTestApplication::start(fluent_services());
    FerroRuntimeXamlLoader::register();
    scope
}

/// `Dispatcher.UIThread.RunJobs()`.
#[allow(dead_code)]
pub(crate) fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
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

/// The services of the reference's test application: the Fluent theme and a global clock.
fn fluent_services() -> TestServices {
    let clock = Rc::new(MockGlobalClock {
        subject: ferroui_base::reactive::LightweightSubject::new(),
        play_state: Cell::new(ferroui_base::animation::PlayState::Run),
    });
    TestServices::styled_window()
        .with_global_clock(clock)
        .with_theme(|| ferroui_themes_fluent::FluentTheme::new().as_style())
}
