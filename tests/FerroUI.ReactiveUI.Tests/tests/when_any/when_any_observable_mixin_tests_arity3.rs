//! `WhenAnyObservableMixinTests` (`ReactiveUI.Tests/WhenAny/WhenAnyObservableMixinTests.Arity3.cs`):
//! the overloads of `WhenAnyObservable` for 3 observable properties.
//!
//! Upstream's overloads by arity are one method taking a tuple of paths here; the overloads with
//! a selector are `when_any_observable_select`.

use crate::mocks::{collect_items, WhenAnyArityTestViewModel};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{prop, IObserver};

#[test]
fn when_any_observable_3_props() {
    let vm = WhenAnyArityTestViewModel::new();
    let subj1 = Subject::<String>::new();
    vm.set_observable_property1(Some(subj1.as_observable()));
    let subj2 = Subject::<String>::new();
    vm.set_observable_property2(Some(subj2.as_observable()));
    let subj3 = Subject::<String>::new();
    vm.set_observable_property3(Some(subj3.as_observable()));
    let list = collect_items(
        &vm.when_any_observable((
            prop!(WhenAnyArityTestViewModel => observable_property1),
            prop!(WhenAnyArityTestViewModel => observable_property2),
            prop!(WhenAnyArityTestViewModel => observable_property3),
        ))
        .observe_on(ImmediateScheduler::instance()),
    );
    subj1.on_next("test".to_owned());
    subj2.on_next("test".to_owned());
    subj3.on_next("test".to_owned());
    assert!(!list.borrow().is_empty());
}

#[test]
fn when_any_observable_3_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let subj1 = Subject::<String>::new();
    vm.set_observable_property1(Some(subj1.as_observable()));
    let subj2 = Subject::<String>::new();
    vm.set_observable_property2(Some(subj2.as_observable()));
    let subj3 = Subject::<String>::new();
    vm.set_observable_property3(Some(subj3.as_observable()));
    let list = collect_items(
        &vm.when_any_observable_select(
            (
                prop!(WhenAnyArityTestViewModel => observable_property1),
                prop!(WhenAnyArityTestViewModel => observable_property2),
                prop!(WhenAnyArityTestViewModel => observable_property3),
            ),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    subj1.on_next("test".to_owned());
    subj2.on_next("test".to_owned());
    subj3.on_next("test".to_owned());
    assert!(!list.borrow().is_empty());
}
