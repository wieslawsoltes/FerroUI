//! `CombinedReactiveCommandTest` (`ReactiveUI.Tests/Commands/CombinedReactiveCommandTest.cs`).

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::{IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn can_execute_is_false_if_any_child_cannot_execute() {
    with_immediate_schedulers();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), Some(Observable::return_(false)), Some(immediate()));
    let fixture = ReactiveCommand::create_combined_with([child1, child2], None, Some(immediate()));
    let can_execute = collect_items(&fixture.can_execute());

    assert_eq!(*can_execute.borrow(), [false]);
}

#[test]
fn can_execute_is_false_if_parent_can_execute_is_false() {
    with_immediate_schedulers();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let fixture = ReactiveCommand::create_combined_with([child1, child2], Some(Observable::return_(false)), Some(immediate()));
    let can_execute = collect_items(&fixture.can_execute());

    assert_eq!(*can_execute.borrow(), [false]);
}

#[test]
fn can_execute_ticks_failures_in_child_can_execute_through_thrown_exceptions() {
    with_immediate_schedulers();
    let can_execute_subject = Subject::<bool>::new();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), Some(can_execute_subject.as_observable()), Some(immediate()));
    let fixture = ReactiveCommand::create_combined_with([child1, child2], None, Some(immediate()));
    let thrown_exceptions = collect_items(&fixture.thrown_exceptions());

    can_execute_subject.on_error(InvalidOperationException::error("oops"));

    assert_eq!(thrown_exceptions.borrow().len(), 1);
    assert_eq!(thrown_exceptions.borrow()[0].to_string(), "oops");
}

#[test]
fn can_execute_ticks_failures_through_thrown_exceptions() {
    with_immediate_schedulers();
    let can_execute_subject = Subject::<bool>::new();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let fixture = ReactiveCommand::create_combined_with([child1, child2], Some(can_execute_subject.as_observable()), Some(immediate()));
    let thrown_exceptions = collect_items(&fixture.thrown_exceptions());

    can_execute_subject.on_error(InvalidOperationException::error("oops"));

    assert_eq!(thrown_exceptions.borrow().len(), 1);
    assert_eq!(thrown_exceptions.borrow()[0].to_string(), "oops");
}

#[test]
fn exceptions_are_delivered_on_output_scheduler() {
    let scheduler = with_immediate_schedulers();
    let child = ReactiveCommand::create_from_observable(|| Observable::throw::<()>(InvalidOperationException::error("oops")));
    let fixture = ReactiveCommand::create_combined_with([child], None, Some(scheduler));
    let exception: Rc<RefCell<Option<ObservableError>>> = Rc::new(RefCell::new(None));
    let thrown = exception.clone();
    let _ = fixture.thrown_exceptions().subscribe_next(move |error| *thrown.borrow_mut() = Some(error));

    let _ = subscribe_ignoring_errors(&fixture.execute_default());

    // With the immediate scheduler, the exceptions are delivered at once.
    assert!(exception.borrow().as_ref().is_some_and(is_invalid_operation));
}

#[test]
fn execute_executes_all_child_commands() {
    with_immediate_schedulers();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child3 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let fixture = ReactiveCommand::create_combined_with([child1.clone(), child2.clone(), child3.clone()], None, Some(immediate()));

    let is_executing = collect_items(&fixture.is_executing());
    let child1_is_executing = collect_items(&child1.is_executing());
    let child2_is_executing = collect_items(&child2.is_executing());
    let child3_is_executing = collect_items(&child3.is_executing());

    let _ = fixture.execute_default().subscribe_ignore();

    assert_eq!(*is_executing.borrow(), [false, true, false]);
    assert_eq!(*child1_is_executing.borrow(), [false, true, false]);
    assert_eq!(*child2_is_executing.borrow(), [false, true, false]);
    assert_eq!(*child3_is_executing.borrow(), [false, true, false]);
}

#[test]
fn execute_ticks_errors_in_any_child_command_through_thrown_exceptions() {
    with_immediate_schedulers();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(()), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(
        || Observable::throw::<()>(InvalidOperationException::error("oops")),
        None,
        Some(immediate()),
    );
    let fixture = ReactiveCommand::create_combined_with([child1, child2], None, Some(immediate()));
    let thrown_exceptions = collect_items(&fixture.thrown_exceptions());

    let _ = subscribe_ignoring_errors(&fixture.execute_default());

    assert_eq!(thrown_exceptions.borrow().len(), 1);
    assert_eq!(thrown_exceptions.borrow()[0].to_string(), "oops");
}

#[test]
fn execute_ticks_through_the_results() {
    with_immediate_schedulers();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(1), None, Some(immediate()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(2), None, Some(immediate()));
    let fixture = ReactiveCommand::create_combined_with([child1, child2], None, Some(immediate()));

    let results = collect_items(&fixture.as_observable());

    let _ = fixture.execute_default().subscribe_ignore();

    assert_eq!(*results.borrow(), [vec![1, 2]]);
}

#[test]
fn result_is_ticked_through_specified_scheduler() {
    let scheduler = with_immediate_schedulers();
    let child1 = ReactiveCommand::create_from_observable_with(|| Observable::return_(1), None, Some(scheduler.clone()));
    let child2 = ReactiveCommand::create_from_observable_with(|| Observable::return_(2), None, Some(scheduler.clone()));
    let fixture = ReactiveCommand::create_combined_with([child1, child2], None, Some(scheduler));
    let results = collect_items(&fixture.as_observable());

    let _ = fixture.execute_default().subscribe_ignore();

    // With the immediate scheduler, the results are delivered at once.
    assert_eq!(results.borrow().len(), 1);
}
