//! Port of `XamlCompilerDiagnosticsFilter.cs`.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use xamlx::{XamlDiagnostic, XamlDiagnosticSeverity};

/// The start of a severity entry of an EditorConfig file, up to the code of
/// the diagnostic: `ferro_xaml_diagnostic.<code>.severity = <severity>`.
const KEY_PREFIX: &str = "ferro_xaml_diagnostic.";

/// What follows the code of the diagnostic in a severity entry.
const KEY_SUFFIX: &str = ".severity";

/// Decides the severity a diagnostic of the markup compiler is reported
/// with, from the severities the EditorConfig files of the build state.
// Upstream leaves the properties of the build that turn warnings into errors
// or silence them to the build system, which applies them to what the task
// reports. The EditorConfig files are read here, as there.
pub struct XamlCompilerDiagnosticsFilter {
    analyzer_config_files: Option<Vec<PathBuf>>,
    lazy_editor_config: OnceCell<HashMap<String, String>>,
}

impl XamlCompilerDiagnosticsFilter {
    /// `analyzer_config_files` are the EditorConfig files of the build; they
    /// are read when the first diagnostic is handled.
    pub fn new(analyzer_config_files: Option<Vec<PathBuf>>) -> Self {
        Self { analyzer_config_files, lazy_editor_config: OnceCell::new() }
    }

    /// The severity to report `diagnostic` with.
    ///
    /// # Panics
    /// Panics when an EditorConfig file that exists cannot be read (the
    /// exception of the original).
    pub fn handle(&self, diagnostic: &XamlDiagnostic) -> XamlDiagnosticSeverity {
        self.handle_code(diagnostic.severity, &diagnostic.code)
    }

    /// The severity to report a diagnostic of `current_severity` and
    /// `diagnostic_code` with.
    ///
    /// # Panics
    /// Panics when an EditorConfig file that exists cannot be read (the
    /// exception of the original).
    pub fn handle_code(&self, current_severity: XamlDiagnosticSeverity, diagnostic_code: &str) -> XamlDiagnosticSeverity {
        let editor_config =
            self.lazy_editor_config.get_or_init(|| Self::parse_editor_config_files(self.analyzer_config_files.as_deref()));

        if let Some(severity) = editor_config.get(diagnostic_code) {
            return match severity.to_lowercase().as_str() {
                "default" => current_severity,
                "error" => XamlDiagnosticSeverity::Error,
                "warning" => XamlDiagnosticSeverity::Warning,
                // "suggestion", "silent", "none"
                _ => XamlDiagnosticSeverity::None,
            };
        }

        current_severity
    }

    fn parse_editor_config_files(analyzer_config_files: Option<&[PathBuf]>) -> HashMap<String, String> {
        // Very naive EditorConfig parser, supporting minimal properties set:
        let mut severities = HashMap::new();
        if let Some(analyzer_config_files) = analyzer_config_files {
            for file_item in analyzer_config_files {
                if file_item.is_file() {
                    let file_content = fs::read(file_item)
                        .unwrap_or_else(|error| panic!("Could not read '{}': {error}", file_item.display()));
                    parse_severities(&String::from_utf8_lossy(&file_content), &mut severities);
                }
            }
        }

        severities
    }
}

/// A character of a word of the pattern (`\w`).
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// The number of bytes of the word `text` starts with.
fn word_length(text: &str) -> usize {
    text.char_indices().find(|(_, c)| !is_word(*c)).map_or(text.len(), |(index, _)| index)
}

/// Adds every severity entry of `content` to `severities`; a later entry of
/// a code replaces an earlier one.
///
/// The entries are found as the pattern of the original finds them,
/// `ferro_xaml_diagnostic\.([\w\d]+)\.severity\s*=\s*(\w*)` anywhere in the
/// text: the pattern is matched by hand, since the crate has no regular
/// expressions.
fn parse_severities(content: &str, severities: &mut HashMap<String, String>) {
    let mut rest = content;
    while let Some(start) = rest.find(KEY_PREFIX) {
        let entry = &rest[start + KEY_PREFIX.len()..];
        match match_entry(entry) {
            Some((code, severity, length)) => {
                severities.insert(code.to_string(), severity.to_string());
                rest = &entry[length..];
            }
            // No entry starts here: the search goes on after the first
            // character of the prefix.
            None => rest = &rest[start + 1..],
        }
    }
}

/// Matches what follows the prefix of an entry: the code, the suffix of the
/// key, the equals sign and the severity, which may be empty. Returns the
/// code, the severity and the number of bytes matched.
fn match_entry(entry: &str) -> Option<(&str, &str, usize)> {
    let code_length = word_length(entry);
    if code_length == 0 {
        return None;
    }
    let code = &entry[..code_length];

    let after_key = entry[code_length..].strip_prefix(KEY_SUFFIX)?;
    let after_equals = after_key.trim_start().strip_prefix('=')?;
    let value = after_equals.trim_start();
    let severity = &value[..word_length(value)];

    Some((code, severity, entry.len() - value.len() + severity.len()))
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of the class.
    use super::*;
    use std::path::Path;

    /// An EditorConfig file of a test, removed when the test ends.
    struct ConfigFile(PathBuf);

    impl ConfigFile {
        fn new(name: &str, content: &str) -> ConfigFile {
            let file = ConfigFile(Self::path(name));
            fs::write(&file.0, content).expect("the file of the test is written");
            file
        }

        fn path(name: &str) -> PathBuf {
            std::env::temp_dir().join(format!("ferroui-diagnostics-filter-{}-{name}.editorconfig", std::process::id()))
        }

        fn as_path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ConfigFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn filter(files: &[&ConfigFile]) -> XamlCompilerDiagnosticsFilter {
        XamlCompilerDiagnosticsFilter::new(Some(files.iter().map(|file| file.as_path().to_path_buf()).collect()))
    }

    #[test]
    fn a_code_without_an_entry_keeps_its_severity() {
        let file = ConfigFile::new("no-entry", "[*.xaml]\nferro_xaml_diagnostic.FRN2000.severity = error\n");
        let filter = filter(&[&file]);

        assert_eq!(XamlDiagnosticSeverity::Warning, filter.handle_code(XamlDiagnosticSeverity::Warning, "FRN2105"));
        assert_eq!(XamlDiagnosticSeverity::Fatal, filter.handle_code(XamlDiagnosticSeverity::Fatal, "FRN2106"));
    }

    #[test]
    fn the_severity_of_an_entry_replaces_the_severity_of_the_diagnostic() {
        let file = ConfigFile::new(
            "severities",
            concat!(
                "root = true\n",
                "[*.xaml]\n",
                "ferro_xaml_diagnostic.A1.severity = error\n",
                "ferro_xaml_diagnostic.A2.severity = warning\n",
                "ferro_xaml_diagnostic.A3.severity = default\n",
                "ferro_xaml_diagnostic.A4.severity = suggestion\n",
                "ferro_xaml_diagnostic.A5.severity = silent\n",
                "ferro_xaml_diagnostic.A6.severity = none\n",
                "ferro_xaml_diagnostic.A7.severity = ERROR\n",
                "ferro_xaml_diagnostic.A8.severity = something\n",
                "ferro_xaml_diagnostic.A9.severity =\n",
            ),
        );
        let filter = filter(&[&file]);
        let handle = |code: &str| filter.handle_code(XamlDiagnosticSeverity::Warning, code);

        assert_eq!(XamlDiagnosticSeverity::Error, handle("A1"));
        assert_eq!(XamlDiagnosticSeverity::Warning, filter.handle_code(XamlDiagnosticSeverity::Error, "A2"));
        assert_eq!(XamlDiagnosticSeverity::Warning, handle("A3"));
        assert_eq!(XamlDiagnosticSeverity::None, handle("A4"));
        assert_eq!(XamlDiagnosticSeverity::None, handle("A5"));
        assert_eq!(XamlDiagnosticSeverity::None, handle("A6"));
        assert_eq!(XamlDiagnosticSeverity::Error, handle("A7"));
        assert_eq!(XamlDiagnosticSeverity::None, handle("A8"));
        assert_eq!(XamlDiagnosticSeverity::None, handle("A9"));
    }

    #[test]
    fn a_diagnostic_is_handled_by_its_code_and_severity() {
        let file = ConfigFile::new("diagnostic", "ferro_xaml_diagnostic.FRN2105.severity=error");
        let filter = filter(&[&file]);

        let diagnostic = XamlDiagnostic {
            code: "FRN2105".to_string(),
            severity: XamlDiagnosticSeverity::Warning,
            title: "A warning".to_string(),
            line_number: None,
            line_position: None,
            min_severity: XamlDiagnosticSeverity::None,
            document: None,
            inner_exception: None,
        };

        assert_eq!(XamlDiagnosticSeverity::Error, filter.handle(&diagnostic));
        assert_eq!(XamlDiagnosticSeverity::Warning, filter.handle(&XamlDiagnostic { code: "FRN2000".to_string(), ..diagnostic }));
    }

    #[test]
    fn the_entries_are_found_wherever_the_pattern_matches() {
        let mut severities = HashMap::new();
        parse_severities(
            concat!(
                // Spaces, tabs and line ends around the equals sign.
                "ferro_xaml_diagnostic.B1.severity\t=   error\n",
                "ferro_xaml_diagnostic.B2.severity =\n  warning\n",
                // Not at the start of a line, and two on one line.
                "# ferro_xaml_diagnostic.B3.severity = none ferro_xaml_diagnostic.B4.severity = error\n",
                // Not entries: no code, another key, no equals sign.
                "ferro_xaml_diagnostic..severity = error\n",
                "ferro_xaml_diagnostic.B5.level = error\n",
                "ferro_xaml_diagnostic.B6.severity error\n",
                // The value ends with the word.
                "ferro_xaml_diagnostic.B_7.severity = error-ish\n",
            ),
            &mut severities,
        );

        let mut found = severities.iter().map(|(code, severity)| (code.as_str(), severity.as_str())).collect::<Vec<_>>();
        found.sort();
        assert_eq!(
            vec![("B1", "error"), ("B2", "warning"), ("B3", "none"), ("B4", "error"), ("B_7", "error")],
            found
        );
    }

    #[test]
    fn a_later_entry_of_a_code_replaces_an_earlier_one() {
        let first = ConfigFile::new(
            "first",
            "ferro_xaml_diagnostic.C1.severity = error\nferro_xaml_diagnostic.C1.severity = warning\nferro_xaml_diagnostic.C2.severity = error\n",
        );
        let second = ConfigFile::new("second", "ferro_xaml_diagnostic.C2.severity = none\n");
        let filter = filter(&[&first, &second]);

        assert_eq!(XamlDiagnosticSeverity::Warning, filter.handle_code(XamlDiagnosticSeverity::Fatal, "C1"));
        assert_eq!(XamlDiagnosticSeverity::None, filter.handle_code(XamlDiagnosticSeverity::Fatal, "C2"));
    }

    #[test]
    fn without_files_and_with_files_that_do_not_exist_every_severity_is_kept() {
        let without = XamlCompilerDiagnosticsFilter::new(None);
        assert_eq!(XamlDiagnosticSeverity::Error, without.handle_code(XamlDiagnosticSeverity::Error, "D1"));

        let missing = XamlCompilerDiagnosticsFilter::new(Some(vec![ConfigFile::path("missing")]));
        assert_eq!(XamlDiagnosticSeverity::Warning, missing.handle_code(XamlDiagnosticSeverity::Warning, "D1"));
    }

    #[test]
    fn the_files_are_read_once_when_the_first_diagnostic_is_handled() {
        let path = ConfigFile::path("lazy");
        let filter = XamlCompilerDiagnosticsFilter::new(Some(vec![path.clone()]));

        // Written after the filter was made, before it handles anything.
        let file = ConfigFile::new("lazy", "ferro_xaml_diagnostic.E1.severity = error\n");
        assert_eq!(path.as_path(), file.as_path());
        assert_eq!(XamlDiagnosticSeverity::Error, filter.handle_code(XamlDiagnosticSeverity::Warning, "E1"));

        fs::write(file.as_path(), "ferro_xaml_diagnostic.E1.severity = none\n").expect("the file is written");
        assert_eq!(XamlDiagnosticSeverity::Error, filter.handle_code(XamlDiagnosticSeverity::Warning, "E1"));
    }
}
