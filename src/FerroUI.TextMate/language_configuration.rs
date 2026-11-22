use crate::{read_json, Error, LanguageConfiguration, Result, StandardTokenType};
use corvus_regex_ecma262::{Flags, Regex};
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// A language-configuration expression and its declared flags. Raw package
/// settings remain available in LanguageConfiguration for forward compatibility.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
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

/// A folding marker on a physical line. The opening match ends at a UTF-16 offset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldMarker {
    Start { end: usize },
    End,
}

/// The document language's indentation and explicit marker folding rules.
#[derive(Default)]
pub struct FoldingRules {
    pub off_side: bool,
    markers: Option<(Regex, Regex)>,
}

impl FoldingRules {
    pub fn marker(&self, line: &[u16]) -> Result<Option<FoldMarker>> {
        let Some((start, end)) = &self.markers else {
            return Ok(None);
        };
        let limit = |error: corvus_regex_ecma262::MatchError| Error::RegexLimit(error.to_string());
        let opening = start.find_utf16(line).map_err(limit)?;
        if start.flags() != end.flags() {
            if let Some(matched) = opening {
                return Ok(Some(FoldMarker::Start { end: matched.end() }));
            }
        }
        let closing = end.find_utf16(line).map_err(limit)?;
        Ok(match (opening, closing) {
            (Some(open), close) if close.as_ref().is_none_or(|close| open.start() <= close.start()) => {
                Some(FoldMarker::Start { end: open.end() })
            }
            (_, Some(_)) => Some(FoldMarker::End),
            _ => None,
        })
    }
}

/// Cached tests for the four indentation expressions contributed by a language.
pub struct IndentationRules {
    rules: Indentation,
    cache: ConfigurationRegexes,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IndentationMatch {
    pub increase: bool,
    pub decrease: bool,
    pub next_line: bool,
    pub ignore: bool,
}

impl IndentationRules {
    fn matches(&self, pattern: &Option<RegexPattern>, text: &str) -> Result<bool> {
        pattern
            .as_ref()
            .map_or(Ok(false), |pattern| pattern.is_match(&self.cache, text))
    }

    pub fn should_decrease(&self, text: &str) -> Result<bool> {
        Ok(self.matches(&self.rules.decrease, text)?
            && !self.matches(&self.rules.unindent, text)?)
    }

    pub fn should_increase(&self, text: &str) -> Result<bool> {
        Ok(self.matches(&self.rules.increase, text)?
            && !self.matches(&self.rules.unindent, text)?)
    }

    /// Ignored lines do not influence indentation, even if another rule matches.
    pub fn classify(&self, text: &str) -> Result<IndentationMatch> {
        if self.matches(&self.rules.unindent, text)? {
            return Ok(IndentationMatch {
                ignore: true,
                ..IndentationMatch::default()
            });
        }
        Ok(IndentationMatch {
            increase: self.matches(&self.rules.increase, text)?,
            decrease: self.matches(&self.rules.decrease, text)?,
            next_line: self.matches(&self.rules.indent_next_line_pattern, text)?,
            ignore: false,
        })
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

/// Ordered Enter rules for a language, with compiled configuration expressions.
#[derive(Default)]
pub struct EnterRules {
    rules: Vec<EnterRule>,
    cache: ConfigurationRegexes,
}

impl EnterRules {
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Returns the first action whose supplied line expressions all match.
    pub fn action(
        &self,
        before: &str,
        after: &str,
        previous: &str,
    ) -> Result<Option<&EnterAction>> {
        for rule in &self.rules {
            if self.matches(rule.before_text.as_ref(), before)?
                && self.matches(rule.after_text.as_ref(), after)?
                && self.matches(rule.previous_line_text.as_ref(), previous)?
            {
                return Ok(Some(&rule.action));
            }
        }
        Ok(None)
    }

    fn matches(&self, expression: Option<&RegexPattern>, text: &str) -> Result<bool> {
        let Some(expression) = expression else {
            return Ok(true);
        };
        expression.is_match(&self.cache, text)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoPair {
    pub open: String,
    pub close: String,
    pub not_in: Vec<String>,
    pub before_text: Option<RegexPattern>,
}
impl AutoPair {
    pub fn allows_token_type(&self, token_type: StandardTokenType) -> bool {
        let name = match token_type {
            StandardTokenType::Other => return true,
            StandardTokenType::Comment => "comment",
            StandardTokenType::String => "string",
            StandardTokenType::RegEx => "regex",
        };
        !self.not_in.iter().any(|exclude| exclude == name)
    }
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
                #[serde(rename = "beforeText")]
                before_text: Option<RegexPattern>,
            },
        }
        Ok(match Raw::deserialize(deserializer)? {
            Raw::Pair((open, close)) => Self {
                open,
                close,
                not_in: Vec::new(),
                before_text: None,
            },
            Raw::Object {
                open,
                close,
                not_in,
                before_text,
            } => Self {
                open,
                close,
                not_in,
                before_text,
            },
        })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AutoClosingPairs {
    pub pairs: Vec<AutoPair>,
}

/// Effective pair rules for keyboard input, including the package's fallbacks.
pub struct LanguagePairs {
    pub auto_closing: Vec<AutoPair>,
    pub surrounding: Vec<AutoPair>,
    auto_close_before: Option<String>,
    cache: ConfigurationRegexes,
}

impl LanguagePairs {
    pub fn is_empty(&self) -> bool {
        self.auto_closing.is_empty() && self.surrounding.is_empty()
    }

    /// Selects the longest opening sequence completed at every caret.
    pub fn opening_pair(&self, typed: &str, prefixes: &[&str]) -> Result<Option<&AutoPair>> {
        if typed.is_empty() || prefixes.is_empty() {
            return Ok(None);
        }
        let mut matched: Option<&AutoPair> = None;
        for pair in &self.auto_closing {
            if matched.is_some_and(|matched| matched.open.len() >= pair.open.len()) {
                continue;
            }
            let mut applies = true;
            for prefix in prefixes {
                if !self.matches_opener(pair, typed, prefix)? {
                    applies = false;
                    break;
                }
            }
            if applies {
                matched = Some(pair);
            }
        }
        Ok(matched)
    }

    /// Checks a configured opener against one prospective caret prefix.
    pub fn matches_opener(&self, pair: &AutoPair, typed: &str, prefix: &str) -> Result<bool> {
        if !pair.open.strip_suffix(typed).is_some_and(|open| prefix.ends_with(open)) {
            return Ok(false);
        }
        match &pair.before_text {
            Some(pattern) => pattern.is_match(&self.cache, &format!("{prefix}{typed}")),
            None => Ok(true),
        }
    }

    pub fn allows_following(&self, after: &str, quote: bool) -> bool {
        let Some(next) = after.chars().next() else {
            return true;
        };
        let allowed = self.auto_close_before.as_deref().unwrap_or(if quote {
            ";:.,=}])> \n\t"
        } else {
            "'\"`;:.,=}])> \n\t"
        });
        allowed.contains(next)
            || (!self
                .auto_closing
                .iter()
                .any(|pair| after.starts_with(&pair.open))
                && self
                    .auto_closing
                    .iter()
                    .any(|pair| after.starts_with(&pair.close)))
    }
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

    pub fn folding_rules(&self) -> Result<FoldingRules> {
        let folding = self.folding()?.unwrap_or_default();
        let markers = folding
            .markers
            .and_then(|markers| markers.start.zip(markers.end))
            .filter(|(start, end)| !start.pattern.is_empty() && !end.pattern.is_empty())
            .map(|(start, end)| Ok((start.compile()?, end.compile()?)))
            .transpose()?;
        Ok(FoldingRules {
            off_side: folding.off_side,
            markers,
        })
    }
    pub fn indentation_rules(&self) -> Result<Option<Indentation>> {
        optional(self.raw.get("indentationRules"))
    }

    pub fn editing_indentation_rules(&self) -> Result<Option<IndentationRules>> {
        let Some(rules) = self.indentation_rules()? else {
            return Ok(None);
        };
        let mut patterns = [
            &rules.increase,
            &rules.decrease,
            &rules.unindent,
            &rules.indent_next_line_pattern,
        ]
        .into_iter()
        .flatten()
        .peekable();
        if patterns.peek().is_none() {
            return Ok(None);
        }
        let cache = ConfigurationRegexes::default();
        for pattern in patterns {
            cache.get(pattern)?;
        }
        Ok(Some(IndentationRules { rules, cache }))
    }
    pub fn on_enter_rules(&self) -> Result<Vec<EnterRule>> {
        Ok(optional(self.raw.get("onEnterRules"))?.unwrap_or_default())
    }
    pub fn enter_rules(&self) -> Result<EnterRules> {
        let rules = self.on_enter_rules()?;
        let cache = ConfigurationRegexes::default();
        for rule in &rules {
            for pattern in [
                &rule.before_text,
                &rule.after_text,
                &rule.previous_line_text,
            ]
            .into_iter()
            .flatten()
            {
                cache.get(pattern)?;
            }
        }
        Ok(EnterRules { rules, cache })
    }
    pub fn auto_closing_pairs(&self) -> Result<AutoClosingPairs> {
        Ok(AutoClosingPairs {
            pairs: optional(self.raw.get("autoClosingPairs"))?.unwrap_or_default(),
        })
    }
    pub fn surrounding_pairs(&self) -> Result<Vec<AutoPair>> {
        Ok(optional(self.raw.get("surroundingPairs"))?.unwrap_or_default())
    }
    pub fn language_pairs(&self) -> Result<LanguagePairs> {
        let mut auto_closing = optional::<Vec<AutoPair>>(self.raw.get("autoClosingPairs"))?
            .unwrap_or_else(|| {
                self.brackets()
                    .into_iter()
                    .map(|(open, close)| AutoPair {
                        open: open.to_owned(),
                        close: close.to_owned(),
                        ..AutoPair::default()
                    })
                    .collect()
            });
        auto_closing.retain(|pair| !pair.open.is_empty() && !pair.close.is_empty());
        let mut surrounding = optional::<Vec<AutoPair>>(self.raw.get("surroundingPairs"))?
            .unwrap_or_else(|| auto_closing.clone());
        surrounding.retain(|pair| !pair.open.is_empty() && !pair.close.is_empty());
        Ok(LanguagePairs {
            auto_closing,
            surrounding,
            auto_close_before: self.auto_close_before().map(str::to_owned),
            cache: ConfigurationRegexes::default(),
        })
    }
    pub fn word_regex(&self) -> Result<Option<RegexPattern>> {
        optional(self.raw.get("wordPattern"))
    }

    /// Returns the configured word at a UTF-16 caret offset. If the package
    /// has no wordPattern, the host keeps its normal word-navigation policy.
    /// The expression runs on the physical line using ECMAScript semantics.
    pub fn word_at_offset(&self, line: &str, offset: usize) -> Result<Option<WordRange>> {
        let text: Vec<_> = line.encode_utf16().collect();
        if offset > text.len() {
            return Err(Error::InvalidOffset(offset));
        }
        let Some(expression) = self.word_regex()? else {
            return Ok(None);
        };
        let regex = expression.compile()?;
        let clock = regex.match_timeout().map(|limit| (Instant::now(), limit));
        for (index, found) in regex.find_iter_utf16(&text).enumerate() {
            let found = found.map_err(|error| Error::RegexLimit(error.to_string()))?;
            if index >= 100_000 || clock.is_some_and(|(start, limit)| start.elapsed() >= limit) {
                return Err(Error::RegexLimit(
                    "word lookup exceeded its match budget".into(),
                ));
            }
            let start_index = found.start();
            let end_index = found.end();
            if start_index > offset {
                break;
            }
            if !found.is_empty() && offset <= end_index {
                return Ok(Some(WordRange {
                    start_index,
                    end_index,
                }));
            }
        }
        Ok(None)
    }
}

impl RegexPattern {
    fn is_match(&self, cache: &ConfigurationRegexes, text: &str) -> Result<bool> {
        let text: Vec<_> = text.encode_utf16().collect();
        cache
            .get(self)?
            .is_match_utf16(&text)
            .map_err(|error| Error::RegexLimit(error.to_string()))
    }

    fn compile(&self) -> Result<Regex> {
        let invalid = |error: corvus_regex_ecma262::Error| Error::Regex {
            pattern: self.pattern.clone(),
            message: error.to_string(),
        };
        let flags = Flags::parse(&self.flags).map_err(invalid)?;
        // The bare WebAssembly standard library provides no monotonic clock.
        let timeout = (!cfg!(all(target_arch = "wasm32", target_os = "unknown")))
            .then_some(Duration::from_millis(50));
        Regex::with_timeout(&self.pattern, flags, timeout).map_err(invalid)
    }
}

#[derive(Default)]
struct ConfigurationRegexes(RefCell<HashMap<RegexPattern, Rc<Regex>>>);

impl ConfigurationRegexes {
    fn get(&self, pattern: &RegexPattern) -> Result<Rc<Regex>> {
        if let Some(regex) = self.0.borrow().get(pattern) {
            return Ok(regex.clone());
        }
        let regex = Rc::new(pattern.compile()?);
        let mut entries = self.0.borrow_mut();
        if entries.len() >= 256 {
            entries.clear();
        }
        entries.insert(pattern.clone(), regex.clone());
        Ok(regex)
    }
}
