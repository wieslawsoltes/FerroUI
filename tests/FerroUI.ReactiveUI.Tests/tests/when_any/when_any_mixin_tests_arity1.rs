//! `WhenAnyMixinTests` (`ReactiveUI.Tests/WhenAny/WhenAnyMixinTests.Arity1.cs`): the overloads of
//! `WhenAny` and `WhenAnyValue` for 1 property.
//!
//! Upstream's overloads by arity are one method taking a tuple of paths here.

use crate::mocks::{collect_items, WhenAnyArityTestViewModel};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;

#[test]
fn when_any_1_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any(
            prop!(WhenAnyArityTestViewModel => property1),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}

#[test]
fn when_any_value_1_prop_expr() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(&vm.when_any_value(prop!(WhenAnyArityTestViewModel => property1)).observe_on(ImmediateScheduler::instance()));
    assert!(!list.borrow().is_empty());
    vm.set_property1(Some("a".to_owned()));
    assert!(list.borrow().len() > 1);
}

#[test]
fn when_any_value_1_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any_value_select(
            prop!(WhenAnyArityTestViewModel => property1),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}
