//! `IInteractionContext<TInput, TOutput>` (ReactiveUI.Binding, `Interactions/IInteractionContext.cs`):
//! what a handler of an interaction receives.

/// The contextual information of an interaction, which its handlers receive
/// (`IInteractionContext<TInput, TOutput>`): the input, whether a handler set the output, and the
/// setting of the output.
///
/// A handler should set the output once; a second call is a logic error and panics. A handler
/// that should only respond when no other did checks [`is_handled`](Self::is_handled) first.
pub trait IInteractionContext<I, O> {
    /// The input of the interaction (`Input`).
    fn input(&self) -> &I;

    /// Whether the interaction is handled, that is whether the output was set (`IsHandled`).
    fn is_handled(&self) -> bool;

    /// Sets the output of the interaction (`SetOutput(TOutput)`).
    fn set_output(&self, output: O);
}
