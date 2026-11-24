//! Port of `CompilerExtensions/Transformers/XamlPropertyPathException.cs`.

use xamlx::ast::IXamlLineInfo;
use xamlx::exceptions::XamlError;

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
