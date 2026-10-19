//! `InteractionBindingIntegrationTests` (ReactiveUI.Tests,
//! `InteractionBinding/InteractionBindingIntegrationTests.cs`).
//!
//! The handler of a view follows the view's view model, as upstream's generated code does for a view
//! that declares one: `bind_interaction` takes a task-based handler, `bind_interaction_observable`
//! an observable-based one. A question is asked with `block_on` of the task of `handle`.
//! Upstream's checks that a view model is collected once overwritten are checks that its `Rc` is
//! dropped (a `Weak` that no longer upgrades).

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::{block_on, Task};
use ferroui_system::{prop, IDisposable, IObservable};
use std::rc::{Rc, Weak};

const INPUT: &str = "123";

/// The path of the interaction of an [`InteractionBindViewModel`] (`vm => vm.Interaction1`).
fn interaction1() -> ferroui_system::component_model::PropertyPath<Rc<InteractionBindViewModel>, Interaction<String, bool>> {
    prop!(InteractionBindViewModel => interaction1)
}

/// The path of the nested interaction (`vm => vm.InteractionViewModel.Interaction1`).
fn nested_interaction1() -> ferroui_system::component_model::PropertyPath<Rc<InteractionAncestorViewModel>, Interaction<String, bool>> {
    prop!(InteractionAncestorViewModel => interaction_view_model.interaction1)
}

/// The observable-based handler of the tests: it answers `true`.
fn observable_handler(input: Rc<dyn IInteractionContext<String, bool>>) -> Rc<dyn IObservable<()>> {
    input.set_output(true);
    Observable::return_(())
}

/// The task-based handler of the tests: it answers `true`.
fn task_handler(input: Rc<dyn IInteractionContext<String, bool>>) -> Task<()> {
    input.set_output(true);
    Task::from_result(())
}

/// Asks `interaction` and gives its answer.
fn ask(interaction: &Interaction<String, bool>) -> bool {
    block_on(interaction.handle(INPUT.to_owned())).expect("the question is handled")
}

/// Whether asking `interaction` fails with an `UnhandledInteractionException`.
fn is_unhandled(interaction: &Interaction<String, bool>) -> bool {
    let error = block_on(interaction.handle(INPUT.to_owned())).expect_err("no handler is registered");
    error.downcast_ref::<UnhandledInteractionException<String, bool>>().is_some()
}

fn bind_view(view_model: Option<Rc<InteractionBindViewModel>>) -> Rc<InteractionBindView> {
    let view = InteractionBindView::new();
    view.set_view_model(view_model);
    view
}

fn ancestor_view(view_model: Option<Rc<InteractionAncestorViewModel>>) -> Rc<InteractionAncestorView> {
    let view = InteractionAncestorView::new();
    view.set_view_model(view_model);
    view
}

#[test]
fn nested_interaction_should_receive_output_from_observable_handler() {
    let vm = InteractionAncestorViewModel::new();
    let view = ancestor_view(Some(vm.clone()));

    let _binding = view.bind_interaction_observable(nested_interaction1(), observable_handler);

    assert!(ask(&vm.interaction_view_model().interaction1()));
}

#[test]
fn nested_interaction_should_receive_output_from_task_handler() {
    let vm = InteractionAncestorViewModel::new();
    let view = ancestor_view(Some(vm.clone()));

    let _binding = view.bind_interaction(nested_interaction1(), task_handler);

    assert!(ask(&vm.interaction_view_model().interaction1()));
}

#[test]
fn nested_view_model_should_be_garbage_collected_when_overwritten() {
    fn get_weak_reference() -> (Rc<dyn IDisposable>, Weak<InteractionBindViewModel>) {
        let vm = InteractionAncestorViewModel::new();
        vm.set_interaction_view_model(InteractionBindViewModel::new());
        let view = ancestor_view(Some(vm.clone()));
        let weak_ref = Rc::downgrade(&vm.interaction_view_model());
        let disposable = view.bind_interaction_observable(nested_interaction1(), observable_handler);
        vm.set_interaction_view_model(InteractionBindViewModel::new());

        (disposable, weak_ref)
    }

    let (_disposable, weak_ref) = get_weak_reference();

    assert!(weak_ref.upgrade().is_none());
}

#[test]
fn receive_output_from_observable_handler() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let _binding = view.bind_interaction_observable(interaction1(), observable_handler);

    assert!(ask(&vm.interaction1()));
}

#[test]
fn receive_output_from_observable_handler_when_view_model_was_initially_null() {
    let view = bind_view(None);

    let _binding = view.bind_interaction_observable(interaction1(), observable_handler);

    view.set_view_model(Some(InteractionBindViewModel::new()));

    assert!(ask(&view.view_model().expect("the view has a view model").interaction1()));
}

#[test]
fn receive_output_from_task_handler() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let _binding = view.bind_interaction(interaction1(), task_handler);

    assert!(ask(&vm.interaction1()));
}

#[test]
fn receive_output_from_task_handler_when_view_model_was_initially_null() {
    let view = bind_view(None);

    let _binding = view.bind_interaction(interaction1(), task_handler);

    view.set_view_model(Some(InteractionBindViewModel::new()));

    assert!(ask(&view.view_model().expect("the view has a view model").interaction1()));
}

#[test]
fn register_observable_handler_to_newly_assigned_nested_view_model() {
    let vm = InteractionAncestorViewModel::new();
    vm.set_interaction_view_model(InteractionBindViewModel::new());
    let view = ancestor_view(Some(vm.clone()));

    let _binding = view.bind_interaction_observable(nested_interaction1(), observable_handler);

    vm.set_interaction_view_model(InteractionBindViewModel::new());

    assert!(ask(&vm.interaction_view_model().interaction1()));
}

#[test]
fn register_observable_handler_to_newly_assigned_view_model() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm));

    let _binding = view.bind_interaction_observable(interaction1(), observable_handler);

    view.set_view_model(Some(InteractionBindViewModel::new()));

    assert!(ask(&view.view_model().expect("the view has a view model").interaction1()));
}

#[test]
fn register_task_handler_to_newly_assigned_nested_view_model() {
    let vm = InteractionAncestorViewModel::new();
    vm.set_interaction_view_model(InteractionBindViewModel::new());
    let view = ancestor_view(Some(vm.clone()));

    let _binding = view.bind_interaction(nested_interaction1(), task_handler);

    vm.set_interaction_view_model(InteractionBindViewModel::new());

    assert!(ask(&vm.interaction_view_model().interaction1()));
}

#[test]
fn register_task_handler_to_newly_assigned_view_model() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm));

    let _binding = view.bind_interaction(interaction1(), task_handler);

    view.set_view_model(Some(InteractionBindViewModel::new()));

    assert!(ask(&view.view_model().expect("the view has a view model").interaction1()));
}

// Upstream asks a view model that was never bound (`firstInteractionVm`); this asks the nested view
// model the binding had registered on before it was overwritten.
#[test]
fn unregister_observable_handler_from_overwritten_nested_view_model() {
    let vm = InteractionAncestorViewModel::new();
    let first_interaction_vm = vm.interaction_view_model();
    let view = ancestor_view(Some(vm.clone()));

    let _binding = view.bind_interaction_observable(nested_interaction1(), observable_handler);

    vm.set_interaction_view_model(InteractionBindViewModel::new());

    assert!(is_unhandled(&first_interaction_vm.interaction1()));
}

#[test]
fn unregister_observable_handler_from_overwritten_view_model() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let _binding = view.bind_interaction_observable(interaction1(), observable_handler);

    view.set_view_model(Some(InteractionBindViewModel::new()));

    assert!(is_unhandled(&vm.interaction1()));
}

#[test]
fn unregister_observable_handler_when_binding_is_disposed() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let disposable = view.bind_interaction_observable(interaction1(), observable_handler);

    disposable.dispose();

    assert!(is_unhandled(&vm.interaction1()));
}

#[test]
fn unregister_observable_handler_when_view_model_is_set_to_null() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let _binding = view.bind_interaction_observable(interaction1(), observable_handler);

    view.set_view_model(None);

    assert!(is_unhandled(&vm.interaction1()));
}

// Upstream asks a view model that was never bound (`firstInteractionVm`); this asks the nested view
// model the binding had registered on before it was overwritten.
#[test]
fn unregister_task_handler_from_overwritten_nested_view_model() {
    let vm = InteractionAncestorViewModel::new();
    let first_interaction_vm = vm.interaction_view_model();
    let view = ancestor_view(Some(vm.clone()));

    let _binding = view.bind_interaction(nested_interaction1(), task_handler);

    vm.set_interaction_view_model(InteractionBindViewModel::new());

    assert!(is_unhandled(&first_interaction_vm.interaction1()));
}

#[test]
fn unregister_task_handler_from_overwritten_view_model() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let _binding = view.bind_interaction(interaction1(), task_handler);

    view.set_view_model(Some(InteractionBindViewModel::new()));

    assert!(is_unhandled(&vm.interaction1()));
}

#[test]
fn unregister_task_handler_when_binding_is_disposed() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let disposable = view.bind_interaction(interaction1(), task_handler);

    disposable.dispose();

    assert!(is_unhandled(&vm.interaction1()));
}

#[test]
fn unregister_task_handler_when_view_model_is_set_to_null() {
    let vm = InteractionBindViewModel::new();
    let view = bind_view(Some(vm.clone()));

    let _binding = view.bind_interaction(interaction1(), task_handler);

    view.set_view_model(None);

    assert!(is_unhandled(&vm.interaction1()));
}

#[test]
fn view_model_should_be_garbage_collected_when_overwritten() {
    fn get_weak_reference() -> (Rc<dyn IDisposable>, Weak<InteractionBindViewModel>) {
        let vm = InteractionBindViewModel::new();
        let view = bind_view(Some(vm.clone()));
        let weak_ref = Rc::downgrade(&vm);
        let disposable = view.bind_interaction(interaction1(), task_handler);
        view.set_view_model(Some(InteractionBindViewModel::new()));

        (disposable, weak_ref)
    }

    let (_disposable, weak_ref) = get_weak_reference();

    assert!(weak_ref.upgrade().is_none());
}
