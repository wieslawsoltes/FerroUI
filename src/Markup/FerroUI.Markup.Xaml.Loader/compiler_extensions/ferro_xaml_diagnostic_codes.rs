//! Port of `CompilerExtensions/FerroXamlDiagnosticCodes.cs`.

use xamlx::diagnostics::XamlXWellKnownDiagnosticCodes;
use xamlx::exceptions::XamlError;
use xamlx::transform::XamlDiagnosticCodeSource;

pub struct FerroXamlDiagnosticCodes;

impl FerroXamlDiagnosticCodes {
    pub const UNKNOWN: &'static str = "FRN9999";

    // XML/XAML parsing errors 1000-1999.
    pub const PARSE_ERROR: &'static str = "FRN1000";
    pub const INVALID_XAML: &'static str = "FRN1001";

    // XAML transform errors 2000-2999.
    pub const TRANSFORM_ERROR: &'static str = "FRN2000";
    pub const DUPLICATE_X_CLASS: &'static str = "FRN2002";
    pub const TYPE_SYSTEM_ERROR: &'static str = "FRN2003";
    pub const FERRO_INTRINSICS_ERROR: &'static str = "FRN2005";
    pub const BINDINGS_ERROR: &'static str = "FRN2100";
    pub const DATA_CONTEXT_RESOLVING_ERROR: &'static str = "FRN2101";
    pub const STYLE_TRANSFORM_ERROR: &'static str = "FRN2200";
    pub const SELECTORS_TRANSFORM_ERROR: &'static str = "FRN2201";
    pub const PROPERTY_PATH_ERROR: &'static str = "FRN2202";
    pub const DUPLICATE_SETTER_ERROR: &'static str = "FRN2203";
    pub const STYLE_IN_MERGED_DICTIONARIES: &'static str = "FRN2204";
    pub const REQUIRED_TEMPLATE_PART_MISSING: &'static str = "FRN2205";
    pub const OPTIONAL_TEMPLATE_PART_MISSING: &'static str = "FRN2206";
    pub const TEMPLATE_PART_WRONG_TYPE: &'static str = "FRN2207";
    pub const ITEM_CONTAINER_INSIDE_TEMPLATE: &'static str = "FRN2208";

    // XAML emit errors 3000-3999.
    pub const EMIT_ERROR: &'static str = "FRN3000";
    pub const XAML_LOADER_UNREACHABLE: &'static str = "FRN3001";

    // Generator specific errors 4000-4999.
    pub const NAME_GENERATOR_ERROR: &'static str = "FRN4001";

    // Reserved 5000-9998
    pub const OBSOLETE: &'static str = "FRN5001";

    /// `XamlXDiagnosticCodeToFerro(object codeOrException)`.
    ///
    /// The exception classes the compiler extensions derive from `XamlTransformException` are
    /// recognised by the derived type name they are tagged with
    /// (`XamlError::with_derived_type_name`): `XamlDataContextException`,
    /// `XamlBindingsTransformException`, `XamlPropertyPathException`,
    /// `XamlStyleTransformException` and `XamlSelectorsTransformException`.
    ///
    /// Upstream throws `ArgumentOutOfRangeException` for an unknown well-known code; the only
    /// well-known code is `Obsolete`, so that case cannot occur here.
    pub fn xaml_x_diagnostic_code_to_ferro(code_or_exception: &XamlDiagnosticCodeSource<'_>) -> String {
        match code_or_exception {
            XamlDiagnosticCodeSource::WellKnown(well_known_diagnostic_codes) => {
                match well_known_diagnostic_codes {
                    XamlXWellKnownDiagnosticCodes::Obsolete => Self::OBSOLETE.to_string(),
                }
            }

            // ExperimentalAttribute reports its own code
            XamlDiagnosticCodeSource::Id(code) => code.to_string(),

            XamlDiagnosticCodeSource::Exception(exception) => {
                Self::exception_to_code(exception).to_string()
            }
        }
    }

    fn exception_to_code(exception: &XamlError) -> &'static str {
        match exception.derived_type_name() {
            Some("XamlDataContextException") => return Self::DATA_CONTEXT_RESOLVING_ERROR,
            Some("XamlBindingsTransformException") => return Self::BINDINGS_ERROR,
            Some("XamlPropertyPathException") => return Self::PROPERTY_PATH_ERROR,
            Some("XamlStyleTransformException") => return Self::STYLE_TRANSFORM_ERROR,
            Some("XamlSelectorsTransformException") => return Self::SELECTORS_TRANSFORM_ERROR,
            _ => {}
        }

        match exception {
            XamlError::Transform(_) => Self::TRANSFORM_ERROR,
            XamlError::TypeSystem(_) => Self::TYPE_SYSTEM_ERROR,
            XamlError::Load(_) => Self::EMIT_ERROR,
            XamlError::Parse(_) => Self::PARSE_ERROR,

            _ => Self::UNKNOWN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler_extensions::transformers::XamlPropertyPathException;
    use xamlx::ast::XamlLineInfo;

    fn code(e: &XamlError) -> String {
        FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro(
            &XamlDiagnosticCodeSource::Exception(e),
        )
    }

    #[test]
    fn maps_code_sources() {
        assert_eq!(
            FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro(
                &XamlDiagnosticCodeSource::WellKnown(XamlXWellKnownDiagnosticCodes::Obsolete)
            ),
            "FRN5001"
        );
        assert_eq!(
            FerroXamlDiagnosticCodes::xaml_x_diagnostic_code_to_ferro(
                &XamlDiagnosticCodeSource::Id("CUSTOM001")
            ),
            "CUSTOM001"
        );
    }

    #[test]
    fn maps_exceptions() {
        let li = XamlLineInfo::new(1, 2);
        assert_eq!(code(&XamlError::transform_exception("x", Some(&li))), "FRN2000");
        assert_eq!(code(&XamlError::type_system_exception("x")), "FRN2003");
        assert_eq!(code(&XamlError::load_exception("x", Some(&li))), "FRN3000");
        assert_eq!(code(&XamlError::parse_exception("x", Some(&li))), "FRN1000");
        assert_eq!(code(&XamlError::invalid_operation("x")), "FRN9999");
        assert_eq!(code(&XamlError::xml_exception("x", 1, 1)), "FRN9999");
        assert_eq!(code(&XamlPropertyPathException::new("x", &li, None)), "FRN2202");
        for (name, expected) in [
            ("XamlDataContextException", "FRN2101"),
            ("XamlBindingsTransformException", "FRN2100"),
            ("XamlStyleTransformException", "FRN2200"),
            ("XamlSelectorsTransformException", "FRN2201"),
        ] {
            let e = XamlError::transform_exception("x", Some(&li)).with_derived_type_name(name);
            assert_eq!(code(&e), expected);
        }
    }
}
