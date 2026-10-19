//! `WhenAnyMixinTests` (`ReactiveUI.Tests/WhenAny/WhenAnyMixinTests.Arity4.cs`): the overloads of
//! `WhenAny` and `WhenAnyValue` for 4 properties.
//!
//! Upstream's overloads by arity are one method taking a tuple of paths here.

use crate::mocks::{collect_items, WhenAnyArityTestViewModel};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;

#[test]
fn when_any_4_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any(
            (
                prop!(WhenAnyArityTestViewModel => property1),
                prop!(WhenAnyArityTestViewModel => property2),
                prop!(WhenAnyArityTestViewModel => property3),
                prop!(WhenAnyArityTestViewModel => property4),
            ),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}

#[test]
fn when_any_value_4_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any_value_select(
            (
                prop!(WhenAnyArityTestViewModel => property1),
                prop!(WhenAnyArityTestViewModel => property2),
                prop!(WhenAnyArityTestViewModel => property3),
                prop!(WhenAnyArityTestViewModel => property4),
            ),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}

#[test]
fn when_any_value_4_props_tuple_expr() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any_value((
            prop!(WhenAnyArityTestViewModel => property1),
            prop!(WhenAnyArityTestViewModel => property2),
            prop!(WhenAnyArityTestViewModel => property3),
            prop!(WhenAnyArityTestViewModel => property4),
        ))
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}
