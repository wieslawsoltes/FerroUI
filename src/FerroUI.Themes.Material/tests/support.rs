//! Shared application for inherited layout contracts.
use crate::{register_types, MaterialTheme};
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

pub fn start_themed_application() -> UnitTestApplicationScope {
    register_types();
    UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(ferroui_base::input::KeyboardDevice::new()))
            .with_global_clock(Rc::new(MockGlobalClock::new()))
            .with_theme(|| {
                let theme = MaterialTheme::new();
                theme.set_reduced_motion(true);
                theme.as_style()
            }),
    )
}

pub struct MockGlobalClock {
    subject: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::TimeSpan>,
    play_state: std::cell::Cell<ferroui_base::animation::PlayState>,
}

impl MockGlobalClock {
    pub fn new() -> Self {
        Self {
            subject: ferroui_base::reactive::LightweightSubject::new(),
            play_state: std::cell::Cell::new(ferroui_base::animation::PlayState::Run),
        }
    }
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
