//! The global clock of the tests.

use ferroui_base::animation::{IClock, IGlobalClock, PlayState, TimeSpan};
use ferroui_base::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use std::cell::Cell;
use std::rc::Rc;

/// The global clock of the tests: the animations a page starts subscribe to it; it ticks when
/// the test tells it to ([`pulse`](Self::pulse)).
#[derive(Default)]
pub struct TestGlobalClock {
    subject: LightweightSubject<TimeSpan>,
    play_state: Cell<Option<PlayState>>,
}

impl IObservable<TimeSpan> for TestGlobalClock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.subject.subscribe(observer)
    }
}

impl TestGlobalClock {
    /// Ticks the clock: its subscribers are told that the time is `time`.
    pub fn pulse(&self, time: TimeSpan) {
        self.subject.on_next(time);
    }
}

impl IClock for TestGlobalClock {
    fn play_state(&self) -> PlayState {
        self.play_state.get().unwrap_or(PlayState::Run)
    }

    fn set_play_state(&self, value: PlayState) {
        self.play_state.set(Some(value))
    }
}

impl IGlobalClock for TestGlobalClock {}
