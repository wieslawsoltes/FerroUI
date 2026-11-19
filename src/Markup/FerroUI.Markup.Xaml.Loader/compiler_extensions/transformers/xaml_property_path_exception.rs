//! Port of `CompilerExtensions/Transformers/XamlPropertyPathException.cs`.

use xamlx::ast::IXamlLineInfo;
use xamlx::exceptions::XamlError;

/// `XamlPropertyPathException : XamlTransformException`.
///
/// Errors are values of the single [`XamlError`] type; the derived class is represented by a
/// `XamlError::Transform` tagged with the class name, which is what
/// `FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro` and [`Self::is`] look at.
#[deprecated(
    since = "0.5.1",
    note = "use `XamlError::property_path` (`FerroTransformErrors`) and `FerroTransformErrorKind::PropertyPath`"
)]
pub struct XamlPropertyPathException;

#[allow(deprecated)]
impl XamlPropertyPathException {
    pub const TYPE_NAME: &'static str = super::FerroTransformErrorKind::PropertyPath.type_name();

    /// `new XamlPropertyPathException(message, lineInfo, innerException)`, now
    /// [`XamlError::property_path`](super::FerroTransformErrors::property_path).
    pub fn new(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner_exception: Option<XamlError>,
    ) -> XamlError {
        super::FerroTransformErrorKind::PropertyPath.error(message, line_info, inner_exception)
    }

    /// `e is XamlPropertyPathException`.
    pub fn is(error: &XamlError) -> bool {
        super::FerroTransformErrorKind::of(error) == Some(super::FerroTransformErrorKind::PropertyPath)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_extensions::transformers::{FerroTransformErrorKind, FerroTransformErrors};
    use xamlx::ast::XamlLineInfo;

    #[test]
    fn is_a_transform_exception_with_line_info() {
        let e = XamlError::property_path("bad path", &XamlLineInfo::new(3, 7), None);
        assert!(matches!(e, XamlError::Transform(_)));
        assert_eq!(e.transform_kind(), Some(FerroTransformErrorKind::PropertyPath));
        assert_eq!(e.type_name(), "XamlPropertyPathException");
        assert_eq!(e.line_number(), Some(3));
        assert_eq!(e.line_position(), Some(7));
        assert_eq!(e.message(), "bad path Line 3, position 7.");

        let inner = XamlError::invalid_operation("inner");
        let e = XamlError::property_path("outer", &XamlLineInfo::new(1, 1), Some(inner));
        assert_eq!(e.inner_exception().map(|i| i.message()), Some("inner".to_string()));
        let plain = XamlError::transform_exception("x", Some(&XamlLineInfo::new(1, 1)));
        assert_eq!(plain.transform_kind(), None);
    }
}
