//! `IInteraction<TInput, TOutput>` (ReactiveUI.Binding, `Interactions/IInteraction.cs`).

use super::IInteractionContext;
use ferroui_system::threading::tasks::Task;
use ferroui_system::{IDisposable, IObservable};
use std::rc::Rc;

/// An interaction between collaborating parts of an application (`IInteraction<TInput, TOutput>`):
/// one part asks a question (the input), and a handler that another part registered answers it
/// (the output). Typically a view model asks its view for a confirmation.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the handlers are shared closures, so that the trait can
/// be a trait object; the asynchronous handler gives a `Task<()>`, and the observable-based one an
/// observable of `()` (upstream's `TDontCare`: its values are ignored). [`Interaction`](super::Interaction)
/// itself has generic forms that take any closure.
pub trait IInteraction<I, O> {
    /// Registers a synchronous handler; disposing the result unregisters it
    /// (`RegisterHandler(Action<IInteractionContext<TInput, TOutput>>)`).
    fn register_handler(&self, handler: Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>)>) -> Rc<dyn IDisposable>;

    /// Registers a task-based asynchronous handler; disposing the result unregisters it
    /// (`RegisterHandler(Func<IInteractionContext<TInput, TOutput>, Task>)`).
    fn register_handler_task(&self, handler: Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>) -> Task<()>>) -> Rc<dyn IDisposable>;

    /// Registers an observable-based asynchronous handler, which finishes when its observable ends
    /// (`RegisterHandler<TDontCare>(Func<IInteractionContext<TInput, TOutput>, IObservable<TDontCare>>)`).
    fn register_handler_observable(
        &self,
        handler: Rc<dyn Fn(Rc<dyn IInteractionContext<I, O>>) -> Rc<dyn IObservable<()>>>,
    ) -> Rc<dyn IDisposable>;

    /// Asks the question: a cold observable that runs the handlers, latest registered first, until
    /// one sets an output, each time it is subscribed, then notifies the output and ends. It fails
    /// with [`UnhandledInteractionException`](super::UnhandledInteractionException) when no handler
    /// sets one (`Handle(TInput)`).
    fn handle(&self, input: I) -> Rc<dyn IObservable<O>>;
}
