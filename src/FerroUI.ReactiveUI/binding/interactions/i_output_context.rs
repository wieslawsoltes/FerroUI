//! `IOutputContext<TInput, TOutput>` (ReactiveUI.Binding, `Interactions/IOutputContext.cs`).

use super::IInteractionContext;

/// An interaction context whose output can be read back (`IOutputContext<TInput, TOutput>`).
pub trait IOutputContext<I, O>: IInteractionContext<I, O> {
    /// The output of the interaction (`GetOutput()`).
    ///
    /// # Panics
    /// When the output has not been set (`InvalidOperationException`).
    fn get_output(&self) -> O;
}
