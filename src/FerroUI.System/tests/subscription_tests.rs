use crate::{IDisposable, Subscription};
use std::cell::Cell;
use std::rc::Rc;

/// A handle that counts its disposals.
struct Counted(Cell<u32>);

impl IDisposable for Counted {
    fn dispose(&self) {
        self.0.set(self.0.get() + 1);
    }
}

fn handle() -> (Rc<Counted>, Rc<dyn IDisposable>) {
    let counted = Rc::new(Counted(Cell::new(0)));
    let handle: Rc<dyn IDisposable> = counted.clone();
    (counted, handle)
}

#[test]
fn dropping_the_guard_disposes_once() {
    let (counted, handle) = handle();
    let guard = Subscription::new(handle);
    assert_eq!(counted.0.get(), 0);
    drop(guard);
    assert_eq!(counted.0.get(), 1);
}

#[test]
fn dispose_releases_now_and_once() {
    let (counted, handle) = handle();
    Subscription::from(handle).dispose();
    assert_eq!(counted.0.get(), 1);
}

#[test]
fn a_detached_guard_keeps_the_subscription() {
    let (counted, handle) = handle();
    let handle = Subscription::new(handle).detach();
    assert_eq!(counted.0.get(), 0);
    handle.dispose();
    assert_eq!(counted.0.get(), 1);
}

#[test]
fn a_guard_in_an_option_is_released_when_cleared() {
    let (counted, handle) = handle();
    let mut field: Option<Subscription> = Some(handle.into());
    field = None;
    assert!(field.is_none());
    assert_eq!(counted.0.get(), 1);
}

/// An observable that counts its live subscriptions; disposing a handle twice
/// would count twice, so the counts also check that the guard disposes once.
#[derive(Default)]
struct Counter {
    subscribed: Cell<u32>,
    disposed: Rc<Cell<u32>>,
}

impl crate::IObservable<i32> for Counter {
    fn subscribe(&self, observer: Rc<dyn crate::IObserver<i32>>) -> Rc<dyn IDisposable> {
        self.subscribed.set(self.subscribed.get() + 1);
        observer.on_next(1);
        let disposed = self.disposed.clone();
        Rc::new(DisposeCounter(disposed))
    }
}

struct DisposeCounter(Rc<Cell<u32>>);

impl IDisposable for DisposeCounter {
    fn dispose(&self) {
        self.0.set(self.0.get() + 1);
    }
}

struct Values(Rc<Cell<i32>>);

impl crate::IObserver<i32> for Values {
    fn on_next(&self, value: i32) {
        self.0.set(self.0.get() + value);
    }
}

#[test]
fn subscribe_guard_unsubscribes_once_on_drop() {
    use crate::SubscribeGuard;
    let source = Counter::default();
    let seen = Rc::new(Cell::new(0));
    let guard = source.subscribe_guard(Rc::new(Values(seen.clone())));
    assert_eq!(source.subscribed.get(), 1);
    assert_eq!(seen.get(), 1);
    assert_eq!(source.disposed.get(), 0);
    drop(guard);
    assert_eq!(source.disposed.get(), 1);
}

#[test]
fn subscribe_guard_detached_keeps_the_subscription() {
    use crate::SubscribeGuard;
    let source = Counter::default();
    let handle = source
        .subscribe_guard(Rc::new(Values(Rc::new(Cell::new(0)))))
        .detach();
    drop(handle);
    assert_eq!(source.disposed.get(), 0);
}

#[test]
fn subscribe_guard_works_on_shared_observables() {
    use crate::{IObservable, SubscribeGuard};
    let counter = Rc::new(Counter::default());
    let source: Rc<dyn IObservable<i32>> = counter.clone();
    let mut field: Option<crate::Subscription> =
        Some(source.subscribe_guard(Rc::new(Values(Rc::new(Cell::new(0))))));
    assert_eq!(counter.subscribed.get(), 1);
    // A dyn observable behind a reference subscribes the same way.
    let by_ref: &dyn IObservable<i32> = &*source;
    by_ref
        .subscribe_guard(Rc::new(Values(Rc::new(Cell::new(0)))))
        .dispose();
    assert_eq!(counter.disposed.get(), 1);
    drop(field.take());
    assert!(field.is_none());
    assert_eq!(counter.disposed.get(), 2);
}
