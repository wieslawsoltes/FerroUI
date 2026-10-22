use crate::{
    read_json, regex_source::RegexCache, Error, FilenameGlob, GrammarConfiguration,
    GrammarContribution, GrammarDefinition, IRegistryOptions, LanguageSnippets, RawGrammar,
    Registry, Result, Theme,
};
use serde_json::Value;
use std::{collections::BTreeMap, fmt, rc::Rc};

include!(concat!(env!("OUT_DIR"), "/resources.rs"));

#[derive(Clone, Debug)]
pub struct LanguageConfiguration {
    pub raw: Value,
}
impl LanguageConfiguration {
    pub fn from_json(source: &str) -> Result<Self> {
        Ok(Self {
            raw: read_json(source)?,
        })
    }
    pub fn line_comment(&self) -> Option<&str> {
        self.raw.get("comments")?.get("lineComment")?.as_str()
    }
    pub fn block_comment(&self) -> Option<(&str, &str)> {
        let pair = self.raw.get("comments")?.get("blockComment")?.as_array()?;
        Some((pair.first()?.as_str()?, pair.get(1)?.as_str()?))
    }
    pub fn brackets(&self) -> Vec<(&str, &str)> {
        self.raw
            .get("brackets")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|entry| Some((entry.get(0)?.as_str()?, entry.get(1)?.as_str()?)))
            .collect()
    }
    pub fn word_pattern(&self) -> Option<&str> {
        let value = self.raw.get("wordPattern")?;
        value.as_str().or_else(|| value.get("pattern")?.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct Language {
    pub id: String,
    pub aliases: Vec<String>,
    pub extensions: Vec<String>,
    pub filenames: Vec<String>,
    pub filename_patterns: Vec<String>,
    pub first_line: Option<String>,
    pub mime_types: Vec<String>,
    pub configuration_file: Option<String>,
    pub scope_name: Option<String>,
    pub configuration: Option<LanguageConfiguration>,
    /// Snippet definitions retain prefix, body, description, and scope fields.
    pub snippets: BTreeMap<String, Value>,
}
impl Language {
    pub fn typed_snippets(&self) -> Result<LanguageSnippets> {
        LanguageSnippets::from_value(&Value::Object(
            self.snippets
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
        ))
    }
}
impl fmt::Display for Language {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(alias) = self.aliases.first() {
            write!(formatter, "{alias} ({})", self.id)
        } else {
            formatter.write_str(&self.id)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeName {
    Abyss,
    Dark,
    DarkPlus,
    DimmedMonokai,
    KimbieDark,
    Light,
    LightPlus,
    OneDark,
    Monokai,
    QuietLight,
    Red,
    SolarizedDark,
    SolarizedLight,
    TomorrowNightBlue,
    HighContrastDark,
    HighContrastLight,
    Dracula,
    AtomOneDark,
    AtomOneLight,
    VisualStudioDark,
    VisualStudioLight,
}

impl ThemeName {
    pub const ALL: &'static [Self] = &[
        Self::Abyss,
        Self::Dark,
        Self::DarkPlus,
        Self::DimmedMonokai,
        Self::KimbieDark,
        Self::Light,
        Self::LightPlus,
        Self::OneDark,
        Self::Monokai,
        Self::QuietLight,
        Self::Red,
        Self::SolarizedDark,
        Self::SolarizedLight,
        Self::TomorrowNightBlue,
        Self::HighContrastDark,
        Self::HighContrastLight,
        Self::Dracula,
        Self::AtomOneDark,
        Self::AtomOneLight,
        Self::VisualStudioDark,
        Self::VisualStudioLight,
    ];
    pub fn resource_name(self) -> &'static str {
        match self {
            Self::Abyss => "abyss-color-theme.json",
            Self::Dark => "dark_vs.json",
            Self::DarkPlus => "dark_plus.json",
            Self::DimmedMonokai => "dimmed-monokai-color-theme.json",
            Self::KimbieDark => "kimbie-dark-color-theme.json",
            Self::Light => "light_vs.json",
            Self::LightPlus => "light_plus.json",
            Self::OneDark => "onedark-color-theme.json",
            Self::Monokai => "monokai-color-theme.json",
            Self::QuietLight => "quietlight-color-theme.json",
            Self::Red => "Red-color-theme.json",
            Self::SolarizedDark => "solarized-dark-color-theme.json",
            Self::SolarizedLight => "solarized-light-color-theme.json",
            Self::TomorrowNightBlue => "tomorrow-night-blue-color-theme.json",
            Self::HighContrastDark => "hc_black.json",
            Self::HighContrastLight => "hc_light.json",
            Self::Dracula => "dracula-color-theme.json",
            Self::AtomOneDark => "atom-one-dark-color-theme.json",
            Self::AtomOneLight => "atom-one-light-color-theme.json",
            Self::VisualStudioDark => "visual-studio-dark-theme.json",
            Self::VisualStudioLight => "visual-studio-light-theme.json",
        }
    }
}

/// Embedded grammar packages, themes, language configuration and snippets.
pub struct BundledCatalog {
    languages: Vec<Language>,
    grammar_paths: Vec<String>,
    packages: Vec<(String, GrammarDefinition)>,
    grammar_contributions: Vec<GrammarContribution>,
    default_theme: ThemeName,
    filename_patterns: Vec<(usize, FilenameGlob)>,
    detection_cache: RegexCache,
}
impl BundledCatalog {
    pub fn new() -> Result<Self> {
        Self::with_default_theme(ThemeName::DarkPlus)
    }
    pub fn with_default_theme(default_theme: ThemeName) -> Result<Self> {
        let mut languages = Vec::new();
        let mut grammar_paths = Vec::new();
        let mut packages = Vec::new();
        let mut grammar_contributions = Vec::new();
        for (path, source) in RESOURCES {
            if !path.starts_with("grammars/") || !path.ends_with("/package.json") {
                continue;
            }
            let package = read_json(source)?;
            let Some(contributes) = package.get("contributes") else {
                continue;
            };
            let directory = path.rsplit_once('/').map_or("", |(p, _)| p);
            let definition = GrammarDefinition::from_json(source)?;
            for grammar in &definition.contributes.grammars {
                let mut grammar = grammar.clone();
                grammar.path = normalize(&format!("{directory}/{}", grammar.path));
                grammar_contributions.push(grammar);
            }
            packages.push((directory.to_owned(), definition));
            let grammars = contributes.get("grammars").and_then(Value::as_array);
            if let Some(grammars) = grammars {
                for grammar in grammars {
                    if let Some(path) = grammar.get("path").and_then(Value::as_str) {
                        grammar_paths.push(normalize(&format!("{directory}/{path}")));
                    }
                }
            }
            if let Some(entries) = contributes.get("languages").and_then(Value::as_array) {
                for entry in entries {
                    let Some(id) = entry.get("id").and_then(Value::as_str) else {
                        continue;
                    };
                    let scope_name = grammars
                        .into_iter()
                        .flatten()
                        .find(|g| g.get("language").and_then(Value::as_str) == Some(id))
                        .and_then(|g| g.get("scopeName"))
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                    let configuration = entry
                        .get("configuration")
                        .and_then(Value::as_str)
                        .map(|name| {
                            Self::resource(&format!("{directory}/{name}"))
                                .and_then(LanguageConfiguration::from_json)
                        })
                        .transpose()?;
                    let mut snippets = BTreeMap::new();
                    for snippet in contributes
                        .get("snippets")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                    {
                        if snippet.get("language").and_then(Value::as_str) != Some(id) {
                            continue;
                        }
                        if let Some(path) = snippet.get("path").and_then(Value::as_str) {
                            if let Some(entries) =
                                read_json(Self::resource(&format!("{directory}/{path}"))?)?
                                    .as_object()
                            {
                                snippets
                                    .extend(entries.iter().map(|(k, v)| (k.clone(), v.clone())));
                            }
                        }
                    }
                    languages.push(Language {
                        id: id.into(),
                        aliases: strings(entry.get("aliases")),
                        extensions: strings(entry.get("extensions")),
                        filenames: strings(entry.get("filenames")),
                        filename_patterns: strings(entry.get("filenamePatterns")),
                        first_line: entry
                            .get("firstLine")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                        mime_types: strings(entry.get("mimetypes")),
                        configuration_file: entry
                            .get("configuration")
                            .and_then(Value::as_str)
                            .map(|name| normalize(&format!("{directory}/{name}"))),
                        scope_name,
                        configuration,
                        snippets,
                    });
                }
            }
        }
        languages.sort_by(|a, b| a.id.cmp(&b.id));
        let mut filename_patterns = Vec::new();
        for (index, language) in languages.iter().enumerate() {
            for pattern in &language.filename_patterns {
                let glob =
                    FilenameGlob::new(pattern).map_err(|error| Error::InvalidFilenamePattern {
                        pattern: pattern.clone(),
                        error,
                    })?;
                filename_patterns.push((index, glob));
            }
        }
        // Stable sorting preserves language-ID order for equally long patterns.
        filename_patterns
            .sort_by(|(_, left), (_, right)| right.pattern().len().cmp(&left.pattern().len()));
        grammar_paths.sort();
        grammar_paths.dedup();
        Ok(Self {
            languages,
            grammar_paths,
            packages,
            grammar_contributions,
            default_theme,
            filename_patterns,
            detection_cache: RegexCache::default(),
        })
    }
    pub fn languages(&self) -> &[Language] {
        &self.languages
    }
    pub fn packages(&self) -> impl Iterator<Item = (&str, &GrammarDefinition)> {
        self.packages
            .iter()
            .map(|(directory, package)| (directory.as_str(), package))
    }
    pub fn grammar_contributions(&self) -> &[GrammarContribution] {
        &self.grammar_contributions
    }
    pub fn grammar_by_scope(&self, scope: &str) -> Option<&GrammarContribution> {
        self.grammar_contributions
            .iter()
            .find(|grammar| grammar.scope_name == scope)
    }
    pub fn grammar_configuration(
        &self,
        scope: &str,
        language_id: impl Fn(&str) -> Option<u8>,
    ) -> Result<GrammarConfiguration> {
        self.grammar_by_scope(scope)
            .ok_or_else(|| Error::MissingGrammar(scope.into()))?
            .configuration(language_id)
    }
    pub fn language_by_alias(&self, alias: &str) -> Option<&Language> {
        self.languages.iter().find(|language| {
            language.id.eq_ignore_ascii_case(alias)
                || language
                    .aliases
                    .iter()
                    .any(|name| name.eq_ignore_ascii_case(alias))
        })
    }
    pub fn language_by_mime_type(&self, mime_type: &str) -> Option<&Language> {
        self.languages.iter().find(|language| {
            language
                .mime_types
                .iter()
                .any(|mime| mime.eq_ignore_ascii_case(mime_type))
        })
    }
    pub fn language_by_id(&self, id: &str) -> Option<&Language> {
        self.languages
            .iter()
            .find(|l| l.id.eq_ignore_ascii_case(id))
    }
    pub fn language_by_extension(&self, extension: &str) -> Option<&Language> {
        let extension = if extension.starts_with('.') {
            extension.to_owned()
        } else {
            format!(".{extension}")
        };
        self.languages.iter().find(|l| {
            l.extensions
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&extension))
        })
    }
    /// Exact basenames take priority, followed by the longest matching filename
    /// pattern and then the longest extension. Patterns with `/` inspect the
    /// full path; other patterns inspect the basename. Comparisons fold ASCII
    /// case consistently on every host. Equally long patterns use language-ID
    /// order. All patterns are checked and compiled when the catalog is built.
    pub fn language_by_filename(&self, filename: &str) -> Option<&Language> {
        let name = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
        self.languages
            .iter()
            .find(|l| l.filenames.iter().any(|f| f.eq_ignore_ascii_case(name)))
            .or_else(|| {
                self.filename_patterns.iter().find_map(|(index, pattern)| {
                    let candidate = if pattern.has_path_separator() {
                        filename
                    } else {
                        name
                    };
                    pattern
                        .is_match_ignore_ascii_case(candidate)
                        .then(|| &self.languages[*index])
                })
            })
            .or_else(|| {
                self.languages
                    .iter()
                    .filter_map(|l| {
                        l.extensions
                            .iter()
                            .filter(|extension| {
                                name.to_ascii_lowercase()
                                    .ends_with(&extension.to_ascii_lowercase())
                            })
                            .map(|extension| (extension.len(), l))
                            .max_by_key(|(length, _)| *length)
                    })
                    .max_by_key(|(length, _)| *length)
                    .map(|(_, l)| l)
            })
    }
    /// Detects shebangs and other upstream first-line declarations using the
    /// active regex backend. Browser callers initialize Oniguruma first.
    pub fn language_by_first_line(&self, line: &str) -> Result<Option<&Language>> {
        for language in &self.languages {
            if let Some(pattern) = &language.first_line {
                if self
                    .detection_cache
                    .search(pattern, line, 0, true, false)?
                    .is_some()
                {
                    return Ok(Some(language));
                }
            }
        }
        Ok(None)
    }
    pub fn scope_by_extension(&self, extension: &str) -> Option<&str> {
        self.language_by_extension(extension)?.scope_name.as_deref()
    }
    pub fn registry(&self) -> Result<Registry> {
        let mut registry = Registry::new();
        for path in &self.grammar_paths {
            registry.add_grammar_json(Self::resource(path)?)?;
        }
        for grammar in &self.grammar_contributions {
            registry.set_injections(
                grammar.scope_name.clone(),
                self.get_injections(&grammar.scope_name)?,
            );
        }
        Ok(registry)
    }
    /// Creates a lazy registry backed by this catalog's resources. The selected
    /// default theme and declared injectTo dependencies are loaded on demand.
    pub fn lazy_registry(self: Rc<Self>) -> Result<Registry> {
        Registry::with_options(self)
    }
    pub fn theme(&self, theme: ThemeName) -> Result<Theme> {
        let source = Self::resource(&format!("themes/{}", theme.resource_name()))?;
        Theme::from_json_with_resolver(source, &|name| {
            Ok(Self::resource(&format!("themes/{name}"))?.into())
        })
    }
    pub fn resource(path: &str) -> Result<&'static str> {
        let path = normalize(path);
        RESOURCES
            .binary_search_by_key(&path.as_str(), |(name, _)| *name)
            .map(|index| RESOURCES[index].1)
            .map_err(|_| Error::MissingResource(path))
    }
    pub fn resource_names() -> impl Iterator<Item = &'static str> {
        RESOURCES.iter().map(|(name, _)| *name)
    }
}
impl IRegistryOptions for BundledCatalog {
    fn get_grammar(&self, scope_name: &str) -> Result<Option<RawGrammar>> {
        self.grammar_by_scope(scope_name)
            .map(|grammar| RawGrammar::from_json(Self::resource(&grammar.path)?))
            .transpose()
    }
    fn get_injections(&self, scope_name: &str) -> Result<Vec<String>> {
        Ok(self
            .grammar_contributions
            .iter()
            .filter(|grammar| grammar.inject_to.iter().any(|target| target == scope_name))
            .map(|grammar| grammar.scope_name.clone())
            .collect())
    }
    fn get_theme(&self, name: &str) -> Result<Option<String>> {
        let path = if name.starts_with("themes/") {
            name.to_owned()
        } else {
            format!("themes/{name}")
        };
        match Self::resource(&path) {
            Ok(source) => Ok(Some(source.into())),
            Err(Error::MissingResource(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }
    fn get_default_theme(&self) -> Result<Option<String>> {
        self.get_theme(self.default_theme.resource_name())
    }
}
fn normalize(path: &str) -> String {
    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}
fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
