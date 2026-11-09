use super::ElementNotEnabledError;

/// The error of a provider member that the state of its element refuses:
/// the exceptions the members of the provider contracts throw in the
/// original, as values (#567).
///
/// The platform bridges turn it into the result of the platform: the
/// UI Automation bridge of Windows into the result code the original's
/// runtime returns for the exception (`UIA_E_ELEMENTNOTENABLED`,
/// `UIA_E_INVALIDOPERATION`, `COR_E_ARGUMENTOUTOFRANGE`, `COR_E_NOTSUPPORTED`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AutomationError {
    /// The element is not enabled (`ElementNotEnabledException`).
    ElementNotEnabled(ElementNotEnabledError),

    /// The state of the element does not allow the operation
    /// (`InvalidOperationException`); the message.
    InvalidOperation(String),

    /// An argument is out of the range of its valid values
    /// (`ArgumentOutOfRangeException`); the name of the parameter.
    ArgumentOutOfRange(String),

    /// The element does not support the operation
    /// (`NotSupportedException`); the message.
    NotSupported(String),
}

impl AutomationError {
    /// The error of an element that is not enabled, with the default
    /// message.
    pub fn element_not_enabled() -> Self {
        Self::ElementNotEnabled(ElementNotEnabledError::new())
    }

    /// An operation the state of the element does not allow, with the
    /// default message of the original.
    pub fn invalid_operation() -> Self {
        Self::InvalidOperation("Operation is not valid due to the current state of the object.".to_owned())
    }

    /// An operation the state of the element does not allow, with a
    /// message.
    pub fn invalid_operation_with(message: impl Into<String>) -> Self {
        Self::InvalidOperation(message.into())
    }

    /// An argument out of the range of its valid values; `param_name` is
    /// the name of the parameter.
    pub fn argument_out_of_range(param_name: impl Into<String>) -> Self {
        Self::ArgumentOutOfRange(param_name.into())
    }

    /// An operation the element does not support, with the default message
    /// of the original.
    pub fn not_supported() -> Self {
        Self::NotSupported("Specified method is not supported.".to_owned())
    }
}

impl From<ElementNotEnabledError> for AutomationError {
    fn from(error: ElementNotEnabledError) -> Self {
        Self::ElementNotEnabled(error)
    }
}

impl std::fmt::Display for AutomationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ElementNotEnabled(error) => std::fmt::Display::fmt(error, f),
            Self::InvalidOperation(message) | Self::NotSupported(message) => f.write_str(message),
            Self::ArgumentOutOfRange(param_name) => {
                write!(f, "Specified argument was out of the range of valid values. (Parameter '{param_name}')")
            }
        }
    }
}

impl std::error::Error for AutomationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ElementNotEnabled(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Not from the reference.
    #[test]
    fn the_messages_are_the_ones_of_the_exceptions() {
        assert_eq!("Element not enabled.", AutomationError::element_not_enabled().to_string());
        assert_eq!(
            "Operation is not valid due to the current state of the object.",
            AutomationError::invalid_operation().to_string()
        );
        assert_eq!(
            "Specified argument was out of the range of valid values. (Parameter 'verticalPercent')",
            AutomationError::argument_out_of_range("verticalPercent").to_string()
        );
        assert_eq!("Specified method is not supported.", AutomationError::not_supported().to_string());
    }

    // Not from the reference.
    #[test]
    fn an_element_that_is_not_enabled_converts() {
        let error: AutomationError = ElementNotEnabledError::with_message("Disabled.").into();
        assert_eq!(AutomationError::ElementNotEnabled(ElementNotEnabledError::with_message("Disabled.")), error);
    }
}
