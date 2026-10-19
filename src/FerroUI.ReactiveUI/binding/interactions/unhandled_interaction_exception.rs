//! `UnhandledInteractionException<TInput, TOutput>` (ReactiveUI.Binding,
//! `Interactions/UnhandledInteractionException.cs`).

use super::Interaction;
use ferroui_system::ObservableError;
use std::error::Error;
use std::fmt;

/// The error of an interaction that no handler handled (`UnhandledInteractionException<TInput, TOutput>`).
pub struct UnhandledInteractionException<I, O> {
    message: String,
    inner_exception: Option<ObservableError>,
    interaction: Option<Interaction<I, O>>,
    input: Option<I>,
}

impl<I, O> UnhandledInteractionException<I, O> {
    /// The error of `interaction`, which no handler handled for `input`
    /// (`UnhandledInteractionException(Interaction<TInput, TOutput>, TInput)`).
    pub fn new(interaction: Interaction<I, O>, input: I) -> Self {
        Self {
            message: "Failed to find a registration for an Interaction.".to_string(),
            inner_exception: None,
            interaction: Some(interaction),
            input: Some(input),
        }
    }

    /// An error without an interaction (`UnhandledInteractionException()`), with the default
    /// message of an exception.
    pub fn empty() -> Self {
        Self::with_message(format!("Exception of type '{}' was thrown.", std::any::type_name::<Self>()))
    }

    /// An error with `message` (`UnhandledInteractionException(string)`).
    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: message.into(), inner_exception: None, interaction: None, input: None }
    }

    /// An error with `message`, caused by `inner_exception`
    /// (`UnhandledInteractionException(string, Exception)`).
    pub fn with_inner(message: impl Into<String>, inner_exception: ObservableError) -> Self {
        Self { message: message.into(), inner_exception: Some(inner_exception), interaction: None, input: None }
    }

    /// The message (`Message`).
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The error that caused this one (`InnerException`).
    pub fn inner_exception(&self) -> Option<&ObservableError> {
        self.inner_exception.as_ref()
    }

    /// The interaction that no handler handled, if the error names one (`Interaction`).
    pub fn interaction(&self) -> Option<&Interaction<I, O>> {
        self.interaction.as_ref()
    }

    /// The input of the interaction that no handler handled, if the error names one (`Input`;
    /// `default` upstream when it does not).
    pub fn input(&self) -> Option<&I> {
        self.input.as_ref()
    }
}

impl<I, O> Default for UnhandledInteractionException<I, O> {
    fn default() -> Self {
        Self::empty()
    }
}

impl<I, O> fmt::Debug for UnhandledInteractionException<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnhandledInteractionException")
            .field("message", &self.message)
            .field("has_interaction", &self.interaction.is_some())
            .field("inner_exception", &self.inner_exception.as_ref().map(|error| error.to_string()))
            .finish()
    }
}

impl<I, O> fmt::Display for UnhandledInteractionException<I, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl<I, O> Error for UnhandledInteractionException<I, O> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner_exception.as_deref()
    }
}
