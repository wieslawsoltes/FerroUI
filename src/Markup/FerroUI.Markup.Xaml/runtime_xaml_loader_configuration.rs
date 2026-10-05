//! Port of `RuntimeXamlLoaderConfiguration.cs`.

use ferroui_base::metadata::MarkupAssembly;
use std::rc::Rc;

/// Called with each diagnostic the runtime loader produces; returns the
/// severity to treat it with.
pub type XamlDiagnosticFunc = Rc<dyn Fn(&RuntimeXamlDiagnostic) -> RuntimeXamlDiagnosticSeverity>;

/// How the runtime XAML loader loads a document.
#[derive(Clone, Default)]
pub struct RuntimeXamlLoaderConfiguration {
    create_source_info: Option<bool>,

    /// The assembly (crate) the document belongs to: names without an
    /// assembly resolve against it first.
    pub local_assembly: Option<&'static MarkupAssembly>,

    /// Whether `{Binding}` is a compiled binding unless the document says
    /// otherwise. Default is `false`.
    pub use_compiled_bindings_by_default: bool,

    /// Whether the document is loaded for a designer. Default is `false`.
    pub design_mode: bool,

    /// Receives the diagnostics of the loader and may change their
    /// severity. Without a handler, errors fail the load and warnings are
    /// ignored.
    pub diagnostic_handler: Option<XamlDiagnosticFunc>,
}

impl RuntimeXamlLoaderConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether source information (document, line, position) is attached to
    /// the objects the loader creates.
    ///
    /// When it was not set explicitly, the default comes from the metadata
    /// of the local assembly ([`MarkupAssembly::CREATE_SOURCE_INFO`], parsed
    /// as a boolean); it is `false` without a local assembly, without the
    /// entry and for a value that is not a boolean.
    pub fn create_source_info(&self) -> bool {
        if self.create_source_info.is_none() {
            if let Some(asm) = self.local_assembly {
                let create_source_info = asm.metadata_value(MarkupAssembly::CREATE_SOURCE_INFO);
                return create_source_info.is_some_and(parse_bool);
            }
        }
        self.create_source_info.unwrap_or(false)
    }

    pub fn set_create_source_info(&mut self, value: bool) {
        self.create_source_info = Some(value);
    }
}

/// `bool.TryParse(text, out value) && value`: `true` in any casing, with
/// surrounding white space allowed.
fn parse_bool(text: &str) -> bool {
    text.trim().eq_ignore_ascii_case("true")
}

/// The severity of a diagnostic of the runtime loader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(i32)]
pub enum RuntimeXamlDiagnosticSeverity {
    /// Diagnostic is reported as an info.
    Info = 1,
    /// Diagnostic is reported as a warning.
    Warning,
    /// Diagnostic is reported as an error; the compilation continues to
    /// collect further diagnostics and then fails.
    Error,
    /// Diagnostic is reported as an error and stops the compilation.
    Fatal,
}

/// A diagnostic of the runtime loader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeXamlDiagnostic {
    pub id: String,
    pub severity: RuntimeXamlDiagnosticSeverity,
    pub title: String,
    pub line_number: Option<i32>,
    pub line_position: Option<i32>,
    /// The document the diagnostic was reported for.
    pub document: Option<String>,
}

impl RuntimeXamlDiagnostic {
    pub fn new(
        id: impl Into<String>,
        severity: RuntimeXamlDiagnosticSeverity,
        title: impl Into<String>,
        line_number: Option<i32>,
        line_position: Option<i32>,
    ) -> Self {
        Self { id: id.into(), severity, title: title.into(), line_number, line_position, document: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_original() {
        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        assert!(configuration.local_assembly.is_none());
        assert!(!configuration.use_compiled_bindings_by_default);
        assert!(!configuration.design_mode);
        assert!(!configuration.create_source_info());
        assert!(configuration.diagnostic_handler.is_none());

        configuration.set_create_source_info(true);
        assert!(configuration.create_source_info());
    }

    #[test]
    fn create_source_info_defaults_to_the_metadata_of_the_local_assembly() {
        static ENABLED: MarkupAssembly = MarkupAssembly {
            name: "Tests.SourceInfo.Enabled",
            crate_name: "tests_source_info_enabled",
            xmlns_definitions: &[],
            xmlns_prefixes: &[],
            metadata: &[("Other", "x"), (MarkupAssembly::CREATE_SOURCE_INFO, " True ")],
        };
        static DISABLED: MarkupAssembly = MarkupAssembly {
            name: "Tests.SourceInfo.Disabled",
            crate_name: "tests_source_info_disabled",
            xmlns_definitions: &[],
            xmlns_prefixes: &[],
            metadata: &[(MarkupAssembly::CREATE_SOURCE_INFO, "false")],
        };
        static UNPARSABLE: MarkupAssembly = MarkupAssembly {
            name: "Tests.SourceInfo.Unparsable",
            crate_name: "tests_source_info_unparsable",
            xmlns_definitions: &[],
            xmlns_prefixes: &[],
            metadata: &[(MarkupAssembly::CREATE_SOURCE_INFO, "yes")],
        };

        let mut configuration = RuntimeXamlLoaderConfiguration::new();
        configuration.local_assembly = Some(&ENABLED);
        assert!(configuration.create_source_info());
        configuration.local_assembly = Some(&DISABLED);
        assert!(!configuration.create_source_info());
        configuration.local_assembly = Some(&UNPARSABLE);
        assert!(!configuration.create_source_info());
        // An assembly without the entry.
        configuration.local_assembly = Some(&crate::ASSEMBLY);
        assert!(!configuration.create_source_info());

        // An explicit value wins over the assembly.
        configuration.local_assembly = Some(&ENABLED);
        configuration.set_create_source_info(false);
        assert!(!configuration.create_source_info());
        configuration.local_assembly = Some(&DISABLED);
        configuration.set_create_source_info(true);
        assert!(configuration.create_source_info());
    }

    #[test]
    fn severities_keep_their_values_and_order() {
        assert_eq!(RuntimeXamlDiagnosticSeverity::Info as i32, 1);
        assert_eq!(RuntimeXamlDiagnosticSeverity::Warning as i32, 2);
        assert_eq!(RuntimeXamlDiagnosticSeverity::Error as i32, 3);
        assert_eq!(RuntimeXamlDiagnosticSeverity::Fatal as i32, 4);
        assert!(RuntimeXamlDiagnosticSeverity::Fatal > RuntimeXamlDiagnosticSeverity::Error);
    }

    #[test]
    fn diagnostic_is_a_record() {
        let mut a = RuntimeXamlDiagnostic::new("FRN1", RuntimeXamlDiagnosticSeverity::Warning, "t", Some(1), None);
        let b = a.clone();
        assert_eq!(a, b);
        a.document = Some("doc".into());
        assert_ne!(a, b);
    }
}
