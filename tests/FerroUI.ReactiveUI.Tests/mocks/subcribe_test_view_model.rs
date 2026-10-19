//! `SubcribeTestViewModel` (`ReactiveUI.Tests/ReactiveProperties/Mocks/SubcribeTestViewModel.cs`).

use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::ReactiveProperty;
use ferroui_system::{IDisposable, IObservable};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

/// A view model that measures the subscription of many subscribers to a reactive property
/// (`SubcribeTestViewModel`).
///
/// Upstream collects the values of all the subscribers in a static list; the list here belongs to
/// the view model, which the one test that uses it creates once.
pub struct SubcribeTestViewModel {
    /// `_cache`: the subscribers.
    cache: Vec<BasicViewModel>,
    /// `_disposedValue`.
    disposed: Cell<bool>,
    /// `Property`.
    pub property: ReactiveProperty<i32>,
    /// `StartupTime`: the milliseconds the subscribers took to subscribe.
    pub startup_time: u128,
    /// `SubscriberCount`.
    pub subscriber_count: usize,
    /// `SubscriberEvents`: the values the subscribers received while subscribing.
    pub subscriber_events: usize,
}

impl SubcribeTestViewModel {
    /// A view model with `count` subscribers (`SubcribeTestViewModel(int)`).
    pub fn new(count: usize) -> Self {
        let property = ReactiveProperty::with_scheduler(1, Some(ImmediateScheduler::instance()), false, false);
        let items = Rc::new(RefCell::new(Vec::new()));
        let stopwatch = Instant::now();

        let cache: Vec<BasicViewModel> =
            (0..count).map(|_| BasicViewModel::new(Rc::new(property.clone()), items.clone())).collect();

        let startup_time = stopwatch.elapsed().as_millis();
        let subscriber_count = cache.len();
        let subscriber_events = items.borrow().len();
        Self { cache, disposed: Cell::new(false), property, startup_time, subscriber_count, subscriber_events }
    }
}

impl IDisposable for SubcribeTestViewModel {
    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        for item in &self.cache {
            item.dispose();
        }
    }
}

/// A subscriber that records the values of an observable (`BasicViewModel`).
struct BasicViewModel {
    /// `_subscription`.
    subscription: Rc<dyn IDisposable>,
    /// `_disposedValue`.
    disposed: Cell<bool>,
}

impl BasicViewModel {
    fn new(observable: Rc<dyn IObservable<i32>>, items: Rc<RefCell<Vec<i32>>>) -> Self {
        let subscription = observable.subscribe_next(move |value| items.borrow_mut().push(value));
        Self { subscription, disposed: Cell::new(false) }
    }

    fn dispose(&self) {
        if self.disposed.replace(true) {
            return;
        }
        self.subscription.dispose();
    }
}
