//! `CallbackPropertyObservableTests`
//! (`ReactiveUI.Binding.Tests/Observables/CallbackPropertyObservableTests.cs`).
//!
//! A getter that throws is a getter that panics; the test catches the panic.

use crate::tests::binding::test_models::RecordingObserver;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::Rc;

const FIRST_VALUE: &str = "first";
const SECOND_VALUE: &str = "second";

fn observe(control: &Rc<NotifyingControl>, distinct: bool) -> CallbackPropertyObservable<Rc<NotifyingControl>, String> {
    CallbackPropertyObservable::new(
        control.clone(),
        |source: &Rc<NotifyingControl>, notify: Rc<dyn Fn()>| NotifyingControl::attach(source, notify),
        |source: &Rc<NotifyingControl>| source.value(),
        distinct,
    )
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

#[test]
fn subscribe_delivers_the_current_value_then_each_notified_change() {
    let control = NotifyingControl::new(FIRST_VALUE);
    let observer = RecordingObserver::<String>::new();
    let subscription = observe(&control, false).subscribe(observer.clone() as Rc<dyn IObserver<String>>);

    control.set_value(SECOND_VALUE);
    control.notify();

    assert_eq!(observer.values(), strings(&[FIRST_VALUE, SECOND_VALUE]));
    subscription.dispose();
}

#[test]
fn subscribe_distinct_drops_an_unchanged_value() {
    let control = NotifyingControl::new(FIRST_VALUE);
    let observer = RecordingObserver::<String>::new();
    let subscription = observe(&control, true).subscribe(observer.clone() as Rc<dyn IObserver<String>>);

    control.notify();
    control.set_value(SECOND_VALUE);
    control.notify();

    assert_eq!(observer.values(), strings(&[FIRST_VALUE, SECOND_VALUE]));
    subscription.dispose();
}

#[test]
fn subscribe_not_distinct_delivers_an_unchanged_value() {
    let control = NotifyingControl::new(FIRST_VALUE);
    let observer = RecordingObserver::<String>::new();
    let subscription = observe(&control, false).subscribe(observer.clone() as Rc<dyn IObserver<String>>);

    control.notify();

    assert_eq!(observer.values(), strings(&[FIRST_VALUE, FIRST_VALUE]));
    subscription.dispose();
}

#[test]
fn dispose_detaches_once_and_stops_delivery() {
    let control = NotifyingControl::new(FIRST_VALUE);
    let observer = RecordingObserver::<String>::new();
    let subscription = observe(&control, false).subscribe(observer.clone() as Rc<dyn IObserver<String>>);
    let callback = control.callback().expect("the callback is attached");

    subscription.dispose();
    subscription.dispose();
    control.set_value(SECOND_VALUE);
    callback();

    assert_eq!(control.detachments.get(), 1);
    assert_eq!(observer.values(), strings(&[FIRST_VALUE]));
}

#[test]
fn subscribe_getter_throws_detaches_and_rethrows() {
    let control = NotifyingControl::new("");
    let observable = CallbackPropertyObservable::<Rc<NotifyingControl>, String>::new(
        control.clone(),
        |source: &Rc<NotifyingControl>, notify: Rc<dyn Fn()>| NotifyingControl::attach(source, notify),
        |_: &Rc<NotifyingControl>| -> String { panic!("read failed") },
        false,
    );

    let outcome = catch_unwind(AssertUnwindSafe(|| observable.subscribe(RecordingObserver::<String>::new() as Rc<dyn IObserver<String>>)));

    assert!(outcome.is_err());
    assert_eq!(control.detachments.get(), 1);
}

// Not ported: Subscribe_NullObserver_Throws - argument validation

// Not ported: Constructor_NullSubscribe_Throws - argument validation

// Not ported: Constructor_NullGetter_Throws - argument validation

/// A control with a change callback of its own, which counts its detachments
/// (`NotifyingControl`).
struct NotifyingControl {
    value: RefCell<String>,
    callback: RefCell<Option<Rc<dyn Fn()>>>,
    detachments: Cell<i32>,
}

impl NotifyingControl {
    fn new(value: &str) -> Rc<Self> {
        Rc::new(Self { value: RefCell::new(value.to_string()), callback: RefCell::new(None), detachments: Cell::new(0) })
    }

    fn value(&self) -> String {
        self.value.borrow().clone()
    }

    fn set_value(&self, value: &str) {
        *self.value.borrow_mut() = value.to_string();
    }

    fn callback(&self) -> Option<Rc<dyn Fn()>> {
        self.callback.borrow().clone()
    }

    /// Attaches `callback`; the returned disposable detaches it (`Attach`).
    fn attach(this: &Rc<Self>, callback: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        *this.callback.borrow_mut() = Some(callback);
        let control = Rc::downgrade(this);
        Disposable::create(move || {
            if let Some(control) = control.upgrade() {
                *control.callback.borrow_mut() = None;
                control.detachments.set(control.detachments.get() + 1);
            }
        })
    }

    /// Runs the attached callback (`Notify`).
    fn notify(&self) {
        if let Some(callback) = self.callback() {
            callback();
        }
    }
}
