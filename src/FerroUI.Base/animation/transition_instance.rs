use crate::animation::{Clock, IClock, PlayState, TimeSpan};
use crate::reactive::{SingleSubscriberObservableBase, IDisposable, IObservable, IObserver, ObservableError};
use crate::utilities::MathUtilities;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Handles the timing and lifetime of a transition: an observable of the
/// progress of the transition, from 0 to 1.
pub struct TransitionInstance {
    this: Weak<TransitionInstance>,
    core: SingleSubscriberObservableBase<f64>,
    timer_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    delay: TimeSpan,
    duration: TimeSpan,
    base_clock: Rc<dyn IClock>,
    clock: RefCell<Option<Rc<Clock>>>,
}

/// Holds the instance strongly: a running transition stays alive until it
/// completes or is disposed, whether or not its subscription is kept.
struct TimerObserver(Rc<TransitionInstance>);

impl IObserver<TimeSpan> for TimerObserver {
    fn on_next(&self, value: TimeSpan) {
        self.0.timer_tick(value);
    }

    fn on_error(&self, error: ObservableError) {
        self.0.core.publish_error(error, || self.0.unsubscribed());
    }

    fn on_completed(&self) {
        self.0.core.publish_completed(|| self.0.unsubscribed());
    }
}

impl TransitionInstance {
    /// Creates the progress of a transition that runs on `clock`, starts
    /// after `delay` and takes `duration`.
    pub fn new(clock: Rc<dyn IClock>, delay: TimeSpan, duration: TimeSpan) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            core: SingleSubscriberObservableBase::new(),
            timer_subscription: RefCell::new(None),
            delay,
            duration,
            base_clock: clock,
            clock: RefCell::new(None),
        })
    }

    fn timer_tick(&self, t: TimeSpan) {
        // [<------------- normalizedTotalDur ------------------>]
        // [<---- Delay ---->][<---------- Duration ------------>]
        //                   ^- normalizedDelayEnd
        //                    [<----   normalizedInterpVal   --->]

        let normalized_interp_val;

        if t < self.delay {
            normalized_interp_val = 0.0;
        } else if MathUtilities::are_close(self.duration.total_seconds(), 0.0) {
            normalized_interp_val = 1.0;
        } else {
            let normalized_total_dur = self.delay + self.duration;
            let normalized_delay_end = self.delay.total_seconds() / normalized_total_dur.total_seconds();
            let normalized_presentation_time = t.total_seconds() / normalized_total_dur.total_seconds();

            if normalized_presentation_time < normalized_delay_end
                || MathUtilities::are_close(normalized_presentation_time, normalized_delay_end)
            {
                normalized_interp_val = 0.0;
            } else {
                normalized_interp_val =
                    (t.total_seconds() - self.delay.total_seconds()) / self.duration.total_seconds();
            }
        }

        // Clamp interpolation value.
        if normalized_interp_val >= 1.0 || normalized_interp_val < 0.0 {
            self.core.publish_next(1.0);
            self.core.publish_completed(|| self.unsubscribed());
        } else {
            self.core.publish_next(normalized_interp_val);
        }
    }

    fn unsubscribed(&self) {
        let subscription = self.timer_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
        let clock = self.clock.borrow().clone();
        if let Some(clock) = clock {
            clock.set_play_state(PlayState::Stop);
        }
    }

    fn subscribed(&self) {
        let clock = Clock::with_parent(&self.base_clock);
        *self.clock.borrow_mut() = Some(clock.clone());
        let Some(this) = self.this.upgrade() else { return };
        let subscription = clock.subscribe(Rc::new(TimerObserver(this)));
        *self.timer_subscription.borrow_mut() = Some(subscription);
        self.core.publish_next(0.0);
    }
}

impl IObservable<f64> for TransitionInstance {
    fn subscribe(&self, observer: Rc<dyn IObserver<f64>>) -> Rc<dyn IDisposable> {
        self.core.subscribe(observer, || self.subscribed());
        self.this.upgrade().expect("the instance is alive while it is being subscribed to")
    }
}

impl IDisposable for TransitionInstance {
    fn dispose(&self) {
        self.core.dispose(|| self.unsubscribed());
    }
}
