//! `WhenAnyObservableMixinTests` (`ReactiveUI.Tests/WhenAny/WhenAnyObservableMixinTests.Arity1.cs`):
//! the overloads of `WhenAnyObservable` for 1 observable property.
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
fn when_any_observable_1_prop() {
    let vm = WhenAnyArityTestViewModel::new();
    let subj1 = Subject::<String>::new();
    vm.set_observable_property1(Some(subj1.as_observable()));
    let list = collect_items(&vm.when_any_observable(prop!(WhenAnyArityTestViewModel => observable_property1)).observe_on(ImmediateScheduler::instance()));
    subj1.on_next("test".to_owned());
    assert!(!list.borrow().is_empty());
}
