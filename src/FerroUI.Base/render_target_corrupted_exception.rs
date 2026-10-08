use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// The error a render target reports when it can no longer be drawn to and has to be created anew.
#[derive(Clone, Debug)]
pub struct RenderTargetCorruptedException {
    message: Option<String>,
    inner_exception: Option<Rc<dyn Error>>,
}

impl RenderTargetCorruptedException {
    /// Creates the exception without a message.
    pub fn new() -> Self {
        Self { message: None, inner_exception: None }
    }

    /// Creates the exception with the given message.
    pub fn new_with_message(message: impl Into<String>) -> Self {
        Self { message: Some(message.into()), inner_exception: None }
    }

    /// Creates the exception for the error that caused it.
    pub fn new_with_inner_exception(inner_exception: Rc<dyn Error>) -> Self {
        Self { message: None, inner_exception: Some(inner_exception) }
    }

    /// Creates the exception with the given message, for the error that
    /// caused it.
    pub fn new_with_message_and_inner_exception(message: impl Into<String>, inner_exception: Rc<dyn Error>) -> Self {
        Self { message: Some(message.into()), inner_exception: Some(inner_exception) }
    }

    /// The message that describes the error: the given one, or the default
    /// message of the exception.
    pub fn message(&self) -> &str {
        self.message.as_deref().unwrap_or("Exception of type 'FerroUI.RenderTargetCorruptedException' was thrown.")
    }

    /// The error that caused the exception, if any.
    pub fn inner_exception(&self) -> Option<&Rc<dyn Error>> {
        self.inner_exception.as_ref()
    }
}

impl Default for RenderTargetCorruptedException {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RenderTargetCorruptedException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl Error for RenderTargetCorruptedException {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner_exception.as_deref()
    }
}
