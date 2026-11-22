use crate::{
    read_json,
    regex_source::{RegexCache, RegexInput},
    Error, FilenameGlob, GrammarConfiguration,
    GrammarContribution, GrammarDefinition, IRegistryOptions, LanguageSnippets, RawGrammar,
    Registry, Result, Theme,
};
use serde_json::Value;
use std::{cmp::Reverse, collections::BTreeMap, fmt, rc::Rc};

#[cfg(any(feature = "bundled-grammars", feature = "editor-demo-grammars"))]
use ferroui_textmate_grammars::RESOURCES;
#[cfg(not(any(feature = "bundled-grammars", feature = "editor-demo-grammars")))]
static RESOURCES: &[(&str, &str)] = &[];

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

/// A TextMate extension package supplied as UTF-8 by the application.
pub struct LanguagePackage<'a> {
    /// Package directory name under `grammars/`.
    pub name: &'a str,
    /// The package.json contents.
    pub manifest: &'a str,
    /// Package-relative paths and the text of each resource.
    pub resources: &'a [(&'a str, &'a str)],
}

/// Bundled and host-supplied grammar packages, themes, language configuration
/// and snippets.
pub struct BundledCatalog {
    languages: Vec<Language>,
    grammar_paths: Vec<String>,
    packages: Vec<(String, GrammarDefinition)>,
    grammar_contributions: Vec<GrammarContribution>,
    package_resources: BTreeMap<String, BTreeMap<String, String>>,
    default_theme: ThemeName,
    filename_patterns: Vec<(usize, FilenameGlob)>,
    detection_cache: RegexCache,
}
impl BundledCatalog {
    pub fn new() -> Result<Self> {
        Self::with_default_theme(ThemeName::DarkPlus)
    }
    pub fn with_default_theme(default_theme: ThemeName) -> Result<Self> {
        Self::with_packages(default_theme, &[])
    }

    /// Builds a catalog from the bundle and the supplied packages. Supplying
    /// an existing package directory replaces that package. Package names and
    /// paths are case-sensitive. Each contributed scope must be unique when
    /// supplied by a host package. Grammar JSON is parsed on registry load.
    pub fn with_packages(default_theme: ThemeName, supplied: &[LanguagePackage<'_>]) -> Result<Self> {
        let mut languages = Vec::new();
        let mut grammar_paths = Vec::new();
        let mut packages = Vec::new();
        let mut grammar_contributions = Vec::new();
        let mut manifests = BTreeMap::new();
        for (path, source) in RESOURCES {
            if let Some(name) = path
                .strip_prefix("grammars/")
                .and_then(|path| path.strip_suffix("/package.json"))
            {
                manifests.insert(name, *source);
            }
        }
        let mut package_resources = BTreeMap::new();
        for package in supplied {
            if package.name.trim().is_empty()
                || matches!(package.name, "." | "..")
                || package.name.contains(['/', '\\', ':', '\0'])
            {
                return Err(Error::InvalidPackage(format!(
                    "invalid package name {:?}", package.name,
                )));
            }
            let directory = format!("grammars/{}", package.name);
            let mut resources = BTreeMap::new();
            for &(path, source) in package.resources {
                let path = package_path(&directory, path)?;
                if resources.insert(path.clone(), source.to_owned()).is_some() {
                    return Err(Error::InvalidPackage(format!(
                        "duplicate resource {directory}/{path}",
                    )));
                }
            }
            if package_resources.insert(package.name.to_owned(), resources).is_some() {
                return Err(Error::InvalidPackage(format!("duplicate package {directory}")));
            }
            manifests.insert(package.name, package.manifest);
        }
        let mut scopes = BTreeMap::new();
        for (name, source) in manifests {
            let package = read_json(source)?;
            let directory = format!("grammars/{name}");
            let from_host = package_resources.contains_key(name);
            if !from_host && package.get("contributes").is_none() {
                continue;
            }
            let definition: GrammarDefinition =
                serde_json::from_value(package).map_err(|error| Error::Json(error.to_string()))?;
            let contributes = &definition.contributes;
            let resolve = |path: &str| {
                if from_host {
                    package_path(&directory, path).map(|path| format!("{directory}/{path}"))
                } else {
                    Ok(normalize(&format!("{directory}/{path}")))
                }
            };
            for grammar in &contributes.grammars {
                let mut grammar = grammar.clone();
                grammar.path = resolve(&grammar.path)?;
                if scopes
                    .insert(grammar.scope_name.clone(), from_host)
                    .is_some_and(|other_from_host| from_host || other_from_host)
                {
                    return Err(Error::InvalidPackage(format!(
                        "duplicate grammar scope {}", grammar.scope_name,
                    )));
                }
                if from_host {
                    grammar_resource(&package_resources, &grammar.path)?;
                }
                grammar_paths.push(grammar.path.clone());
                grammar_contributions.push(grammar);
            }
            for entry in &contributes.languages {
                let scope_name = contributes
                    .grammars
                    .iter()
                    .find(|grammar| grammar.language.as_deref() == Some(entry.id.as_str()))
                    .map(|grammar| grammar.scope_name.clone());
                let configuration_file = entry
                    .configuration_file
                    .as_deref()
                    .filter(|path| !path.is_empty())
                    .map(&resolve)
                    .transpose()?;
                let configuration = configuration_file
                    .as_deref()
                    .map(|path| {
                        LanguageConfiguration::from_json(grammar_resource(&package_resources, path)?)
                    })
                    .transpose()?;
                let mut snippets = BTreeMap::new();
                for snippet in contributes
                    .snippets
                    .iter()
                    .filter(|snippet| snippet.language == entry.id)
                {
                    let path = resolve(&snippet.path)?;
                    if let Value::Object(entries) =
                        read_json(grammar_resource(&package_resources, &path)?)?
                    {
                        snippets.extend(entries);
                    }
                }
                languages.push(Language {
                    id: entry.id.clone(),
                    aliases: entry.aliases.clone(),
                    extensions: entry.extensions.clone(),
                    filenames: entry.filenames.clone(),
                    filename_patterns: entry.filename_patterns.clone(),
                    first_line: entry.first_line.clone(),
                    mime_types: entry.mime_types.clone(),
                    configuration_file,
                    scope_name,
                    configuration,
                    snippets,
                });
            }
            packages.push((directory, definition));
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
            package_resources,
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
    /// case consistently on every host. Ties use language-ID order, so a shared
    /// extension has the same default as in [`Self::language_by_extension`].
    /// All patterns are checked and compiled when the catalog is built.
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
                    .flat_map(|language| {
                        language
                            .extensions
                            .iter()
                            .map(move |extension| (language, extension))
                    })
                    .filter_map(|(language, extension)| {
                        let start = name.len().checked_sub(extension.len())?;
                        name.get(start..)?
                            .eq_ignore_ascii_case(extension)
                            .then_some((language, extension.len()))
                    })
                    .min_by_key(|(_, length)| Reverse(*length))
                    .map(|(language, _)| language)
            })
    }
    /// Detects shebangs and other upstream first-line declarations using the
    /// active regex backend. Browser callers initialize Oniguruma first.
    pub fn language_by_first_line(&self, line: &str) -> Result<Option<&Language>> {
        let input = RegexInput::new(line);
        for language in &self.languages {
            if let Some(pattern) = &language.first_line {
                if self
                    .detection_cache
                    .search(pattern, &input, 0, true, false)?
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
            registry.add_grammar(self.read_grammar(path)?);
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

    fn read_grammar(&self, path: &str) -> Result<RawGrammar> {
        let grammar = RawGrammar::from_json(grammar_resource(&self.package_resources, path)?)?;
        if package_resource_parts(path).is_some_and(|(name, _)| self.package_resources.contains_key(name))
        {
            if let Some(declared) = self
                .grammar_contributions
                .iter()
                .find(|entry| entry.path == path && entry.scope_name != grammar.scope_name)
            {
                return Err(Error::InvalidPackage(format!(
                    "{path} declares scope {} but contains {}",
                    declared.scope_name, grammar.scope_name,
                )));
            }
        }
        Ok(grammar)
    }
}
impl IRegistryOptions for BundledCatalog {
    fn get_grammar(&self, scope_name: &str) -> Result<Option<RawGrammar>> {
        self.grammar_by_scope(scope_name)
            .map(|grammar| self.read_grammar(&grammar.path))
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
fn package_path(directory: &str, path: &str) -> Result<String> {
    let invalid = || Error::InvalidPackage(format!("invalid resource path {path:?} in {directory}"));
    if path.starts_with(['/', '\\']) || path.contains([':', '\0']) {
        return Err(invalid());
    }
    let mut parts = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "." | "" => {}
            ".." if parts.pop().is_none() => return Err(invalid()),
            ".." => {}
            _ => parts.push(part),
        }
    }
    if parts.is_empty() {
        return Err(invalid());
    }
    Ok(parts.join("/"))
}

fn grammar_resource<'a>(
    packages: &'a BTreeMap<String, BTreeMap<String, String>>,
    path: &str,
) -> Result<&'a str> {
    let custom = package_resource_parts(path)
        .and_then(|(name, relative)| packages.get(name).map(|resources| (resources, relative)));
    if let Some((resources, relative)) = custom {
        return resources
            .get(relative)
            .map(String::as_str)
            .ok_or_else(|| Error::MissingResource(path.into()));
    }
    BundledCatalog::resource(path)
}

fn package_resource_parts(path: &str) -> Option<(&str, &str)> {
    path.strip_prefix("grammars/")?.split_once('/')
}
