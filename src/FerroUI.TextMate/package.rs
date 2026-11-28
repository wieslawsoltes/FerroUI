use crate::{read_json, Error, GrammarConfiguration, Result, ScopeSelector, StandardTokenType};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Engines {
    pub vscode: Option<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Scripts {
    #[serde(rename = "update-grammar")]
    pub update_grammar: Option<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// Package source metadata. Both the npm string shorthand and object form are
/// accepted; unknown object fields remain available to the host.
#[derive(Clone, Debug, Default)]
pub struct Repository {
    pub kind: Option<String>,
    pub url: String,
    pub other: BTreeMap<String, Value>,
}
impl<'de> Deserialize<'de> for Repository {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Url(String),
            Object {
                #[serde(rename = "type")]
                kind: Option<String>,
                #[serde(default)]
                url: String,
                #[serde(flatten)]
                other: BTreeMap<String, Value>,
            },
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Url(url) => Self {
                url,
                ..Self::default()
            },
            Raw::Object { kind, url, other } => Self { kind, url, other },
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageContribution {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    #[serde(default)]
    pub filename_patterns: Vec<String>,
    pub first_line: Option<String>,
    #[serde(rename = "configuration")]
    pub configuration_file: Option<String>,
    #[serde(default, rename = "mimetypes")]
    pub mime_types: Vec<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarContribution {
    pub language: Option<String>,
    pub scope_name: String,
    pub path: String,
    #[serde(default)]
    pub inject_to: Vec<String>,
    #[serde(default)]
    pub embedded_languages: BTreeMap<String, String>,
    #[serde(default)]
    pub token_types: BTreeMap<String, String>,
    #[serde(default)]
    pub balanced_bracket_scopes: Vec<String>,
    #[serde(default)]
    pub unbalanced_bracket_scopes: Vec<String>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}
impl GrammarContribution {
    /// Converts contributed scope settings using the host's language-ID table.
    /// Unknown host languages retain ID zero or are omitted from overrides.
    pub fn configuration(
        &self,
        language_id: impl Fn(&str) -> Option<u8>,
    ) -> Result<GrammarConfiguration> {
        let mut configuration = GrammarConfiguration {
            initial_language: self.language.as_deref().and_then(&language_id).unwrap_or(0),
            embedded_languages: self
                .embedded_languages
                .iter()
                .filter_map(|(scope, language)| language_id(language).map(|id| (scope.clone(), id)))
                .collect(),
            ..GrammarConfiguration::default()
        };
        for (selector, token_type) in &self.token_types {
            let token_type = match token_type.as_str() {
                "other" => StandardTokenType::Other,
                "comment" => StandardTokenType::Comment,
                "string" => StandardTokenType::String,
                "regex" => StandardTokenType::RegEx,
                unknown => {
                    return Err(Error::InvalidGrammar(format!(
                        "unknown contributed token type: {unknown}"
                    )))
                }
            };
            for selector in ScopeSelector::parse_many(selector)? {
                configuration.token_types.push((selector, token_type));
            }
        }
        for selector in &self.balanced_bracket_scopes {
            configuration
                .balanced_bracket_selectors
                .extend(ScopeSelector::parse_many(selector)?);
        }
        for selector in &self.unbalanced_bracket_scopes {
            configuration
                .unbalanced_bracket_selectors
                .extend(ScopeSelector::parse_many(selector)?);
        }
        Ok(configuration)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SnippetContribution {
    pub language: String,
    pub path: String,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Contributes {
    #[serde(default)]
    pub languages: Vec<LanguageContribution>,
    #[serde(default)]
    pub grammars: Vec<GrammarContribution>,
    #[serde(default)]
    pub snippets: Vec<SnippetContribution>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}

/// Typed package.json metadata with lossless storage for fields that do not
/// participate in grammar execution. Resource resolution remains a host task.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarDefinition {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub license: Option<String>,
    pub engines: Option<Engines>,
    pub scripts: Option<Scripts>,
    #[serde(default)]
    pub contributes: Contributes,
    pub repository: Option<Repository>,
    #[serde(flatten)]
    pub other: BTreeMap<String, Value>,
}
impl GrammarDefinition {
    pub fn from_json(source: &str) -> Result<Self> {
        serde_json::from_value(read_json(source)?).map_err(|error| Error::Json(error.to_string()))
    }
}
