//! The transform errors of the compiler extensions (#423, docs/idiomatic-rust.md, section 7.2).
//!
//! Not a port of an upstream file. Upstream derives five exception classes from
//! `XamlTransformException` (`XamlBindingsTransformException`, `XamlDataContextException`,
//! `XamlPropertyPathException`, `XamlSelectorsTransformException`, `XamlStyleTransformException`).
//! Errors of the compiler are values of the one [`XamlError`] type of `xamlx`, which does not know
//! the kinds of this crate: a transform error of a kind is a `XamlError::Transform` tagged with the
//! name of its class (`XamlError::with_derived_type_name`), the name the diagnostic codes are
//! chosen by (`FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro`). The constructors of
//! [`FerroTransformErrors`] make them; the unit structs of the classes are deprecated forms of them.

use xamlx::ast::IXamlLineInfo;
use xamlx::exceptions::XamlError;

/// The kind of a transform error of the compiler extensions: upstream's derived class of
/// `XamlTransformException`, whose name the error carries and its diagnostic code is chosen by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FerroTransformErrorKind {
    /// `XamlBindingsTransformException` (`FRN2100`): a binding or `x:Bind` that does not compile.
    Bindings,
    /// `XamlDataContextException` (`FRN2101`): the data context type cannot be resolved.
    DataContext,
    /// `XamlPropertyPathException` (`FRN2202`): a property path that does not resolve.
    PropertyPath,
    /// `XamlSelectorsTransformException` (`FRN2201`): a selector that does not transform.
    Selectors,
    /// `XamlStyleTransformException` (`FRN2200`): a setter of a style that does not transform.
    Style,
}

impl FerroTransformErrorKind {
    /// Every kind.
    pub const ALL: [FerroTransformErrorKind; 5] = [
        Self::Bindings,
        Self::DataContext,
        Self::PropertyPath,
        Self::Selectors,
        Self::Style,
    ];

    /// The name of upstream's class, which the error carries and the diagnostic code is chosen by.
    pub const fn type_name(self) -> &'static str {
        match self {
            Self::Bindings => "XamlBindingsTransformException",
            Self::DataContext => "XamlDataContextException",
            Self::PropertyPath => "XamlPropertyPathException",
            Self::Selectors => "XamlSelectorsTransformException",
            Self::Style => "XamlStyleTransformException",
        }
    }

    /// The transform error of this kind: `new <Class>(message, lineInfo, innerException)`.
    pub fn error(
        self,
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> XamlError {
        let error = match inner {
            Some(inner) => {
                XamlError::transform_exception_with_inner(message, Some(line_info), inner)
            }
            None => XamlError::transform_exception(message, Some(line_info)),
        };
        error.with_derived_type_name(self.type_name())
    }

    /// The kind of `error`, if it is a transform error of one of the kinds (`e is <Class>`).
    pub fn of(error: &XamlError) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| error.is_derived_type(kind.type_name()))
    }
}

/// The constructors of the transform errors of the compiler extensions on [`XamlError`], and the
/// test of their kind (`e is XamlBindingsTransformException`). Each constructor takes the message,
/// the position and the inner error, as upstream's constructors do.
pub trait FerroTransformErrors: Sized {
    /// `new XamlBindingsTransformException(message, lineInfo, innerException)`.
    fn bindings_transform(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self;
    /// `new XamlDataContextException(message, lineInfo, innerException)`.
    fn data_context(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self;
    /// `new XamlPropertyPathException(message, lineInfo, innerException)`.
    fn property_path(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self;
    /// `new XamlSelectorsTransformException(message, lineInfo, innerException)`.
    fn selectors_transform(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self;
    /// `new XamlStyleTransformException(message, lineInfo, innerException)`.
    fn style_transform(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self;
    /// The kind of the error, if it is a transform error of one of the kinds.
    fn transform_kind(&self) -> Option<FerroTransformErrorKind>;
}

impl FerroTransformErrors for XamlError {
    fn bindings_transform(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self {
        FerroTransformErrorKind::Bindings.error(message, line_info, inner)
    }

    fn data_context(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self {
        FerroTransformErrorKind::DataContext.error(message, line_info, inner)
    }

    fn property_path(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self {
        FerroTransformErrorKind::PropertyPath.error(message, line_info, inner)
    }

    fn selectors_transform(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self {
        FerroTransformErrorKind::Selectors.error(message, line_info, inner)
    }

    fn style_transform(
        message: impl Into<String>,
        line_info: &dyn IXamlLineInfo,
        inner: Option<XamlError>,
    ) -> Self {
        FerroTransformErrorKind::Style.error(message, line_info, inner)
    }

    fn transform_kind(&self) -> Option<FerroTransformErrorKind> {
        FerroTransformErrorKind::of(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xamlx::ast::XamlLineInfo;

    /// Not from upstream (#423): each constructor makes a transform error of its kind, with the
    /// message, the position and the inner error, and the type name its diagnostic code is chosen
    /// by.
    #[test]
    fn the_constructors_make_transform_errors_of_their_kinds() {
        type Constructor = fn(String, &dyn IXamlLineInfo, Option<XamlError>) -> XamlError;
        let constructors: [(FerroTransformErrorKind, Constructor); 5] = [
            (FerroTransformErrorKind::Bindings, |m, l, i| {
                XamlError::bindings_transform(m, l, i)
            }),
            (FerroTransformErrorKind::DataContext, |m, l, i| {
                XamlError::data_context(m, l, i)
            }),
            (FerroTransformErrorKind::PropertyPath, |m, l, i| {
                XamlError::property_path(m, l, i)
            }),
            (FerroTransformErrorKind::Selectors, |m, l, i| {
                XamlError::selectors_transform(m, l, i)
            }),
            (FerroTransformErrorKind::Style, |m, l, i| {
                XamlError::style_transform(m, l, i)
            }),
        ];
        for (kind, constructor) in constructors {
            let error = constructor("bad".to_string(), &XamlLineInfo::new(3, 7), None);
            assert!(matches!(error, XamlError::Transform(_)));
            assert_eq!(error.type_name(), kind.type_name());
            assert_eq!(error.transform_kind(), Some(kind));
            assert_eq!(
                (error.line_number(), error.line_position()),
                (Some(3), Some(7))
            );
            assert_eq!(error.message(), "bad Line 3, position 7.");
            let inner = constructor(
                "outer".to_string(),
                &XamlLineInfo::new(1, 1),
                Some(XamlError::invalid_operation("inner")),
            );
            assert_eq!(
                inner.inner_exception().map(|inner| inner.message()),
                Some("inner".to_string())
            );
        }
        assert_eq!(
            XamlError::transform_exception("plain", Some(&XamlLineInfo::new(1, 1)))
                .transform_kind(),
            None
        );
    }

    /// Not from upstream (#423): each deprecated factory gives the error of its constructor, and
    /// tells its kind as before.
    #[test]
    #[allow(deprecated)]
    fn the_deprecated_factories_give_the_errors_of_the_constructors() {
        use crate::compiler_extensions::transformers::{
            XamlBindingsTransformException, XamlDataContextException, XamlPropertyPathException,
            XamlSelectorsTransformException, XamlStyleTransformException,
        };
        let line = XamlLineInfo::new(2, 4);
        let pairs = [
            (
                XamlBindingsTransformException::new("m", &line, None),
                XamlError::bindings_transform("m", &line, None),
            ),
            (
                XamlDataContextException::new("m", &line, None),
                XamlError::data_context("m", &line, None),
            ),
            (
                XamlPropertyPathException::new("m", &line, None),
                XamlError::property_path("m", &line, None),
            ),
            (
                XamlSelectorsTransformException::new("m", &line, None),
                XamlError::selectors_transform("m", &line, None),
            ),
            (
                XamlStyleTransformException::new("m", &line, None),
                XamlError::style_transform("m", &line, None),
            ),
        ];
        for (old, new) in &pairs {
            assert_eq!(
                (old.type_name(), old.message(), old.transform_kind()),
                (new.type_name(), new.message(), new.transform_kind())
            );
        }
        assert!(
            XamlBindingsTransformException::is(&pairs[0].1)
                && !XamlStyleTransformException::is(&pairs[0].1)
        );
        assert_eq!(
            XamlStyleTransformException::TYPE_NAME,
            "XamlStyleTransformException"
        );
    }
}
