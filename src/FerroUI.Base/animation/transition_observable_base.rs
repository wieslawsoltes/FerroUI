use crate::animation::easings::Easing;
use crate::reactive::{SingleSubscriberObservableBase, IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// Provides the base for observables implementing transitions: eases the
/// progress and produces the value for it.
pub struct TransitionObservableBase<T: 'static, F: Fn(f64) -> T + 'static> {
    this: Weak<Self>,
    core: SingleSubscriberObservableBase<T>,
    easing: Easing,
    progress: Rc<dyn IObservable<f64>>,
    progress_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    produce_value: F,
}

impl<T: 'static, F: Fn(f64) -> T + 'static> TransitionObservableBase<T, F> {
    /// Creates the observable. `produce_value` produces the value at the
    /// given eased progress.
    pub fn new(progress: Rc<dyn IObservable<f64>>, easing: Easing, produce_value: F) -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            core: SingleSubscriberObservableBase::new(),
            easing,
            progress,
            progress_subscription: RefCell::new(None),
            produce_value,
        })
    }

    /// The progress the transition is driven by.
    pub fn progress(&self) -> &Rc<dyn IObservable<f64>> {
        &self.progress
    }

    fn subscribed(&self) {
        let Some(this) = self.this.upgrade() else { return };
        let subscription = self.progress.subscribe(this);
        // The progress may have completed while subscribing.
        if self.core.has_observer() {
            *self.progress_subscription.borrow_mut() = Some(subscription);
        } else {
            subscription.dispose();
        }
    }

    fn unsubscribed(&self) {
        let subscription = self.progress_subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

impl<T: 'static, F: Fn(f64) -> T + 'static> IObservable<T> for TransitionObservableBase<T, F> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.core.subscribe(observer, || self.subscribed());
        self.this.upgrade().expect("the observable is alive while it is being subscribed to")
    }
}

impl<T: 'static, F: Fn(f64) -> T + 'static> IDisposable for TransitionObservableBase<T, F> {
    fn dispose(&self) {
        self.core.dispose(|| self.unsubscribed());
    }
}

impl<T: 'static, F: Fn(f64) -> T + 'static> IObserver<f64> for TransitionObservableBase<T, F> {
    fn on_completed(&self) {
        self.core.publish_completed(|| self.unsubscribed());
    }

    fn on_error(&self, error: ObservableError) {
        self.core.publish_error(error, || self.unsubscribed());
    }

    fn on_next(&self, value: f64) {
        let progress = self.easing.ease(value);
        self.core.publish_next((self.produce_value)(progress));
    }
}
