//! `WhenAnyMixinTests` (`ReactiveUI.Tests/WhenAny/WhenAnyMixinTests.Arity11.cs`): the overloads of
//! `WhenAny` and `WhenAnyValue` for 11 properties.
//!
//! Upstream's overloads by arity are one method taking a tuple of paths here.

use crate::mocks::{collect_items, WhenAnyArityTestViewModel};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::prop;

#[test]
fn when_any_11_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any(
            (
                prop!(WhenAnyArityTestViewModel => property1),
                prop!(WhenAnyArityTestViewModel => property2),
                prop!(WhenAnyArityTestViewModel => property3),
                prop!(WhenAnyArityTestViewModel => property4),
                prop!(WhenAnyArityTestViewModel => property5),
                prop!(WhenAnyArityTestViewModel => property6),
                prop!(WhenAnyArityTestViewModel => property7),
                prop!(WhenAnyArityTestViewModel => property8),
                prop!(WhenAnyArityTestViewModel => property9),
                prop!(WhenAnyArityTestViewModel => property10),
                prop!(WhenAnyArityTestViewModel => property11),
            ),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}

#[test]
fn when_any_value_11_props_sel() {
    let vm = WhenAnyArityTestViewModel::new();
    let list = collect_items(
        &vm.when_any_value_select(
            (
                prop!(WhenAnyArityTestViewModel => property1),
                prop!(WhenAnyArityTestViewModel => property2),
                prop!(WhenAnyArityTestViewModel => property3),
                prop!(WhenAnyArityTestViewModel => property4),
                prop!(WhenAnyArityTestViewModel => property5),
                prop!(WhenAnyArityTestViewModel => property6),
                prop!(WhenAnyArityTestViewModel => property7),
                prop!(WhenAnyArityTestViewModel => property8),
                prop!(WhenAnyArityTestViewModel => property9),
                prop!(WhenAnyArityTestViewModel => property10),
                prop!(WhenAnyArityTestViewModel => property11),
            ),
            |_| "x".to_owned(),
        )
        .observe_on(ImmediateScheduler::instance()),
    );
    assert!(!list.borrow().is_empty());
}
