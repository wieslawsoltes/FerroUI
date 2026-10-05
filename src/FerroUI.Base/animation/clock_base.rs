use crate::animation::{PlayState, TimeSpan};
use crate::reactive::{IDisposable, IObservable, IObserver, LightweightSubject};
use std::cell::Cell;
use std::rc::Rc;

/// The shared implementation of clocks that are driven by another time
/// source: converts the source's time to the time elapsed since this clock's
/// first pulse, not counting the time spent paused.
///
/// The owning clock forwards its source's ticks to [`pulse`](Self::pulse)
/// and supplies what to do when the clock stops.
pub struct ClockBase {
    observable: LightweightSubject<TimeSpan>,
    previous_time: Cell<Option<TimeSpan>>,
    internal_time: Cell<TimeSpan>,
    play_state: Cell<PlayState>,
}

impl Default for ClockBase {
    fn default() -> Self {
        Self::new()
    }
}

impl ClockBase {
    pub fn new() -> Self {
        Self {
            observable: LightweightSubject::new(),
            previous_time: Cell::new(None),
            internal_time: Cell::new(TimeSpan::ZERO),
            play_state: Cell::new(PlayState::Run),
        }
    }

    /// Whether anything is subscribed to the clock.
    pub fn has_subscriptions(&self) -> bool {
        self.observable.has_observers()
    }

    /// The playback state of the clock.
    #[inline]
    pub fn play_state(&self) -> PlayState {
        self.play_state.get()
    }

    /// Sets the playback state of the clock.
    #[inline]
    pub fn set_play_state(&self, value: PlayState) {
        self.play_state.set(value)
    }

    /// Advances the clock to `system_time` and notifies the subscribers.
    /// `stop` runs afterwards if the clock has been stopped.
    pub fn pulse(&self, system_time: TimeSpan, stop: impl FnOnce()) {
        match self.previous_time.get() {
            None => {
                self.previous_time.set(Some(system_time));
                self.internal_time.set(TimeSpan::ZERO);
            }
            Some(previous_time) => {
                if self.play_state.get() == PlayState::Pause {
                    self.previous_time.set(Some(system_time));
                    return;
                }
                let delta = system_time - previous_time;
                self.internal_time.set(self.internal_time.get() + delta);
                self.previous_time.set(Some(system_time));
            }
        }

        self.observable.on_next(self.internal_time.get());

        if self.play_state.get() == PlayState::Stop {
            stop();
        }
    }

    /// Subscribes to the clock's time.
    pub fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.observable.subscribe(observer)
    }
}
