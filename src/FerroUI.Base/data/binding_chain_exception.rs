use std::fmt;

/// An error that signals that a binding chain is broken: the value could not
/// be read at some point of the binding path.
#[derive(Clone, Debug, Default)]
pub struct BindingChainException {
    message: Option<String>,
    expression: Option<String>,
    expression_error_point: Option<String>,
}

impl BindingChainException {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self { message: Some(message.into()), expression: None, expression_error_point: None }
    }

    pub fn with_expression(
        message: impl Into<String>,
        expression: impl Into<String>,
        error_point: impl Into<String>,
    ) -> Self {
        Self {
            message: Some(message.into()),
            expression: Some(expression.into()),
            expression_error_point: Some(error_point.into()),
        }
    }

    /// The expression that could not be evaluated.
    pub fn expression(&self) -> Option<&str> {
        self.expression.as_deref()
    }

    /// The point in the expression at which the error occurred.
    pub fn expression_error_point(&self) -> Option<&str> {
        self.expression_error_point.as_deref()
    }

    /// The full message, including the expression and error point.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for BindingChainException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.expression, &self.expression_error_point, &self.message) {
            (Some(expression), Some(point), Some(message)) => {
                write!(f, "An error occurred binding to '{expression}' at '{point}': '{message}'")
            }
            (Some(expression), Some(point), None) => {
                write!(f, "An error occurred binding to '{expression}' at '{point}'.")
            }
            (_, _, Some(message)) => write!(f, "An error occurred in a binding: '{message}'"),
            _ => f.write_str("An error occurred in a binding."),
        }
    }
}

impl std::error::Error for BindingChainException {}
