//! `InteractionContextTests` (ReactiveUI.Binding.Tests, `Interactions/InteractionContextTests.cs`).
//! `Throws<InvalidOperationException>` is a panic with upstream's message.

use ferroui_reactiveui::*;

const SAMPLE_INPUT: &str = "input";
const SAMPLE_OUTPUT: i32 = 42;
const SECOND_OUTPUT: i32 = 2;

fn create_context(input: &str) -> InteractionContext<String, i32> {
    InteractionContext::new(input.to_owned())
}

#[test]
fn input_returns_construction_value() {
    let context = create_context("hello");

    assert_eq!(context.input(), "hello");
}

#[test]
fn is_handled_false_before_set_output() {
    let context = create_context(SAMPLE_INPUT);

    assert!(!context.is_handled());
}

#[test]
fn set_output_sets_is_handled_true() {
    let context = create_context(SAMPLE_INPUT);

    context.set_output(SAMPLE_OUTPUT);

    assert!(context.is_handled());
}

#[test]
fn get_output_returns_set_value() {
    let context = create_context(SAMPLE_INPUT);

    context.set_output(SAMPLE_OUTPUT);

    assert_eq!(context.get_output(), SAMPLE_OUTPUT);
}

#[test]
#[should_panic(expected = "Output has already been set.")]
fn set_output_called_twice_throws() {
    let context = create_context(SAMPLE_INPUT);
    context.set_output(1);

    context.set_output(SECOND_OUTPUT);
}

#[test]
#[should_panic(expected = "Output has not been set.")]
fn get_output_before_set_output_throws() {
    let context = create_context(SAMPLE_INPUT);

    let _ = context.get_output();
}
