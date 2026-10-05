use crate::data::{BindingError, BindingErrorType};

/// The error state of a binding expression: the error and its kind.
///
/// (The managed counterpart is named like [`BindingError`], which in this
/// port is the error object itself.)
#[derive(Clone, Debug)]
pub struct ExpressionError {
    pub exception: BindingError,
    pub error_type: BindingErrorType,
}

impl ExpressionError {
    pub fn new(exception: BindingError, error_type: BindingErrorType) -> Self {
        Self { exception, error_type }
    }
}
