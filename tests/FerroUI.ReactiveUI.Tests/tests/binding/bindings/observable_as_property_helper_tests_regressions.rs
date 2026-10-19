//! `ObservableAsPropertyHelperTests`, its partial file
//! `ObservableAsPropertyHelperTests.Regressions.cs` (`ReactiveUI.Binding.Tests/Bindings`).

use super::observable_as_property_helper_tests::{never_source, recording, SUPPLIED_VALUE};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn null_after_non_null_reads_back_null() {
    let source = Subject::<Option<String>>::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), |_| {}, None);

    source.on_next(Some("first".to_string()));
    source.on_next(None);

    assert_eq!(fixture.value(), None);
    fixture.dispose();
}

#[test]
fn deferred_disposed_before_first_read_returns_initial_value_without_subscribing() {
    let subscriptions = Rc::new(Cell::new(0));
    let counter = subscriptions.clone();
    let source = CountingObservable::new(move || counter.set(counter.get() + 1), None);
    let fixture = ObservableAsPropertyHelper::with_initial_value_deferred(source, |_| {}, SUPPLIED_VALUE, true);

    fixture.dispose();

    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert_eq!(subscriptions.get(), 0);
}

#[test]
fn synchronous_source_reading_value_from_callback_subscribes_once() {
    let subscriptions = Rc::new(Cell::new(0));
    let counter = subscriptions.clone();
    let source = CountingObservable::new(move || counter.set(counter.get() + 1), Some(SUPPLIED_VALUE));
    let slot: Rc<RefCell<Option<ObservableAsPropertyHelper<i32>>>> = Rc::new(RefCell::new(None));
    let read = Rc::new(Cell::new(-1));

    let (callback_slot, callback_read) = (slot.clone(), read.clone());
    let fixture = ObservableAsPropertyHelper::with_initial_value_deferred(
        source,
        move |value| {
            let helper = callback_slot.borrow().clone();
            callback_read.set(helper.map_or(value, |helper| helper.value()));
        },
        0,
        true,
    );
    *slot.borrow_mut() = Some(fixture.clone());
    let value = fixture.value();

    // The first read subscribes, and the value the source emits while being subscribed to is
    // delivered before the read returns.
    assert_eq!(subscriptions.get(), 1);
    assert_eq!(value, SUPPLIED_VALUE);
    assert_eq!(read.get(), SUPPLIED_VALUE);
    fixture.dispose();
    *slot.borrow_mut() = None;
}

#[test]
fn scheduler_initial_value_readable_before_notification_runs() {
    let sequencer = TestScheduler::new();
    let (notified, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value_on(never_source(), on_changed, SUPPLIED_VALUE, Some(sequencer.scheduler()));

    assert_eq!(fixture.value(), SUPPLIED_VALUE);
    assert!(notified.borrow().is_empty());
    fixture.dispose();
}

/// A source that counts its subscriptions and, when given a value, produces it on each
/// subscription (`CountingObservable<TValue>`; upstream's `default` means no value).
struct CountingObservable<T> {
    on_subscribe: Box<dyn Fn()>,
    emit_on_subscribe: Option<T>,
}

impl<T: Clone + 'static> CountingObservable<T> {
    fn new(on_subscribe: impl Fn() + 'static, emit_on_subscribe: Option<T>) -> Rc<dyn IObservable<T>> {
        Rc::new(Self { on_subscribe: Box::new(on_subscribe), emit_on_subscribe })
    }
}

impl<T: Clone + 'static> IObservable<T> for CountingObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        (self.on_subscribe)();
        if let Some(value) = &self.emit_on_subscribe {
            observer.on_next(value.clone());
        }
        Disposable::empty()
    }
}
