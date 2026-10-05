use crate::animation::{ClockBase, IClock, IGlobalClock, PlayState, TimeSpan};
use crate::reactive::{IDisposable, IObservable, IObserver, ObservableError};
use crate::{FerroLocator, LocatorExtensions};
use std::cell::RefCell;
use std::rc::Rc;

/// A clock that runs on another clock: its time starts at zero on the first
/// tick of the parent after it was created, and stands still while it is
/// paused.
///
/// The clock subscribes to its parent when it is created, whether or not
/// anything is subscribed to it, and unsubscribes on the first tick after
/// its play state was set to [`PlayState::Stop`].
pub struct Clock {
    base: ClockBase,
    parent_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

struct ParentObserver(Rc<Clock>);

impl IObserver<TimeSpan> for ParentObserver {
    fn on_next(&self, value: TimeSpan) {
        let clock = &self.0;
        clock.base.pulse(value, || clock.stop());
    }

    fn on_error(&self, _error: ObservableError) {}

    fn on_completed(&self) {}
}

impl Clock {
    /// The global clock: the `dyn IGlobalClock` service.
    ///
    /// Panics when no global clock is registered.
    pub fn global_clock() -> Rc<dyn IClock> {
        FerroLocator::current().get_required_service::<dyn IGlobalClock>()
    }

    /// Creates a clock that runs on the global clock.
    pub fn new() -> Rc<Clock> {
        Self::with_parent(&Self::global_clock())
    }

    /// Creates a clock that runs on `parent`.
    pub fn with_parent(parent: &Rc<dyn IClock>) -> Rc<Clock> {
        let clock = Rc::new(Clock { base: ClockBase::new(), parent_subscription: RefCell::new(None) });
        let subscription = parent.subscribe(Rc::new(ParentObserver(clock.clone())));
        *clock.parent_subscription.borrow_mut() = Some(subscription);
        clock
    }

    fn stop(&self) {
        let subscription = self.parent_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

impl IObservable<TimeSpan> for Clock {
    fn subscribe(&self, observer: Rc<dyn IObserver<TimeSpan>>) -> Rc<dyn IDisposable> {
        self.base.subscribe(observer)
    }
}

impl IClock for Clock {
    fn play_state(&self) -> PlayState {
        self.base.play_state()
    }

    fn set_play_state(&self, value: PlayState) {
        self.base.set_play_state(value)
    }
}
