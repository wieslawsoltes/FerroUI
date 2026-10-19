//! `ObservableAsPropertyHelperTests`, its partial file `ObservableAsPropertyHelperTests.Dispose.cs`
//! (`ReactiveUI.Binding.Tests/Bindings`).

use super::observable_as_property_helper_tests::{never_source, recording, SUPPLIED_VALUE};
use crate::tests::binding::test_models::ManualObservable;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactiveui::*;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[test]
fn dispose_stops_following_source_value_keeps_last_delivered_value() {
    const IGNORED_VALUE: i32 = 2;
    let source = Subject::<i32>::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.as_observable(), |_| {}, 0);
    source.on_next(SUPPLIED_VALUE);

    fixture.dispose();
    source.on_next(IGNORED_VALUE);

    assert_eq!(fixture.value(), SUPPLIED_VALUE);
}

#[test]
fn dispose_source_keeps_pushing_anyway_disposed_guard_ignores_later_values() {
    const IGNORED_VALUE: i32 = 2;
    let source = ManualObservable::<i32>::new();
    let (received, on_changed) = recording();
    let fixture = ObservableAsPropertyHelper::with_initial_value(source.clone() as Rc<dyn IObservable<i32>>, on_changed, 0);
    received.borrow_mut().clear();

    source.observer().expect("the helper subscribed").on_next(SUPPLIED_VALUE);
    fixture.dispose();
    source.observer().expect("the helper subscribed").on_next(IGNORED_VALUE);

    assert_eq!(*received.borrow(), vec![SUPPLIED_VALUE]);
    assert_eq!(fixture.value(), SUPPLIED_VALUE);
}

#[test]
fn dispose_called_twice_second_call_is_no_op() {
    let fixture = ObservableAsPropertyHelper::with_initial_value(never_source(), |_| {}, SUPPLIED_VALUE);

    fixture.dispose();
    fixture.dispose();

    assert_eq!(fixture.value(), SUPPLIED_VALUE);
}

#[test]
fn dispose_during_deferred_activation_disposes_the_new_subscription_rather_than_throwing() {
    let source = DisposeDuringSubscribeObservable::new();
    let fixture = ObservableAsPropertyHelper::with_initial_value_deferred(source.clone() as Rc<dyn IObservable<i32>>, |_| {}, 0, true);
    *source.helper.borrow_mut() = Some(fixture.clone());

    let _ = fixture.value();

    assert!(source.subscription_disposed.get());
    *source.helper.borrow_mut() = None;
}

/// A source that disposes the helper while the helper subscribes to it
/// (`DisposeDuringSubscribeObservable`).
struct DisposeDuringSubscribeObservable {
    helper: RefCell<Option<ObservableAsPropertyHelper<i32>>>,
    subscription_disposed: Rc<Cell<bool>>,
}

impl DisposeDuringSubscribeObservable {
    fn new() -> Rc<Self> {
        Rc::new(Self { helper: RefCell::new(None), subscription_disposed: Rc::new(Cell::new(false)) })
    }
}

impl IObservable<i32> for DisposeDuringSubscribeObservable {
    fn subscribe(&self, _observer: Rc<dyn IObserver<i32>>) -> Rc<dyn IDisposable> {
        let helper = self.helper.borrow().clone();
        if let Some(helper) = helper {
            helper.dispose();
        }
        let disposed = self.subscription_disposed.clone();
        Disposable::create(move || disposed.set(true))
    }
}
