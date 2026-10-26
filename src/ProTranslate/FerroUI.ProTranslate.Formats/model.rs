use std::collections::{BTreeMap, BTreeSet};

/// A format supported by the catalog conversion layer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationFileFormat {
    ProTranslateJson,
    I18NextJson,
    FlutterArb,
    Resx,
    AndroidResources,
    AppleStrings,
    AppleStringsdict,
    AppleXcstrings,
    GettextPo,
    Xliff12,
    Xliff20,
    Csv,
    Tsv,
}

impl TranslationFileFormat {
    /// All supported formats, in the order of the upstream public enumeration.
    pub const ALL: [Self; 13] = [
        Self::ProTranslateJson,
        Self::I18NextJson,
        Self::FlutterArb,
        Self::Resx,
        Self::AndroidResources,
        Self::AppleStrings,
        Self::AppleStringsdict,
        Self::AppleXcstrings,
        Self::GettextPo,
        Self::Xliff12,
        Self::Xliff20,
        Self::Csv,
        Self::Tsv,
    ];
}

/// A normalized translation. Plurals use `key.category` and retain the category separately.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranslationCatalogEntry {
    pub key: String,
    pub culture: Option<String>,
    pub value: String,
    pub source: Option<String>,
    pub comment: Option<String>,
    pub context: Option<String>,
    pub state: Option<String>,
    pub plural_category: Option<String>,
    pub format: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl TranslationCatalogEntry {
    pub fn new(key: impl Into<String>, culture: Option<&str>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            culture: culture.filter(|s| !s.trim().is_empty()).map(str::to_owned),
            value: value.into(),
            ..Self::default()
        }
    }

    /// The plural group's key without its category suffix.
    pub fn base_key(&self) -> &str {
        if let Some(category) = &self.plural_category {
            self.key
                .strip_suffix(&format!(".{category}"))
                .unwrap_or(&self.key)
        } else {
            &self.key
        }
    }
}

/// An ordered collection of normalized translations and its source language.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TranslationCatalog {
    pub name: Option<String>,
    pub source_culture: Option<String>,
    pub entries: Vec<TranslationCatalogEntry>,
}

impl TranslationCatalog {
    pub fn add(
        &mut self,
        key: impl Into<String>,
        culture: Option<&str>,
        value: impl Into<String>,
    ) -> &mut TranslationCatalogEntry {
        self.entries
            .push(TranslationCatalogEntry::new(key, culture, value));
        self.entries.last_mut().expect("entry was appended")
    }

    /// Unique, case-insensitively ordered culture tags.
    pub fn cultures(&self) -> Vec<String> {
        let mut cultures = BTreeMap::new();
        for culture in self.entries.iter().filter_map(|e| e.culture.as_ref()) {
            if !culture.is_empty() {
                cultures
                    .entry(culture.to_ascii_lowercase())
                    .or_insert_with(|| culture.clone());
            }
        }
        cultures.into_values().collect()
    }

    /// Select exactly one culture, preserving the source API's last-entry-wins rule.
    pub fn to_dictionary(&self, culture: Option<&str>) -> BTreeMap<String, String> {
        self.entries
            .iter()
            .filter(|e| same_culture(e.culture.as_deref(), culture))
            .map(|e| (e.key.clone(), e.value.clone()))
            .collect()
    }

    /// Validate record identity without changing the original order or discarding duplicates.
    pub fn validate(&self) -> Vec<TranslationFormatDiagnostic> {
        let mut identities = BTreeSet::new();
        let mut diagnostics = Vec::new();
        for entry in &self.entries {
            if entry.key.trim().is_empty() {
                diagnostics.push(TranslationFormatDiagnostic::entry(
                    Severity::Error,
                    "PTF002",
                    "Translation keys must not be empty.",
                    entry,
                ));
            }
            if !identities.insert((
                entry.culture.as_deref().unwrap_or("").to_ascii_lowercase(),
                entry.key.as_str(),
            )) {
                diagnostics.push(TranslationFormatDiagnostic::entry(Severity::Warning, "PTF003", "Duplicate culture/key; dictionary and single-value exports select the last record.", entry));
            }
        }
        diagnostics
    }

    /// Normalize the usual hyphen/underscore BCP-47 spelling without assuming OS locale data.
    pub fn normalize_culture_name(culture: Option<&str>) -> String {
        let Some(culture) = culture else {
            return String::new();
        };
        culture
            .trim()
            .replace('_', "-")
            .split('-')
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    part.to_ascii_lowercase()
                } else if part.len() == 2
                    || (part.len() == 3 && part.bytes().all(|b| b.is_ascii_digit()))
                {
                    part.to_ascii_uppercase()
                } else if part.len() == 4 && part.bytes().all(|b| b.is_ascii_alphabetic()) {
                    let mut part = part.to_ascii_lowercase();
                    part[..1].make_ascii_uppercase();
                    part
                } else {
                    part.to_ascii_lowercase()
                }
            })
            .collect::<Vec<_>>()
            .join("-")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationFormatOptions {
    pub culture: Option<String>,
    pub source_culture: Option<String>,
    pub name: Option<String>,
    pub include_comments: bool,
    /// Override CSV delimiter. TSV always uses a tab.
    pub delimiter: char,
}

impl Default for TranslationFormatOptions {
    fn default() -> Self {
        Self {
            culture: None,
            source_culture: None,
            name: None,
            include_comments: true,
            delimiter: ',',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TranslationFormatDiagnosticSeverity {
    Info,
    Warning,
    Error,
}
pub type Severity = TranslationFormatDiagnosticSeverity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationFormatDiagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub key: Option<String>,
    pub culture: Option<String>,
}

impl TranslationFormatDiagnostic {
    pub fn new(severity: Severity, code: &str, message: impl Into<String>) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            key: None,
            culture: None,
        }
    }
    pub(crate) fn entry(
        severity: Severity,
        code: &str,
        message: impl Into<String>,
        entry: &TranslationCatalogEntry,
    ) -> Self {
        Self {
            key: Some(entry.key.clone()),
            culture: entry.culture.clone(),
            ..Self::new(severity, code, message)
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TranslationFormatResult {
    pub catalog: TranslationCatalog,
    pub diagnostics: Vec<TranslationFormatDiagnostic>,
}
impl TranslationFormatResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// Exported text plus diagnostics describing information the target format cannot retain.
#[derive(Clone, Debug, Default)]
pub struct TranslationExportResult {
    pub content: String,
    pub diagnostics: Vec<TranslationFormatDiagnostic>,
}
impl TranslationExportResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

pub(crate) fn same_culture(a: Option<&str>, b: Option<&str>) -> bool {
    a.unwrap_or("").eq_ignore_ascii_case(b.unwrap_or(""))
}

pub(crate) fn select_entries<'a>(
    catalog: &'a TranslationCatalog,
    culture: Option<&str>,
) -> Vec<&'a TranslationCatalogEntry> {
    let mut selected = BTreeMap::<&str, &TranslationCatalogEntry>::new();
    // Neutral values precede exact-culture values, independent of source ordering.
    for entry in &catalog.entries {
        if culture.is_none() || entry.culture.is_none() {
            selected.insert(&entry.key, entry);
        }
    }
    if let Some(culture) = culture {
        for entry in &catalog.entries {
            if same_culture(entry.culture.as_deref(), Some(culture)) {
                selected.insert(&entry.key, entry);
            }
        }
    }
    selected.into_values().collect()
}
