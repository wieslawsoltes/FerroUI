//! `BindingInteractionIntegrationTests` (`ReactiveUI.Tests/Interactions/BindingInteractionIntegrationTests.cs`):
//! the interactions of ReactiveUI.Binding as ReactiveUI uses them. `await interaction.Handle(..)`
//! is `block_on` of its task.

use crate::mocks::*;
use ferroui_reactiveui::*;
use ferroui_system::threading::tasks::block_on;

const REQUEST: &str = "request";

#[test]
fn handle_registered_handler_returns_output() {
    let interaction = Interaction::<String, bool>::new();
    let registration = interaction.register_handler(|context| context.set_output(context.input() == "accept"));

    assert!(block_on(interaction.handle("accept".to_owned())).expect("the question is handled"));
    registration.dispose();
}

#[test]
fn handle_disposed_latest_handler_uses_earlier_handler() {
    let interaction = Interaction::<String, String>::new();
    let first = interaction.register_handler(|context| context.set_output("first".to_owned()));
    let latest = interaction.register_handler(|context| context.set_output("latest".to_owned()));

    assert_eq!(block_on(interaction.handle(REQUEST.to_owned())).expect("the question is handled"), "latest");
    latest.dispose();
    assert_eq!(block_on(interaction.handle(REQUEST.to_owned())).expect("the question is handled"), "first");
    first.dispose();
}

#[test]
fn handle_async_handler_returns_output() {
    let interaction = Interaction::<i32, i32>::new();
    let registration = interaction.register_handler_task(|context| async move {
        yield_now().await?;
        context.set_output(*context.input() + 1);
        Ok(())
    });

    assert_eq!(block_on(interaction.handle(1)).expect("the question is handled"), 2);
    registration.dispose();
}

#[test]
fn handle_no_handler_throws() {
    let interaction = Interaction::<String, bool>::new();

    let error = block_on(interaction.handle(REQUEST.to_owned())).expect_err("no handler handles the question");

    assert!(error.downcast_ref::<UnhandledInteractionException<String, bool>>().is_some());
}
