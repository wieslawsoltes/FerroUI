//! `WhenAnyObservableMixinTests` (`ReactiveUI.Tests/WhenAny/WhenAnyObservableMixinTests.Arity11.cs`):
//! the overloads of `WhenAnyObservable` for 11 observable properties.
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
fn when_any_observable_11_props() {
    let vm = WhenAnyArityTestViewModel::new();
    let subj1 = Subject::<String>::new();
    vm.set_observable_property1(Some(subj1.as_observable()));
    let subj2 = Subject::<String>::new();
    vm.set_observable_property2(Some(subj2.as_observable()));
    let subj3 = Subject::<String>::new();
    vm.set_observable_property3(Some(subj3.as_observable()));
    let subj4 = Subject::<String>::new();
    vm.set_observable_property4(Some(subj4.as_observable()));
    let subj5 = Subject::<String>::new();
    vm.set_observable_property5(Some(subj5.as_observable()));
    let subj6 = Subject::<String>::new();
    vm.set_observable_property6(Some(subj6.as_observable()));
    let subj7 = Subject::<String>::new();
    vm.set_observable_property7(Some(subj7.as_observable()));
    let subj8 = Subject::<String>::new();
    vm.set_observable_property8(Some(subj8.as_observable()));
    let subj9 = Subject::<String>::new();
    vm.set_observable_property9(Some(subj9.as_observable()));
    let subj10 = Subject::<String>::new();
    vm.set_observable_property10(Some(subj10.as_observable()));
    let subj11 = Subject::<String>::new();
    vm.set_observable_property11(Some(subj11.as_observable()));
    let list = collect_items(
        &vm.when_any_observable((
            prop!(WhenAnyArityTestViewModel => observable_property1),
            prop!(WhenAnyArityTestViewModel => observable_property2),
            prop!(WhenAnyArityTestViewModel => observable_property3),
            prop!(WhenAnyArityTestViewModel => observable_property4),
            prop!(WhenAnyArityTestViewModel => observable_property5),
            prop!(WhenAnyArityTestViewModel => observable_property6),
            prop!(WhenAnyArityTestViewModel => observable_property7),
            prop!(WhenAnyArityTestViewModel => observable_property8),
            prop!(WhenAnyArityTestViewModel => observable_property9),
            prop!(WhenAnyArityTestViewModel => observable_property10),
            prop!(WhenAnyArityTestViewModel => observable_property11),
        ))
        .observe_on(ImmediateScheduler::instance()),
    );
    subj1.on_next("test".to_owned());
    subj2.on_next("test".to_owned());
    subj3.on_next("test".to_owned());
    subj4.on_next("test".to_owned());
    subj5.on_next("test".to_owned());
    subj6.on_next("test".to_owned());
    subj7.on_next("test".to_owned());
    subj8.on_next("test".to_owned());
    subj9.on_next("test".to_owned());
    subj10.on_next("test".to_owned());
    subj11.on_next("test".to_owned());
    assert!(!list.borrow().is_empty());
}

#[test]
fn when_any_observable_11_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let subj1 = Subject::<String>::new();
    vm.set_observable_property1(Some(subj1.as_observable()));
    let subj2 = Subject::<String>::new();
    vm.set_observable_property2(Some(subj2.as_observable()));
    let subj3 = Subject::<String>::new();
    vm.set_observable_property3(Some(subj3.as_observable()));
    let subj4 = Subject::<String>::new();
    vm.set_observable_property4(Some(subj4.as_observable()));
    let subj5 = Subject::<String>::new();
    vm.set_observable_property5(Some(subj5.as_observable()));
    let subj6 = Subject::<String>::new();
    vm.set_observable_property6(Some(subj6.as_observable()));
    let subj7 = Subject::<String>::new();
    vm.set_observable_property7(Some(subj7.as_observable()));
    let subj8 = Subject::<String>::new();
    vm.set_observable_property8(Some(subj8.as_observable()));
    let subj9 = Subject::<String>::new();
    vm.set_observable_property9(Some(subj9.as_observable()));
    let subj10 = Subject::<String>::new();
    vm.set_observable_property10(Some(subj10.as_observable()));
    let subj11 = Subject::<String>::new();
    vm.set_observable_property11(Some(subj11.as_observable()));
    let list = collect_items(
        &vm.when_any_observable_select(
            (
                prop!(WhenAnyArityTestViewModel => observable_property1),
                prop!(WhenAnyArityTestViewModel => observable_property2),
                prop!(WhenAnyArityTestViewModel => observable_property3),
                prop!(WhenAnyArityTestViewModel => observable_property4),
                prop!(WhenAnyArityTestViewModel => observable_property5),
                prop!(WhenAnyArityTestViewModel => observable_property6),
                prop!(WhenAnyArityTestViewModel => observable_property7),
                prop!(WhenAnyArityTestViewModel => observable_property8),
                prop!(WhenAnyArityTestViewModel => observable_property9),
                prop!(WhenAnyArityTestViewModel => observable_property10),
                prop!(WhenAnyArityTestViewModel => observable_property11),
            ),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    subj1.on_next("test".to_owned());
    subj2.on_next("test".to_owned());
    subj3.on_next("test".to_owned());
    subj4.on_next("test".to_owned());
    subj5.on_next("test".to_owned());
    subj6.on_next("test".to_owned());
    subj7.on_next("test".to_owned());
    subj8.on_next("test".to_owned());
    subj9.on_next("test".to_owned());
    subj10.on_next("test".to_owned());
    subj11.on_next("test".to_owned());
    assert!(!list.borrow().is_empty());
}
