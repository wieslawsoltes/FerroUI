//! `WhenAnyObservableTests` (`ReactiveUI.Tests/WhenAny/WhenAnyObservableTests.cs`).
//!
//! The commands of [`TestWhenAnyObsViewModel`] are observed through the observables of their
//! results (`command1_results`, named `Command1` in the paths), since `WhenAnyObservable` takes a
//! property that holds an observable. Upstream awaits each execution; here an execution is
//! subscribed, and it completes before the subscription returns (the commands deliver on the
//! immediate scheduler).

use crate::mocks::TestWhenAnyObsViewModel;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::{prop, IObservable};
use std::cell::RefCell;
use std::rc::Rc;

/// The expected sequence of combined results produced by the `WhenAnyObservable` combining tests.
const EXPECTED_COMBINING_RESULTS: [&str; 3] = ["foo : 1", "foo : 2", "bar : 2"];

/// The path `x => x.Command1`.
fn command1() -> PropertyPath<Rc<TestWhenAnyObsViewModel>, Option<Rc<dyn IObservable<i32>>>> {
    prop!(TestWhenAnyObsViewModel => command1_results as "Command1")
}

/// Executes `command` with `parameter` and waits for the execution to end (`await command.Execute(parameter)`).
fn execute<P: Clone + Default + 'static, R: Clone + 'static>(command: &ReactiveCommand<P, R>, parameter: P) {
    let _ = command.execute(parameter).subscribe_next(|_| {});
}

#[test]
fn null_observables_do_not_cause_exceptions() {
    let fixture = TestWhenAnyObsViewModel::new();
    fixture.set_command1(None);

    subscribe_to_null_low_arity_merge_overloads(&fixture);
    subscribe_to_null_high_arity_merge_overloads(&fixture);
    subscribe_to_null_low_arity_combine_latest_overloads(&fixture);
    subscribe_to_null_high_arity_combine_latest_overloads(&fixture);
}

#[test]
fn when_any_observable_smoke_test_combining() {
    const SECOND_COMMAND_VALUE: i32 = 2;
    const COUNT_AFTER_SECOND_EMISSION: usize = 2;
    const COUNT_AFTER_THIRD_EMISSION: usize = 3;
    let fixture = TestWhenAnyObsViewModel::new();

    let list = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = list.clone();
    let _ = fixture
        .when_any_observable_select(
            (
                prop!(TestWhenAnyObsViewModel => command3_results as "Command3"),
                command1(),
            ),
            |(s, i): (String, i32)| format!("{s} : {i}"),
        )
        .observe_on(ImmediateScheduler::instance())
        .subscribe_next(move |value| sink.borrow_mut().push(value));
    assert!(list.borrow().is_empty());

    let first_command = fixture.command1().expect("Command1 is set");
    execute(&first_command, 1);
    execute(&fixture.command3(), "foo".to_owned());
    assert_eq!(list.borrow().len(), 1);

    execute(&first_command, SECOND_COMMAND_VALUE);
    assert_eq!(list.borrow().len(), COUNT_AFTER_SECOND_EMISSION);

    execute(&fixture.command3(), "bar".to_owned());
    assert_eq!(list.borrow().len(), COUNT_AFTER_THIRD_EMISSION);
    assert!(EXPECTED_COMBINING_RESULTS.iter().zip(list.borrow().iter()).all(|(expected, actual)| *expected == actual.as_str()));
}

#[test]
fn when_any_observable_smoke_test_merging() {
    const SECOND_COMMAND_VALUE: i32 = 2;
    const COUNT_AFTER_SECOND_EMISSION: usize = 2;
    const COUNT_AFTER_THIRD_EMISSION: usize = 3;
    let fixture = TestWhenAnyObsViewModel::new();

    let list = Rc::new(RefCell::new(Vec::<i32>::new()));
    let sink = list.clone();
    let _ = fixture
        .when_any_observable((command1(), prop!(TestWhenAnyObsViewModel => command2_results as "Command2")))
        .observe_on(ImmediateScheduler::instance())
        .subscribe_next(move |value| sink.borrow_mut().push(value));
    assert!(list.borrow().is_empty());

    let first_command = fixture.command1().expect("Command1 is set");
    execute(&first_command, 1);
    assert_eq!(list.borrow().len(), 1);

    execute(&fixture.command2(), SECOND_COMMAND_VALUE);
    assert_eq!(list.borrow().len(), COUNT_AFTER_SECOND_EMISSION);

    execute(&first_command, 1);
    assert_eq!(list.borrow().len(), COUNT_AFTER_THIRD_EMISSION);
    assert!([1, SECOND_COMMAND_VALUE, 1].iter().zip(list.borrow().iter()).all(|(expected, actual)| expected == actual));
}

/// Subscribes to the lower-arity merge overloads of `WhenAnyObservable` against a null observable.
fn subscribe_to_null_low_arity_merge_overloads(fixture: &Rc<TestWhenAnyObsViewModel>) {
    // these are the lower-arity overloads of WhenAnyObservable that perform a Merge
    let _ = fixture.when_any_observable(command1()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
}

/// Subscribes to the higher-arity merge overloads of `WhenAnyObservable` against a null observable.
fn subscribe_to_null_high_arity_merge_overloads(fixture: &Rc<TestWhenAnyObsViewModel>) {
    subscribe_to_null_merge_overloads_arity7_through9(fixture);
    subscribe_to_null_merge_overloads_arity10_through12(fixture);
}

/// Subscribes to the arity 7-9 merge overloads of `WhenAnyObservable` against a null observable.
fn subscribe_to_null_merge_overloads_arity7_through9(fixture: &Rc<TestWhenAnyObsViewModel>) {
    // these are the higher-arity overloads of WhenAnyObservable that perform a Merge
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
}

/// Subscribes to the arity 10-12 merge overloads of `WhenAnyObservable` against a null observable.
fn subscribe_to_null_merge_overloads_arity10_through12(fixture: &Rc<TestWhenAnyObsViewModel>) {
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
    let _ = fixture.when_any_observable((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1())).subscribe_next(|_| {});
}

/// Subscribes to the lower-arity combine-latest overloads of `WhenAnyObservable` against a null
/// observable.
fn subscribe_to_null_low_arity_combine_latest_overloads(fixture: &Rc<TestWhenAnyObsViewModel>) {
    // these are the lower-arity overloads of WhenAnyObservable that perform a CombineLatest
    let _ = fixture.when_any_observable_select((command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
}

/// Subscribes to the higher-arity combine-latest overloads of `WhenAnyObservable` against a null
/// observable.
fn subscribe_to_null_high_arity_combine_latest_overloads(fixture: &Rc<TestWhenAnyObsViewModel>) {
    subscribe_to_null_combine_latest_overloads_arity8_through10(fixture);
    subscribe_to_null_combine_latest_overloads_arity11_through12(fixture);
}

/// Subscribes to the arity 8-10 combine-latest overloads of `WhenAnyObservable` against a null
/// observable.
fn subscribe_to_null_combine_latest_overloads_arity8_through10(fixture: &Rc<TestWhenAnyObsViewModel>) {
    // these are the higher-arity overloads of WhenAnyObservable that perform a CombineLatest
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
}

/// Subscribes to the arity 11-12 combine-latest overloads of `WhenAnyObservable` against a null
/// observable.
fn subscribe_to_null_combine_latest_overloads_arity11_through12(fixture: &Rc<TestWhenAnyObsViewModel>) {
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
    let _ = fixture.when_any_observable_select((command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1(), command1()), |_| ()).subscribe_next(|_| {});
}
