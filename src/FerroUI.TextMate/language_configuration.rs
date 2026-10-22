use crate::{read_json, regex_source::RegexCache, Error, LanguageConfiguration, Result};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::collections::BTreeMap;

/// A language-configuration expression and its declared flags. Raw package
/// settings remain available in LanguageConfiguration for forward compatibility.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegexPattern {
    pub pattern: String,
    pub flags: String,
}
impl<'de> Deserialize<'de> for RegexPattern {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            String(String),
            Object {
                pattern: String,
                #[serde(default)]
                flags: String,
            },
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::String(pattern) => Self {
                pattern,
                flags: String::new(),
            },
            Raw::Object { pattern, flags } => Self { pattern, flags },
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Comments {
    pub line_comment: Option<String>,
    pub block_comment: Option<(String, String)>,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct Markers {
    pub start: Option<RegexPattern>,
    pub end: Option<RegexPattern>,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Folding {
    #[serde(default)]
    pub off_side: bool,
    pub markers: Option<Markers>,
}
impl Folding {
    pub fn is_empty(&self) -> bool {
        self.markers.as_ref().is_none_or(|markers| {
            markers.start.as_ref().is_none_or(|p| p.pattern.is_empty())
                || markers.end.as_ref().is_none_or(|p| p.pattern.is_empty())
        })
    }
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Indentation {
    #[serde(rename = "increaseIndentPattern")]
    pub increase: Option<RegexPattern>,
    #[serde(rename = "decreaseIndentPattern")]
    pub decrease: Option<RegexPattern>,
    #[serde(rename = "unIndentedLinePattern")]
    pub unindent: Option<RegexPattern>,
    pub indent_next_line_pattern: Option<RegexPattern>,
}
impl Indentation {
    pub fn is_empty(&self) -> bool {
        self.increase.as_ref().is_none_or(|p| p.pattern.is_empty())
            || self.decrease.as_ref().is_none_or(|p| p.pattern.is_empty())
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IndentAction {
    #[default]
    None,
    Indent,
    Outdent,
    IndentOutdent,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterAction {
    #[serde(default)]
    pub indent: IndentAction,
    pub append_text: Option<String>,
    pub remove_text: Option<usize>,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EnterRule {
    pub before_text: Option<RegexPattern>,
    pub after_text: Option<RegexPattern>,
    pub previous_line_text: Option<RegexPattern>,
    #[serde(default)]
    pub action: EnterAction,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoPair {
    pub open: String,
    pub close: String,
    pub not_in: Vec<String>,
}
impl<'de> Deserialize<'de> for AutoPair {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Pair((String, String)),
            Object {
                open: String,
                close: String,
                #[serde(default, rename = "notIn")]
                not_in: Vec<String>,
            },
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Pair((open, close)) => Self {
                open,
                close,
                not_in: Vec::new(),
            },
            Raw::Object {
                open,
                close,
                not_in,
            } => Self {
                open,
                close,
                not_in,
            },
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoClosingPairs {
    pub pairs: Vec<AutoPair>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LanguageSnippet {
    pub prefixes: Vec<String>,
    pub body: Vec<String>,
    pub description: Option<String>,
    pub scopes: Vec<String>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LanguageSnippets {
    pub snippets: BTreeMap<String, LanguageSnippet>,
}
impl LanguageSnippets {
    pub fn from_json(source: &str) -> Result<Self> {
        Self::from_value(&read_json(source)?)
    }
    pub fn from_value(raw: &Value) -> Result<Self> {
        let entries = raw
            .as_object()
            .ok_or_else(|| Error::Json("snippets must be an object".into()))?;
        Ok(Self {
            snippets: entries
                .iter()
                .map(|(name, value)| {
                    (
                        name.clone(),
                        LanguageSnippet {
                            prefixes: strings(value.get("prefix")),
                            body: strings(value.get("body")),
                            description: value
                                .get("description")
                                .and_then(Value::as_str)
                                .map(str::to_owned),
                            scopes: strings(value.get("scope"))
                                .into_iter()
                                .flat_map(|scopes| {
                                    scopes
                                        .split(',')
                                        .map(|scope| scope.trim().to_owned())
                                        .filter(|scope| !scope.is_empty())
                                        .collect::<Vec<_>>()
                                })
                                .collect(),
                        },
                    )
                })
                .collect(),
        })
    }
}
fn strings(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}
fn optional<T: serde::de::DeserializeOwned>(value: Option<&Value>) -> Result<Option<T>> {
    value
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value(value.clone()).map_err(|e| Error::Json(e.to_string())))
        .transpose()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WordRange {
    pub start_index: usize,
    pub end_index: usize,
}

impl LanguageConfiguration {
    pub fn auto_close_before(&self) -> Option<&str> {
        self.raw.get("autoCloseBefore")?.as_str()
    }
    pub fn comments(&self) -> Result<Option<Comments>> {
        optional(self.raw.get("comments"))
    }
    pub fn folding(&self) -> Result<Option<Folding>> {
        optional(self.raw.get("folding"))
    }
    pub fn indentation_rules(&self) -> Result<Option<Indentation>> {
        optional(self.raw.get("indentationRules"))
    }
    pub fn on_enter_rules(&self) -> Result<Vec<EnterRule>> {
        Ok(optional(self.raw.get("onEnterRules"))?.unwrap_or_default())
    }
    pub fn auto_closing_pairs(&self) -> Result<AutoClosingPairs> {
        Ok(AutoClosingPairs {
            pairs: optional(self.raw.get("autoClosingPairs"))?.unwrap_or_default(),
        })
    }
    pub fn surrounding_pairs(&self) -> Result<Vec<AutoPair>> {
        Ok(optional(self.raw.get("surroundingPairs"))?.unwrap_or_default())
    }
    pub fn word_regex(&self) -> Result<Option<RegexPattern>> {
        optional(self.raw.get("wordPattern"))
    }

    /// Returns the configured word at a UTF-16 caret offset. If the package
    /// has no wordPattern, the host keeps its normal word-navigation policy.
    /// Expressions run on one physical line through the active regex backend.
    pub fn word_at_offset(&self, line: &str, offset: usize) -> Result<Option<WordRange>> {
        if offset > line.encode_utf16().count() {
            return Err(Error::InvalidOffset(offset));
        }
        let Some(expression) = self.word_regex()? else {
            return Ok(None);
        };
        if expression
            .flags
            .chars()
            .any(|flag| !matches!(flag, 'g' | 'i' | 'm' | 's' | 'u' | 'y'))
        {
            return Err(Error::Regex {
                pattern: expression.pattern,
                message: "unsupported language-configuration regex flag".into(),
            });
        }
        // Oniguruma's m option enables dot-all; anchors already operate on
        // physical line boundaries. g/u are implemented by iteration/UTF-8.
        let options = format!(
            "{}{}",
            if expression.flags.contains('i') {
                "i"
            } else {
                ""
            },
            if expression.flags.contains('s') {
                "m"
            } else {
                ""
            }
        );
        let pattern = if options.is_empty() {
            expression.pattern
        } else {
            format!("(?{options}:{})", expression.pattern)
        };
        let cache = RegexCache::default();
        let mut cursor = 0;
        for _ in 0..100_000 {
            let Some(captures) = cache.search(&pattern, line, cursor, true, true)? else {
                return Ok(None);
            };
            let Some((start, end)) = captures.first().copied().flatten() else {
                return Ok(None);
            };
            if expression.flags.contains('y') && start != cursor {
                return Ok(None);
            }
            let start_index = line[..start].encode_utf16().count();
            let end_index = start_index + line[start..end].encode_utf16().count();
            if start != end && start_index <= offset && offset <= end_index {
                return Ok(Some(WordRange {
                    start_index,
                    end_index,
                }));
            }
            if start_index > offset || end >= line.len() {
                return Ok(None);
            }
            cursor = if end > cursor {
                end
            } else {
                cursor + line[cursor..].chars().next().map_or(0, char::len_utf8)
            };
        }
        Err(Error::RegexLimit(
            "word lookup exceeded its match budget".into(),
        ))
    }
}
