use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Path, PathBuf};

use ferroui_protranslate_core::{placeholder_indexes, Culture};
use ferroui_protranslate_formats::{
    TranslationCatalogConverter, TranslationFileFormat, TranslationFormatDiagnosticSeverity,
    TranslationFormatOptions,
};

use crate::{analyze_rust_source, KeyUsage, SourceAnalysisOptions};

/// Build diagnostics preserve the upstream PTA/PTSG identifiers where applicable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

/// A deterministic, source-associated catalog or usage diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub key: Option<String>,
    pub culture: Option<String>,
}

impl fmt::Display for BuildDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{file}")?;
            if let Some(line) = self.line {
                write!(f, ":{line}")?;
            }
            write!(f, ": ")?;
        }
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// One explicitly supplied catalog. `name` is a stable source label, not a discovery path.
#[derive(Clone, Debug)]
pub struct CatalogInput {
    pub name: String,
    pub content: String,
    pub format: TranslationFileFormat,
    pub culture: Option<String>,
}

impl CatalogInput {
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let name = name.into();
        let format = TranslationCatalogConverter::detect_format(&name);
        let culture = infer_culture(&name);
        Self {
            name,
            content: content.into(),
            format,
            culture,
        }
    }

    pub fn with_culture(mut self, culture: impl Into<String>) -> Self {
        self.culture = Some(culture.into());
        self
    }

    pub fn with_format(mut self, format: TranslationFileFormat) -> Self {
        self.format = format;
        self
    }
}

/// The generated Rust and manifest can be inspected without writing any files.
#[derive(Clone, Debug)]
pub struct Compilation {
    pub rust_source: String,
    pub manifest_json: String,
    pub diagnostics: Vec<BuildDiagnostic>,
}

impl Compilation {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }
}

#[derive(Clone, Debug)]
struct Entry {
    key: String,
    culture: String,
    value: String,
    file: String,
    placeholders: Vec<usize>,
    format_valid: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct KeyDeclaration {
    key: String,
    file: String,
    line: usize,
}

/// Framework-independent compiler usable in build scripts, editors and CI tools.
#[derive(Clone, Debug, Default)]
pub struct CatalogCompiler {
    catalogs: Vec<CatalogInput>,
    keys: Vec<(String, String)>,
    usages: Vec<KeyUsage>,
    source_diagnostics: Vec<BuildDiagnostic>,
    notify_properties: bool,
}

impl CatalogCompiler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Emit the native `INotifyPropertyChanged` contract for typed property bindings.
    /// The consuming crate must depend on `ferroui-system` when enabled.
    pub fn notify_properties(&mut self, enabled: bool) -> &mut Self {
        self.notify_properties = enabled;
        self
    }

    pub fn add_catalog(&mut self, catalog: CatalogInput) -> &mut Self {
        self.catalogs.push(catalog);
        self
    }

    /// Blank lines and lines beginning with `#` are ignored, like upstream key lists.
    pub fn add_key_list(
        &mut self,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> &mut Self {
        self.keys.push((name.into(), content.into()));
        self
    }

    /// Validate a resolved key reference; callers with semantic information can use this
    /// instead of the explicitly scoped Rust syntax scanner.
    pub fn add_usage(&mut self, usage: KeyUsage) -> &mut Self {
        self.usages.push(usage);
        self
    }

    /// Analyze only the receivers selected in `options`; unrelated `.format` calls
    /// are never treated as localization calls merely because the name matches.
    pub fn add_rust_source(
        &mut self,
        name: &str,
        source: &str,
        options: &SourceAnalysisOptions,
    ) -> &mut Self {
        let result = analyze_rust_source(name, source, options);
        self.usages.extend(result.usages);
        self.source_diagnostics.extend(result.diagnostics);
        self
    }

    pub fn compile(&self) -> Compilation {
        let mut diagnostics = self.source_diagnostics.clone();
        let mut entries = BTreeMap::<(String, String), Entry>::new();
        let mut keys = BTreeSet::<String>::new();
        let mut sources = BTreeSet::<String>::new();
        let mut cultures = BTreeSet::<String>::new();
        let mut key_declarations = Vec::new();
        let mut catalogs = self.catalogs.iter().collect::<Vec<_>>();
        catalogs.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.culture.cmp(&b.culture))
                .then_with(|| format!("{:?}", a.format).cmp(&format!("{:?}", b.format)))
                .then(a.content.cmp(&b.content))
        });

        for input in catalogs {
            sources.insert(input.name.clone());
            if let Some(culture) = &input.culture {
                match Culture::parse(culture) {
                    Ok(culture) => {
                        cultures.insert(culture.name().to_owned());
                    }
                    Err(error) => {
                        let mut invalid = diagnostic(
                            DiagnosticSeverity::Error,
                            "PTA005",
                            format!("Invalid culture '{culture}': {error}"),
                            &input.name,
                            None,
                        );
                        invalid.culture = Some(culture.clone());
                        diagnostics.push(invalid);
                        continue;
                    }
                }
            }
            let options = TranslationFormatOptions {
                culture: input.culture.clone(),
                ..Default::default()
            };
            let result = TranslationCatalogConverter::import(
                &input.content,
                input.format,
                &options,
                Some(&input.name),
            );
            for diagnostic in result.diagnostics {
                diagnostics.push(BuildDiagnostic {
                    severity: match diagnostic.severity {
                        TranslationFormatDiagnosticSeverity::Error => DiagnosticSeverity::Error,
                        _ => DiagnosticSeverity::Warning,
                    },
                    code: diagnostic.code,
                    message: diagnostic.message,
                    file: Some(input.name.clone()),
                    line: None,
                    key: diagnostic.key,
                    culture: diagnostic.culture.or_else(|| input.culture.clone()),
                });
            }
            for entry in result.catalog.entries {
                let culture_name = entry.culture.as_deref().unwrap_or("");
                let culture = match Culture::parse(culture_name) {
                    Ok(culture) => culture.name().to_owned(),
                    Err(error) => {
                        diagnostics.push(diagnostic(
                            DiagnosticSeverity::Error,
                            "PTA005",
                            format!("Invalid culture '{culture_name}': {error}"),
                            &input.name,
                            Some(&entry.key),
                        ));
                        continue;
                    }
                };
                cultures.insert(culture.clone());
                if entry.key.trim().is_empty() {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        "PTA005",
                        "Translation keys must not be empty".into(),
                        &input.name,
                        None,
                    ));
                    continue;
                }
                keys.insert(entry.key.clone());
                let (placeholders, format_valid) = match placeholder_indexes(&entry.value) {
                    Ok(indexes) => (indexes, true),
                    Err(error) => {
                        diagnostics.push(diagnostic(
                            DiagnosticSeverity::Warning,
                            "PTSG003",
                            format!("'{}' is not a composite format: {error}", entry.key),
                            &input.name,
                            Some(&entry.key),
                        ));
                        (Vec::new(), false)
                    }
                };
                let identity = (culture.clone(), entry.key.clone());
                if let Some(previous) = entries.get(&identity) {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        "PTSG002",
                        format!(
                            "Duplicate translation '{}' for culture '{}' (first in '{}')",
                            entry.key, culture, previous.file
                        ),
                        &input.name,
                        Some(&entry.key),
                    ));
                    continue;
                }
                entries.insert(
                    identity,
                    Entry {
                        key: entry.key,
                        culture,
                        value: entry.value,
                        file: input.name.clone(),
                        placeholders,
                        format_valid,
                    },
                );
            }
        }

        for (file, content) in &self.keys {
            sources.insert(file.clone());
            let mut seen = BTreeSet::new();
            for (line, key) in content
                .strip_prefix('\u{feff}')
                .unwrap_or(content)
                .lines()
                .map(str::trim)
                .enumerate()
            {
                if key.is_empty() || key.starts_with('#') {
                    continue;
                }
                key_declarations.push(KeyDeclaration {
                    key: key.to_owned(),
                    file: file.clone(),
                    line: line + 1,
                });
                if !seen.insert(key) {
                    let mut d = diagnostic(
                        DiagnosticSeverity::Warning,
                        "PTSG002",
                        format!("Duplicate key declaration '{key}'"),
                        file,
                        Some(key),
                    );
                    d.line = Some(line + 1);
                    diagnostics.push(d);
                }
                keys.insert(key.to_owned());
            }
        }
        key_declarations.sort();

        let mut arities = BTreeMap::<String, usize>::new();
        let mut placeholder_sets = BTreeMap::<String, Vec<usize>>::new();
        let mut unknown_arities = BTreeSet::new();
        for entry in entries.values() {
            if !entry.format_valid {
                unknown_arities.insert(entry.key.clone());
                continue;
            }
            let arity = entry.placeholders.last().map_or(0, |i| i + 1);
            arities.entry(entry.key.clone()).or_insert(arity);
            if let Some(previous) = placeholder_sets.get(&entry.key) {
                if previous != &entry.placeholders {
                    diagnostics.push(diagnostic(
                        DiagnosticSeverity::Error,
                        "PTA002",
                        format!(
                            "Translation '{}' has placeholders {:?} in '{}', expected {:?}",
                            entry.key, entry.placeholders, entry.culture, previous
                        ),
                        &entry.file,
                        Some(&entry.key),
                    ));
                }
            } else {
                placeholder_sets.insert(entry.key.clone(), entry.placeholders.clone());
            }
        }
        for key in unknown_arities {
            arities.remove(&key);
        }
        for culture in &cultures {
            for key in &keys {
                if !entries.contains_key(&(culture.clone(), key.clone())) {
                    diagnostics.push(BuildDiagnostic {
                        severity: DiagnosticSeverity::Warning,
                        code: "PTA003".into(),
                        message: format!("Culture '{culture}' is missing translation '{key}'"),
                        file: None,
                        line: None,
                        key: Some(key.clone()),
                        culture: Some(culture.clone()),
                    });
                }
            }
        }
        let identifiers = identifiers(&keys);
        let constants = identifiers
            .iter()
            .map(|(key, name)| (name.to_ascii_uppercase(), key.clone()))
            .collect::<BTreeMap<_, _>>();
        for usage in &self.usages {
            let key = usage.key.as_ref().or_else(|| {
                usage
                    .generated_constant
                    .as_ref()
                    .and_then(|name| constants.get(name))
            });
            let (code, message) = if let Some(key) = key {
                if !keys.contains(key) {
                    ("PTA001", format!("Missing static translation key '{key}'"))
                } else if let (Some(expected), Some(actual)) =
                    (arities.get(key), usage.argument_count)
                {
                    if *expected == actual {
                        continue;
                    }
                    (
                        "PTA002",
                        format!("Translation '{key}' expects {expected} arguments, got {actual}"),
                    )
                } else {
                    continue;
                }
            } else if let Some(constant) = &usage.generated_constant {
                (
                    "PTA001",
                    format!(
                        "Missing generated translation constant 'ProTranslateKeys::{constant}'"
                    ),
                )
            } else {
                (
                    "PTA004",
                    "Dynamic translation key cannot be validated at build time".into(),
                )
            };
            diagnostics.push(BuildDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: code.into(),
                message,
                file: Some(usage.file.clone()),
                line: usage.line,
                key: key.cloned(),
                culture: None,
            });
        }
        diagnostics.sort_by(|a, b| {
            (&a.file, a.line, &a.code, &a.key, &a.culture, &a.message)
                .cmp(&(&b.file, b.line, &b.code, &b.key, &b.culture, &b.message))
        });
        let manifest = serde_json::json!({
            "schema_version": 1,
            "provider": "ProTranslate.Generated",
            "notify_properties": self.notify_properties,
            "keys": keys,
            "cultures": cultures,
            "source_files": sources,
            "key_declarations": key_declarations.iter().map(|declaration| serde_json::json!({
                "key": declaration.key,
                "source_file": declaration.file,
                "line": declaration.line,
            })).collect::<Vec<_>>(),
            "entries": entries.values().map(|e| serde_json::json!({
                "key": e.key, "culture": e.culture, "source_file": e.file,
                "placeholder_indexes": e.placeholders,
                "format_valid": e.format_valid,
            })).collect::<Vec<_>>()
        });
        Compilation {
            rust_source: render(
                &entries,
                &identifiers,
                &arities,
                &sources,
                &cultures,
                &key_declarations,
                self.notify_properties,
            ),
            manifest_json: serde_json::to_string_pretty(&manifest).expect("serializable manifest")
                + "\n",
            diagnostics,
        }
    }
}

fn diagnostic(
    severity: DiagnosticSeverity,
    code: &str,
    message: String,
    file: &str,
    key: Option<&str>,
) -> BuildDiagnostic {
    BuildDiagnostic {
        severity,
        code: code.into(),
        message,
        file: Some(file.into()),
        line: None,
        key: key.map(str::to_owned),
        culture: None,
    }
}

fn identifiers(keys: &BTreeSet<String>) -> BTreeMap<String, String> {
    let mut used = BTreeSet::new();
    keys.iter()
        .map(|key| {
            let mut name = String::new();
            for c in key.chars() {
                if c.is_ascii_alphanumeric() {
                    name.push(c.to_ascii_lowercase());
                } else if !name.ends_with('_') {
                    name.push('_');
                }
            }
            let mut name = name.trim_matches('_').to_owned();
            if name.is_empty() {
                name = "key".into();
            }
            if name.starts_with(|c: char| c.is_ascii_digit()) {
                name.insert_str(0, "key_");
            }
            // Every emitted method has a prefix. Constants are uppercase, so Rust
            // keywords are harmless except `Self`, which uppercases to `SELF`.
            let base = name.clone();
            let mut suffix = 2;
            while !used.insert(name.clone()) {
                name = format!("{base}_{suffix}");
                suffix += 1;
            }
            (key.clone(), name)
        })
        .collect()
}

fn render(
    entries: &BTreeMap<(String, String), Entry>,
    identifiers: &BTreeMap<String, String>,
    arities: &BTreeMap<String, usize>,
    sources: &BTreeSet<String>,
    cultures: &BTreeSet<String>,
    key_declarations: &[KeyDeclaration],
    notify_properties: bool,
) -> String {
    let mut out = String::from(
        "// Generated by ferroui-protranslate-build. Do not edit.\n\
         #[derive(Clone, Copy, Debug, Default)]\n\
         pub struct ProTranslateKeys;\n\
         impl ProTranslateKeys {\n",
    );
    for (key, identifier) in identifiers {
        writeln!(
            out,
            "    pub const {}: &'static str = {key:?};",
            identifier.to_ascii_uppercase()
        )
        .unwrap();
    }
    out.push_str(
        "}\n\n#[derive(Clone, Copy, Debug, Default)]\npub struct GeneratedTranslationProvider;\n",
    );
    out.push_str("impl ::ferroui_protranslate_core::TranslationProvider for GeneratedTranslationProvider {\n");
    out.push_str("    fn name(&self) -> &str { \"ProTranslate.Generated\" }\n");
    out.push_str("    fn get_string(&self, key: &str, culture: &::ferroui_protranslate_core::Culture) -> Result<::ferroui_protranslate_core::LocalizedString, ::ferroui_protranslate_core::TranslationError> {\n");
    out.push_str("        ::ferroui_protranslate_core::validate_translation_key(key)?;\n        let identity = (culture.name(), key);\n        Ok(match PROTRANSLATE_ENTRIES.binary_search_by(|entry| (entry.0, entry.1).cmp(&identity)).or_else(|_| PROTRANSLATE_ENTRIES.binary_search_by(|entry| (entry.0, entry.1).cmp(&(\"\", key)))) {\n");
    out.push_str("            Ok(index) => ::ferroui_protranslate_core::LocalizedString::found(key, PROTRANSLATE_ENTRIES[index].2, culture.clone(), self.name()),\n");
    out.push_str("            Err(_) => ::ferroui_protranslate_core::LocalizedString::missing(key, culture.clone(), self.name()),\n        })\n    }\n}\n\n");
    out.push_str("pub static PROTRANSLATE_ENTRIES: &[(&str, &str, &str)] = &[\n");
    for entry in entries.values() {
        writeln!(
            out,
            "    ({:?}, {:?}, {:?}),",
            entry.culture, entry.key, entry.value
        )
        .unwrap();
    }
    out.push_str("];\npub static PROTRANSLATE_SOURCE_FILES: &[&str] = &[\n");
    for source in sources {
        writeln!(out, "    {source:?},").unwrap();
    }
    out.push_str("];\n\n");
    out.push_str(r#"#[derive(Clone, Debug)]
pub struct NamedGeneratedTranslationProvider { name: String }
impl GeneratedTranslationProvider {
    pub fn with_name(name: impl Into<String>) -> Result<NamedGeneratedTranslationProvider, ::ferroui_protranslate_core::TranslationError> {
        let name = name.into();
        if name.trim().is_empty() { return Err(::ferroui_protranslate_core::TranslationError::InvalidProviderName); }
        Ok(NamedGeneratedTranslationProvider { name })
    }
}
impl ::ferroui_protranslate_core::TranslationProvider for NamedGeneratedTranslationProvider {
    fn name(&self) -> &str { &self.name }
    fn get_string(&self, key: &str, culture: &::ferroui_protranslate_core::Culture) -> Result<::ferroui_protranslate_core::LocalizedString, ::ferroui_protranslate_core::TranslationError> {
        let mut value = ::ferroui_protranslate_core::TranslationProvider::get_string(&GeneratedTranslationProvider, key, culture)?;
        value.provider_name = self.name.clone();
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProTranslateProviderManifestEntry {
    pub key: &'static str,
    pub culture: Option<&'static str>,
    pub source_file: &'static str,
    pub placeholder_indexes: &'static [usize],
    pub format_valid: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProTranslateKeyDeclaration {
    pub key: &'static str,
    pub source_file: &'static str,
    pub line: usize,
}
pub struct ProTranslateProviderManifest;
impl ProTranslateProviderManifest {
    pub const SOURCE_FILES: &'static [&'static str] = PROTRANSLATE_SOURCE_FILES;
    pub const KEYS: &'static [&'static str] = &[
"#);
    for key in identifiers.keys() {
        writeln!(out, "        {key:?},").unwrap();
    }
    out.push_str("    ];\n    pub const CULTURES: &'static [&'static str] = &[\n");
    for culture in cultures {
        writeln!(out, "        {culture:?},").unwrap();
    }
    out.push_str(
        "    ];\n    pub const ENTRIES: &'static [ProTranslateProviderManifestEntry] = &[\n",
    );
    for entry in entries.values() {
        let culture = (!entry.culture.is_empty()).then_some(entry.culture.as_str());
        writeln!(out, "        ProTranslateProviderManifestEntry {{ key: {:?}, culture: {:?}, source_file: {:?}, placeholder_indexes: &{:?}, format_valid: {} }},", entry.key, culture, entry.file, entry.placeholders, entry.format_valid).unwrap();
    }
    out.push_str(
        "    ];\n    pub const KEY_DECLARATIONS: &'static [ProTranslateKeyDeclaration] = &[\n",
    );
    for declaration in key_declarations {
        writeln!(
            out,
            "        ProTranslateKeyDeclaration {{ key: {:?}, source_file: {:?}, line: {} }},",
            declaration.key, declaration.file, declaration.line
        )
        .unwrap();
    }
    out.push_str("    ];\n}\n\n");
    if notify_properties {
        out.push_str(r#"struct ProTranslateStringsState {
    translations: ::ferroui_protranslate_core::TranslationService,
    changed: ::ferroui_system::events::Event<str>,
    _subscription: ::ferroui_protranslate_core::Subscription,
}
#[derive(Clone)]
pub struct ProTranslateStrings { state: ::std::rc::Rc<ProTranslateStringsState> }
impl ::core::cmp::PartialEq for ProTranslateStrings {
    fn eq(&self, other: &Self) -> bool { ::std::rc::Rc::ptr_eq(&self.state, &other.state) }
}
impl ::core::cmp::Eq for ProTranslateStrings {}
impl ::ferroui_system::component_model::INotifyPropertyChanged for ProTranslateStrings {
    fn property_changed(&self) -> &::ferroui_system::events::Event<str> { &self.state.changed }
}
impl ProTranslateStrings {
    pub fn new(translations: ::ferroui_protranslate_core::TranslationService) -> Self {
        let state = ::std::rc::Rc::new_cyclic(|weak: &::std::rc::Weak<ProTranslateStringsState>| {
            let weak = weak.clone();
            let subscription = translations.subscribe(move |_| {
                if let Some(state) = weak.upgrade() { state.changed.raise(""); }
            });
            ProTranslateStringsState { translations, changed: ::ferroui_system::events::Event::new(), _subscription: subscription }
        });
        Self { state }
    }
    pub fn translations(&self) -> &::ferroui_protranslate_core::TranslationService { &self.state.translations }
    pub fn property_changed(&self) -> &::ferroui_system::events::Event<str> { &self.state.changed }
    pub fn refresh(&self) { self.state.changed.raise(""); }
"#);
    } else {
        out.push_str("#[derive(Clone)]\npub struct ProTranslateStrings { translations: ::ferroui_protranslate_core::TranslationService }\n");
        out.push_str("impl ProTranslateStrings {\n    pub fn new(translations: ::ferroui_protranslate_core::TranslationService) -> Self { Self { translations } }\n");
        out.push_str("    pub fn translations(&self) -> &::ferroui_protranslate_core::TranslationService { &self.translations }\n");
    }
    out.push_str("    pub fn culture(&self) -> ::ferroui_protranslate_core::Culture { self.translations().current_culture() }\n");
    out.push_str("    pub fn ui_culture(&self) -> ::ferroui_protranslate_core::Culture { self.translations().current_ui_culture() }\n");
    render_accessor_methods(&mut out, identifiers, arities, "pub ");
    out.push_str("}\n\npub trait ProTranslateAccessors {\n    fn translations(&self) -> &::ferroui_protranslate_core::TranslationService;\n");
    render_accessor_methods(&mut out, identifiers, arities, "");
    out.push_str("}\nimpl ProTranslateAccessors for ::ferroui_protranslate_core::TranslationService {\n    fn translations(&self) -> &::ferroui_protranslate_core::TranslationService { self }\n}\n");
    out
}

fn render_accessor_methods(
    out: &mut String,
    identifiers: &BTreeMap<String, String>,
    arities: &BTreeMap<String, usize>,
    visibility: &str,
) {
    for (key, identifier) in identifiers {
        let arity = arities.get(key).copied();
        writeln!(out, "    {visibility}fn get_{identifier}(&self) -> Result<::ferroui_protranslate_core::LocalizedString, ::ferroui_protranslate_core::TranslationError> {{ self.translations().get_string({key:?}) }}").unwrap();
        writeln!(out, "    {visibility}fn value_{identifier}(&self) -> Result<String, ::ferroui_protranslate_core::TranslationError> {{ self.get_{identifier}().map(|value| value.value) }}").unwrap();
        if let Some(arity) = arity {
            writeln!(out, "    {visibility}fn format_{identifier}(&self, arguments: [::ferroui_protranslate_core::FormatArgument; {arity}]) -> Result<String, ::ferroui_protranslate_core::TranslationError> {{ self.translations().format({key:?}, &arguments) }}").unwrap();
            writeln!(out, "    {visibility}fn observe_{identifier}(&self, arguments: [::ferroui_protranslate_core::FormatArgument; {arity}]) -> ::ferroui_protranslate_core::ObservableLocalizedString {{ self.translations().observe({key:?}, arguments.into()) }}").unwrap();
        } else {
            // Key-only inputs have no format contract. A zero-length array would
            // incorrectly forbid formatting a value supplied by another provider.
            writeln!(out, "    {visibility}fn format_{identifier}(&self, arguments: &[::ferroui_protranslate_core::FormatArgument]) -> Result<String, ::ferroui_protranslate_core::TranslationError> {{ self.translations().format({key:?}, arguments) }}").unwrap();
            writeln!(out, "    {visibility}fn observe_{identifier}(&self, arguments: Vec<::ferroui_protranslate_core::FormatArgument>) -> ::ferroui_protranslate_core::ObservableLocalizedString {{ self.translations().observe({key:?}, arguments) }}").unwrap();
        }
    }
}

fn infer_culture(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let parent = normalized
        .rsplit_once('/')
        .and_then(|(parent, _)| parent.rsplit('/').next());
    let format = TranslationCatalogConverter::detect_format(path);
    if let Some(parent) = parent {
        if format == TranslationFileFormat::AndroidResources {
            if let Some(qualifiers) = parent.strip_prefix("values-") {
                // Android configuration qualifiers can follow the locale; they are
                // not part of BCP-47. Other configuration-only folders are neutral.
                let parts: Vec<_> = qualifiers.split('-').collect();
                if let Some(bcp47) = parts.iter().find_map(|part| part.strip_prefix("b+")) {
                    return Some(bcp47.replace('+', "-"));
                }
                if let Some((index, language)) = parts.iter().enumerate().find(|(_, part)| {
                    (part.len() == 2 || part.len() == 3)
                        && part.bytes().all(|c| c.is_ascii_alphabetic())
                        && !matches!(**part, "car" | "desk")
                }) {
                    let mut culture = (*language).to_owned();
                    let mut next = index + 1;
                    if let Some(script) = parts.get(next).filter(|part| is_android_script(part)) {
                        culture.push('-');
                        culture.push_str(script);
                        next += 1;
                    }
                    if let Some(region) = parts
                        .get(next)
                        .and_then(|part| part.strip_prefix(['r', 'R']))
                    {
                        if (region.len() == 2 && region.bytes().all(|c| c.is_ascii_alphabetic()))
                            || (region.len() == 3 && region.bytes().all(|c| c.is_ascii_digit()))
                        {
                            culture.push('-');
                            culture.push_str(region);
                        }
                    }
                    return Some(culture);
                }
            }
        }
        if matches!(
            format,
            TranslationFileFormat::AppleStrings | TranslationFileFormat::AppleStringsdict
        ) {
            if let Some(culture) = parent.strip_suffix(".lproj") {
                return Some(if culture.eq_ignore_ascii_case("Base") {
                    String::new()
                } else {
                    culture.replace('_', "-")
                });
            }
        }
    }
    let name = path.rsplit(['/', '\\']).next()?;
    let lower = name.to_ascii_lowercase();
    let stem = [
        ".protranslate.json",
        ".i18next.json",
        ".i18n.json",
        ".strings.xml",
    ]
    .into_iter()
    .find_map(|suffix| {
        lower
            .ends_with(suffix)
            .then(|| &name[..name.len() - suffix.len()])
    })
    .or_else(|| name.rsplit_once('.').map(|(stem, _)| stem))?;
    if stem
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Strings."))
    {
        // This is an explicit culture-bearing naming convention. Keep an invalid
        // tag so compile reports PTA005 instead of silently creating neutral data.
        return Some(stem[8..].into());
    }
    let candidate = stem.rsplit_once('.')?.1;
    Culture::parse(candidate)
        .ok()
        .map(|culture| culture.name().to_owned())
}

fn is_android_script(value: &str) -> bool {
    // Legacy catalog paths may spell out an ISO 15924 script before the region.
    // Four-letter Android configuration qualifiers are not script subtags.
    value.len() == 4
        && value
            .bytes()
            .all(|character| character.is_ascii_alphabetic())
        && !matches!(
            value.to_ascii_lowercase().as_str(),
            "land" | "port" | "long" | "desk" | "dpad" | "ldpi" | "mdpi" | "hdpi"
        )
}

/// Build-script failure, including all source diagnostics when generation was rejected.
#[derive(Debug)]
pub enum BuildError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    MissingOutputDirectory,
    Diagnostics(Vec<BuildDiagnostic>),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "{}: {source}", path.display()),
            Self::MissingOutputDirectory => write!(
                f,
                "OUT_DIR is not set; use generate_to for an explicit directory"
            ),
            Self::Diagnostics(diagnostics) => {
                writeln!(f, "ProTranslate catalog generation failed:")?;
                for diagnostic in diagnostics {
                    writeln!(f, "{diagnostic}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Explicit format and culture overrides for one build-script catalog file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogFileOptions {
    /// Overrides filename inference. An empty string selects the neutral catalog.
    pub culture: Option<String>,
    /// Overrides extension detection for catalogs with application-specific names.
    pub format: Option<TranslationFileFormat>,
}

/// Cargo build-script entry point. Files are explicit and tracked with
/// `rerun-if-changed`. Only changed output bytes are written.
#[derive(Clone, Debug, Default)]
pub struct Builder {
    catalogs: Vec<(PathBuf, CatalogFileOptions)>,
    sources: Vec<(PathBuf, SourceAnalysisOptions)>,
    deny_warnings: bool,
    notify_properties: bool,
}

impl Builder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn catalog(mut self, path: impl Into<PathBuf>) -> Self {
        self.catalogs
            .push((path.into(), CatalogFileOptions::default()));
        self
    }

    pub fn catalog_with_options(
        mut self,
        path: impl Into<PathBuf>,
        options: CatalogFileOptions,
    ) -> Self {
        self.catalogs.push((path.into(), options));
        self
    }

    pub fn catalog_for_culture(self, path: impl Into<PathBuf>, culture: impl Into<String>) -> Self {
        self.catalog_with_options(
            path,
            CatalogFileOptions {
                culture: Some(culture.into()),
                ..Default::default()
            },
        )
    }

    pub fn source(mut self, path: impl Into<PathBuf>, options: SourceAnalysisOptions) -> Self {
        self.sources.push((path.into(), options));
        self
    }

    /// Generate native property notifications. Add `ferroui-system` to the
    /// consuming crate; the default generated provider requires only the core.
    pub fn notify_properties(mut self, enabled: bool) -> Self {
        self.notify_properties = enabled;
        self
    }

    pub fn deny_warnings(mut self, enabled: bool) -> Self {
        self.deny_warnings = enabled;
        self
    }

    pub fn generate(self) -> Result<Compilation, BuildError> {
        let out = std::env::var_os("OUT_DIR").ok_or(BuildError::MissingOutputDirectory)?;
        self.generate_to(Path::new(&out))
    }

    pub fn generate_to(self, directory: &Path) -> Result<Compilation, BuildError> {
        let mut compiler = CatalogCompiler::new();
        compiler.notify_properties(self.notify_properties);
        let mut paths = self.catalogs;
        paths.sort_by(|a, b| a.0.cmp(&b.0));
        paths.dedup();
        for (path, options) in paths {
            let name = tracked_name(&path)?;
            let content = read(&path)?;
            println!("cargo:rerun-if-changed={name}");
            if name.ends_with(".protranslate.keys.txt") {
                compiler.add_key_list(name, content);
            } else {
                let mut input = CatalogInput::new(name, content);
                if let Some(culture) = options.culture {
                    input.culture = Some(culture);
                }
                if let Some(format) = options.format {
                    input.format = format;
                }
                compiler.add_catalog(input);
            }
        }
        for (path, options) in self.sources {
            let name = tracked_name(&path)?;
            let content = read(&path)?;
            println!("cargo:rerun-if-changed={name}");
            compiler.add_rust_source(&name, &content, &options);
        }
        let result = compiler.compile();
        for diagnostic in &result.diagnostics {
            // Cargo protocol directives must occupy one physical line.
            println!(
                "cargo:warning={}",
                diagnostic.to_string().replace(['\n', '\r'], " ")
            );
        }
        if result.has_errors() || (self.deny_warnings && !result.diagnostics.is_empty()) {
            return Err(BuildError::Diagnostics(result.diagnostics));
        }
        fs::create_dir_all(directory).map_err(|source| BuildError::Io {
            path: directory.to_owned(),
            source,
        })?;
        write_if_changed(&directory.join("protranslate.rs"), &result.rust_source)?;
        write_if_changed(
            &directory.join("protranslate.manifest.json"),
            &result.manifest_json,
        )?;
        Ok(result)
    }
}

fn tracked_name(path: &Path) -> Result<String, BuildError> {
    let name = path.to_str().ok_or_else(|| BuildError::Io {
        path: path.to_owned(),
        source: std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cargo dependency paths must be UTF-8",
        ),
    })?;
    let name = if cfg!(windows) {
        name.replace('\\', "/")
    } else {
        name.to_owned()
    };
    if name.contains(['\n', '\r']) {
        return Err(BuildError::Io {
            path: path.to_owned(),
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cargo dependency paths must not contain newlines",
            ),
        });
    }
    Ok(name)
}

fn read(path: &Path) -> Result<String, BuildError> {
    fs::read_to_string(path).map_err(|source| BuildError::Io {
        path: path.to_owned(),
        source,
    })
}

fn write_if_changed(path: &Path, content: &str) -> Result<(), BuildError> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(());
    }
    fs::write(path, content).map_err(|source| BuildError::Io {
        path: path.to_owned(),
        source,
    })
}
