//! `InteractionTests` (ReactiveUI.Binding.Tests, `Interactions/InteractionTests.cs`).
//!
//! `await interaction.Handle(..)` is `block_on` of its task. A handler that throws upstream returns
//! an `Err`, or panics where upstream throws synchronously.

use crate::mocks::*;
use ferroui_reactive::linq::Observable;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::block_on;
use ferroui_system::ObservableError;
use std::cell::Cell;
use std::rc::Rc;

const FIRST_HANDLER_RESULT: &str = "first";
const SECOND_HANDLER_RESULT: &str = "second";
const HELLO_LENGTH: i32 = 5;
const SAMPLE_OUTPUT: i32 = 42;
const ACTION_OUTPUT: i32 = 99;
const HANDLE_INPUT: &str = "input";
const HANDLER_ERROR_MESSAGE: &str = "test error";
const REGISTERED_HANDLER_COUNT: usize = 2;

fn is_unhandled<I: 'static, O: 'static>(error: &ObservableError) -> bool {
    error.downcast_ref::<UnhandledInteractionException<I, O>>().is_some()
}

#[test]
fn handle_sync_handler_returns_output() {
    let interaction = Interaction::<String, i32>::new();
    let registration = interaction.register_handler(|context| context.set_output(context.input().len() as i32));

    let result = block_on(interaction.handle("hello".to_owned())).expect("the question is handled");

    assert_eq!(result, HELLO_LENGTH);
    registration.dispose();
}

#[test]
fn handle_task_handler_returns_output() {
    let interaction = Interaction::<String, bool>::new();
    let registration = interaction.register_handler_task(|context| async move {
        yield_now().await?;
        context.set_output(true);
        Ok(())
    });

    let result = block_on(interaction.handle("question".to_owned())).expect("the question is handled");

    assert!(result);
    registration.dispose();
}

#[test]
fn handle_observable_handler_returns_output() {
    let interaction = Interaction::<String, i32>::new();
    let registration = interaction.register_handler_observable(|context| {
        context.set_output(SAMPLE_OUTPUT);
        Observable::return_(0)
    });

    let result = block_on(interaction.handle("test".to_owned())).expect("the question is handled");

    assert_eq!(result, SAMPLE_OUTPUT);
    registration.dispose();
}

#[test]
fn handle_multiple_handlers_lifo_order() {
    let interaction = Interaction::<String, String>::new();
    let first = interaction.register_handler(|context| context.set_output(FIRST_HANDLER_RESULT.to_owned()));
    let second = interaction.register_handler(|context| context.set_output(SECOND_HANDLER_RESULT.to_owned()));

    let result = block_on(interaction.handle(HANDLE_INPUT.to_owned())).expect("the question is handled");

    assert_eq!(result, SECOND_HANDLER_RESULT);
    first.dispose();
    second.dispose();
}

/// Upstream lists the handlers through a subclass (`HandlerListingInteraction`), since
/// `GetHandlers` is protected; it is public here.
#[test]
fn get_handlers_returns_a_copy_of_the_registered_handlers() {
    let interaction = Interaction::<String, String>::new();
    let first = interaction.register_handler(|context| context.set_output(FIRST_HANDLER_RESULT.to_owned()));
    let second = interaction.register_handler(|context| context.set_output(SECOND_HANDLER_RESULT.to_owned()));

    let mut handlers = interaction.get_handlers();
    handlers.clear();

    assert_eq!(interaction.get_handlers().len(), REGISTERED_HANDLER_COUNT);
    assert_eq!(block_on(interaction.handle(HANDLE_INPUT.to_owned())).expect("the question is handled"), SECOND_HANDLER_RESULT);
    first.dispose();
    second.dispose();
}

#[test]
fn handle_no_handlers_throws_unhandled_interaction_exception() {
    let interaction = Interaction::<String, i32>::new();

    let error = block_on(interaction.handle("test".to_owned())).expect_err("no handler handles the question");

    assert!(is_unhandled::<String, i32>(&error));
}

#[test]
fn dispose_unregisters_handler() {
    let interaction = Interaction::<String, i32>::new();
    let registration = interaction.register_handler(|context| context.set_output(SAMPLE_OUTPUT));
    registration.dispose();

    let error = block_on(interaction.handle("test".to_owned())).expect_err("no handler handles the question");

    assert!(is_unhandled::<String, i32>(&error));
}

#[test]
fn handle_handler_skips_falls_to_next() {
    let interaction = Interaction::<String, String>::new();
    let first = interaction.register_handler(|context| context.set_output(FIRST_HANDLER_RESULT.to_owned()));
    let second = interaction.register_handler(|_context| {
        // Intentionally does not set the output: skip.
    });

    let result = block_on(interaction.handle(HANDLE_INPUT.to_owned())).expect("the question is handled");

    assert_eq!(result, FIRST_HANDLER_RESULT);
    first.dispose();
    second.dispose();
}

// Not ported: RegisterHandler_Action_NullHandler_Throws - argument validation
// Not ported: RegisterHandler_TaskHandler_NullHandler_Throws - argument validation
// Not ported: RegisterHandler_ObservableHandler_NullHandler_Throws - argument validation

#[test]
fn handle_observable_handler_on_error_propagates_exception() {
    let interaction = Interaction::<String, bool>::new();
    let registration = interaction.register_handler_observable(|context| {
        context.set_output(true);
        Observable::throw::<i32>(InvalidOperationException::error(HANDLER_ERROR_MESSAGE))
    });

    let error = block_on(interaction.handle("test".to_owned())).expect_err("the handler fails");

    assert!(is_invalid_operation(&error));
    assert_eq!(error.to_string(), HANDLER_ERROR_MESSAGE);
    registration.dispose();
}

#[test]
fn register_handler_action_sets_output() {
    let interaction = Interaction::<String, i32>::new();
    let registration = interaction.register_handler(|context| context.set_output(ACTION_OUTPUT));

    let result = block_on(interaction.handle("test".to_owned())).expect("the question is handled");

    assert_eq!(result, ACTION_OUTPUT);
    registration.dispose();
}

#[test]
fn when_handled_runs_handlers_on_each_subscription() {
    let interaction = Interaction::<String, i32>::new();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let registration = interaction.register_handler(move |context| {
        count.set(count.get() + 1);
        context.set_output(context.input().len() as i32);
    });

    let question = interaction.when_handled("hello".to_owned());
    assert_eq!(calls.get(), 0);

    let first = CompletionObserver::<i32>::new();
    let subscription = question.subscribe(first.clone());
    assert_eq!(block_on(first.completion()).expect("the question is handled"), HELLO_LENGTH);
    subscription.dispose();

    let second = CompletionObserver::<i32>::new();
    let subscription = question.subscribe(second.clone());
    assert_eq!(block_on(second.completion()).expect("the question is handled"), HELLO_LENGTH);
    subscription.dispose();

    assert_eq!(calls.get(), REGISTERED_HANDLER_COUNT);
    registration.dispose();
}

#[test]
fn when_handled_no_handlers_fails_with_unhandled_interaction_exception() {
    let interaction = Interaction::<String, i32>::new();
    let observer = CompletionObserver::<i32>::new();

    let subscription = interaction.when_handled(HANDLE_INPUT.to_owned()).subscribe(observer.clone());

    let error = block_on(observer.completion()).expect_err("no handler handles the question");
    assert!(is_unhandled::<String, i32>(&error));
    subscription.dispose();
}

#[test]
fn interface_handle_emits_the_output() {
    let interaction: Rc<dyn IInteraction<String, i32>> = Rc::new(Interaction::<String, i32>::new());
    let registration = interaction.register_handler(Rc::new(|context: Rc<dyn IInteractionContext<String, i32>>| context.set_output(SAMPLE_OUTPUT)));
    let observer = CompletionObserver::<i32>::new();

    let subscription = interaction.handle(HANDLE_INPUT.to_owned()).subscribe(observer.clone());

    assert_eq!(block_on(observer.completion()).expect("the question is handled"), SAMPLE_OUTPUT);
    subscription.dispose();
    registration.dispose();
}

#[test]
fn handle_with_scheduler_invokes_handler_on_the_scheduler() {
    let scheduler = ManualScheduler::new();
    let interaction = Interaction::<String, i32>::with_scheduler(Some(scheduler.scheduler()));
    let invoked = Rc::new(Cell::new(false));
    let flag = invoked.clone();
    let registration = interaction.register_handler(move |context| {
        flag.set(true);
        context.set_output(SAMPLE_OUTPUT);
    });

    let pending = interaction.handle(HANDLE_INPUT.to_owned());
    assert!(!invoked.get());

    assert_eq!(scheduler.run_pending(), 1);
    assert_eq!(block_on(pending).expect("the question is handled"), SAMPLE_OUTPUT);
    assert!(invoked.get());
    registration.dispose();
}

#[test]
fn handle_with_scheduler_falls_to_next_handler() {
    let interaction = Interaction::<String, String>::with_scheduler(Some(immediate()));
    let first = interaction.register_handler(|context| context.set_output(FIRST_HANDLER_RESULT.to_owned()));
    let second = interaction.register_handler(|_context| {
        // Intentionally does not set the output: skip.
    });

    assert_eq!(block_on(interaction.handle(HANDLE_INPUT.to_owned())).expect("the question is handled"), FIRST_HANDLER_RESULT);
    first.dispose();
    second.dispose();
}

/// The handler throws before it gives a task (`static Task (_) => throw ...`): here it panics,
/// and the question fails with the panic as a `PanicException`.
#[test]
fn handle_with_scheduler_handler_throws_faults_handle() {
    let interaction = Interaction::<String, bool>::with_scheduler(Some(immediate()));
    let registration =
        interaction.register_handler_task(|_context| -> std::future::Ready<Result<(), ObservableError>> { panic!("{}", HANDLER_ERROR_MESSAGE) });

    let error = block_on(interaction.handle(HANDLE_INPUT.to_owned())).expect_err("the handler fails");

    assert!(error.downcast_ref::<PanicException>().is_some());
    assert_eq!(error.to_string(), HANDLER_ERROR_MESSAGE);
    registration.dispose();
}

#[test]
fn handle_with_null_scheduler_invokes_handler_inline() {
    let interaction = Interaction::<String, i32>::with_scheduler(None);
    let registration = interaction.register_handler(|context| context.set_output(SAMPLE_OUTPUT));

    let pending = interaction.handle(HANDLE_INPUT.to_owned());

    assert!(pending.is_completed());
    assert_eq!(block_on(pending).expect("the question is handled"), SAMPLE_OUTPUT);
    registration.dispose();
}
