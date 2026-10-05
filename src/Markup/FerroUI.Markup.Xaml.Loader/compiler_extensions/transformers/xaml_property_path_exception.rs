//! Port of `CompilerExtensions/Transformers/XamlPropertyPathException.cs`.

use xamlx::ast::IXamlLineInfo;
use xamlx::exceptions::XamlError;

/// `XamlPropertyPathException : XamlTransformException`.
///
/// Errors are values of the single [`XamlError`] type; the derived class is represented by a
/// `XamlError::Transform` tagged with the class name, which is what
/// `FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro` and [`Self::is`] look at.
pub struct XamlPropertyPathException;

impl XamlPropertyPathException {
    pub const TYPE_NAME: &'static str = "XamlPropertyPathException";

    /// `new XamlPropertyPathException(message, lineInfo, innerException)`.
    pub fn new(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner_exception: Option<XamlError>,
    ) -> XamlError {
        let error = match inner_exception {
            Some(inner) => {
                XamlError::transform_exception_with_inner(message, Some(line_info), inner)
            }
            None => XamlError::transform_exception(message, Some(line_info)),
        };
        error.with_derived_type_name(Self::TYPE_NAME)
    }

    /// `e is XamlPropertyPathException`.
    pub fn is(error: &XamlError) -> bool {
        error.is_derived_type(Self::TYPE_NAME)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xamlx::ast::XamlLineInfo;

    #[test]
    fn is_a_transform_exception_with_line_info() {
        let e = XamlPropertyPathException::new("bad path", &XamlLineInfo::new(3, 7), None);
        assert!(matches!(e, XamlError::Transform(_)));
        assert!(XamlPropertyPathException::is(&e));
        assert_eq!(e.type_name(), "XamlPropertyPathException");
        assert_eq!(e.line_number(), Some(3));
        assert_eq!(e.line_position(), Some(7));
        assert_eq!(e.message(), "bad path Line 3, position 7.");

        let inner = XamlError::invalid_operation("inner");
        let e = XamlPropertyPathException::new("outer", &XamlLineInfo::new(1, 1), Some(inner));
        assert_eq!(e.inner_exception().map(|i| i.message()), Some("inner".to_string()));
        assert!(!XamlPropertyPathException::is(&XamlError::transform_exception(
            "x",
            Some(&XamlLineInfo::new(1, 1))
        )));
    }
}
