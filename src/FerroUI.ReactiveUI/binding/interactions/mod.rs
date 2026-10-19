//! `Interactions` (ReactiveUI.Binding): questions that one part of an application asks and a
//! handler registered by another part answers, typically a view model asking its view.

mod i_interaction;
mod i_interaction_context;
mod i_output_context;
mod interaction;
mod interaction_context;
mod unhandled_interaction_exception;

pub use i_interaction::IInteraction;
pub use i_interaction_context::IInteractionContext;
pub use i_output_context::IOutputContext;
pub use interaction::{Interaction, InteractionHandler};
pub use interaction_context::InteractionContext;
pub use unhandled_interaction_exception::UnhandledInteractionException;
