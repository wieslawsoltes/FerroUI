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
