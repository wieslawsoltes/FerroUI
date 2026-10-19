//! `WhenAnyObservableTests` (`ReactiveUI.Binding.GeneratedCode.Tests/WhenAnyObservable/WhenAnyObservableTests.cs`).

use crate::mocks::collect_items;
use crate::tests::binding::generated_code::test_models::scenarios::when_any_observable_scenarios;
use crate::tests::binding::generated_code::test_models::ObservablePropertyViewModel;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_system::{IObservable, IObserver};
use std::cell::RefCell;
use std::rc::Rc;

/// The value emitted from the first observable.
const FROM1: &str = "From1";

/// The value emitted from the second observable.
const FROM2: &str = "From2";

/// A subject as the observable a property holds.
fn held(subject: &Rc<Subject<String>>) -> Option<Rc<dyn IObservable<String>>> {
    Some(subject.clone() as Rc<dyn IObservable<String>>)
}

#[test]
fn single_observable_switch_emits_inner_values() {
    let subject = Subject::<String>::new();
    let vm = ObservablePropertyViewModel::new();
    vm.set_my_command(held(&subject));

    let values = collect_items(&when_any_observable_scenarios::single_observable_switch(&vm));

    subject.on_next("Hello".to_string());

    assert!(values.borrow().contains(&"Hello".to_string()));
}

#[test]
fn single_observable_switch_resubscribes_on_new_observable() {
    let subject1 = Subject::<String>::new();
    let vm = ObservablePropertyViewModel::new();
    vm.set_my_command(held(&subject1));

    let values = collect_items(&when_any_observable_scenarios::single_observable_switch(&vm));

    subject1.on_next(FROM1.to_string());

    assert!(values.borrow().contains(&FROM1.to_string()));

    // Replace with new observable
    let subject2 = Subject::<String>::new();
    vm.set_my_command(held(&subject2));

    subject2.on_next(FROM2.to_string());

    assert!(values.borrow().contains(&FROM2.to_string()));

    // Old observable should no longer propagate
    subject1.on_next("StaleFrom1".to_string());

    assert!(!values.borrow().contains(&"StaleFrom1".to_string()));
}

#[test]
fn two_observables_merge_emits_both_streams() {
    let subject1 = Subject::<String>::new();
    let subject2 = Subject::<String>::new();
    let vm = ObservablePropertyViewModel::new();
    vm.set_my_command(held(&subject1));
    vm.set_other_command(held(&subject2));

    let values = collect_items(&when_any_observable_scenarios::two_observables_merge(&vm));

    subject1.on_next(FROM1.to_string());
    subject2.on_next(FROM2.to_string());

    assert!(values.borrow().contains(&FROM1.to_string()));
    assert!(values.borrow().contains(&FROM2.to_string()));
}

#[test]
fn disposal_stops_listening() {
    let subject = Subject::<String>::new();
    let vm = ObservablePropertyViewModel::new();
    vm.set_my_command(held(&subject));
    let values = Rc::new(RefCell::new(Vec::<String>::new()));

    let sink = values.clone();
    let subscription = when_any_observable_scenarios::single_observable_switch(&vm).subscribe_next(move |value| sink.borrow_mut().push(value));

    subject.on_next("Before".to_string());

    assert!(values.borrow().contains(&"Before".to_string()));

    subscription.dispose();

    subject.on_next("After".to_string());

    assert!(!values.borrow().contains(&"After".to_string()));
}
