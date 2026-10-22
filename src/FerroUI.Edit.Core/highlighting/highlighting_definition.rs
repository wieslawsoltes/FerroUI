use fancy_regex::{Regex, RegexBuilder};
use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HighlightingColor {
    pub name: Option<String>,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font_size: Option<f64>,
}
impl HighlightingColor {
    pub fn merge_with(&self, overlay: &Self) -> Self {
        Self {
            name: overlay.name.clone().or_else(|| self.name.clone()),
            foreground: overlay
                .foreground
                .clone()
                .or_else(|| self.foreground.clone()),
            background: overlay
                .background
                .clone()
                .or_else(|| self.background.clone()),
            bold: overlay.bold.or(self.bold),
            italic: overlay.italic.or(self.italic),
            underline: overlay.underline.or(self.underline),
            font_size: overlay.font_size.or(self.font_size),
        }
    }
}
#[derive(Debug)]
pub enum HighlightingError {
    Xml(String),
    Pattern(String),
    Reference(String),
    InvalidDefinition(String),
    MatchBudget(String),
    InvalidState,
    Disposed,
    ReentrantOperation,
    HighlightingGroupAlreadyOpen,
    NoHighlightingGroup,
    HighlightingInterrupted,
}
impl fmt::Display for HighlightingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xml(s) => write!(f, "Invalid highlighting XML: {s}"),
            Self::Pattern(s) => write!(f, "Invalid highlighting regex: {s}"),
            Self::Reference(s) => write!(f, "Unresolved highlighting reference: {s}"),
            Self::InvalidDefinition(s) => write!(f, "Invalid highlighting definition: {s}"),
            Self::MatchBudget(s) => write!(f, "Highlighting budget exceeded: {s}"),
            Self::InvalidState => {
                f.write_str("Highlighting state belongs to a different definition")
            }
            Self::Disposed => f.write_str("The document highlighter has been disposed"),
            Self::ReentrantOperation => {
                f.write_str("A highlighting operation is already in progress")
            }
            Self::HighlightingGroupAlreadyOpen => {
                f.write_str("A highlighting group is already open")
            }
            Self::NoHighlightingGroup => f.write_str("No highlighting group is open"),
            Self::HighlightingInterrupted => {
                f.write_str("The document or highlighting cache changed during highlighting")
            }
        }
    }
}
impl std::error::Error for HighlightingError {}

#[derive(Clone, Debug)]
pub struct HighlightingRule {
    pub regex: Regex,
    pub color: Option<HighlightingColor>,
}
#[derive(Clone, Debug)]
pub struct HighlightingSpan {
    pub start_expression: Regex,
    pub end_expression: Regex,
    pub rule_set: Option<usize>,
    pub span_color: Option<HighlightingColor>,
    pub start_color: Option<HighlightingColor>,
    pub end_color: Option<HighlightingColor>,
    pub multiline: bool,
}
#[derive(Clone, Debug, Default)]
pub struct HighlightingRuleSet {
    pub name: Option<String>,
    pub rules: Vec<HighlightingRule>,
    pub spans: Vec<HighlightingSpan>,
    pub imports: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct HighlightingDefinition {
    pub name: String,
    pub extensions: Vec<String>,
    pub properties: HashMap<String, String>,
    pub colors: HashMap<String, HighlightingColor>,
    pub rule_sets: Vec<HighlightingRuleSet>,
    pub main_rule_set: usize,
}

impl HighlightingDefinition {
    pub fn load(xml: &str) -> Result<Self, HighlightingError> {
        Self::load_with_dependencies(xml, &[])
    }
    /// Resolves named colors, local/inline rule sets and cross-definition
    /// `Definition/RuleSet` references against explicitly supplied dependencies.
    pub fn load_with_dependencies(
        xml: &str,
        dependencies: &[&str],
    ) -> Result<Self, HighlightingError> {
        let mut documents = Vec::with_capacity(dependencies.len() + 1);
        for text in std::iter::once(xml).chain(dependencies.iter().copied()) {
            documents.push(
                roxmltree::Document::parse(text)
                    .map_err(|e| HighlightingError::Xml(e.to_string()))?,
            );
        }
        let mut definitions = HashMap::new();
        let mut names = HashMap::new();
        let mut node_sets = HashMap::new();
        let mut sets = Vec::new();
        let mut palettes = Vec::new();
        let mut main = None;
        for (di, document) in documents.iter().enumerate() {
            let root = document.root_element();
            if root.tag_name().name() != "SyntaxDefinition" {
                return Err(HighlightingError::InvalidDefinition(
                    "Only XSHD v2 SyntaxDefinition documents are accepted".into(),
                ));
            }
            if let Some(name) = root.attribute("name") {
                definitions.insert(name.to_owned(), di);
            }
            let mut palette = HashMap::new();
            for node in root.children().filter(|n| n.has_tag_name("Color")) {
                let Some(name) = node.attribute("name") else {
                    return Err(HighlightingError::InvalidDefinition(
                        "A named color needs a name".into(),
                    ));
                };
                let color = parse_color(node)?;
                palette.insert(name.to_owned(), color);
            }
            palettes.push(palette);
            for node in root.descendants().filter(|n| n.has_tag_name("RuleSet")) {
                let index = sets.len();
                node_sets.insert((di, node.id()), index);
                if let Some(name) = node.attribute("name") {
                    names.insert((di, name.to_owned()), index);
                } else if node.parent() == Some(root)
                    || node.parent().is_some_and(|n| n.has_tag_name("RuleSets"))
                {
                    names.insert((di, String::new()), index);
                    if di == 0 {
                        if main.replace(index).is_some() {
                            return Err(HighlightingError::InvalidDefinition(
                                "More than one main RuleSet".into(),
                            ));
                        }
                    }
                }
                sets.push(HighlightingRuleSet {
                    name: node.attribute("name").map(str::to_owned),
                    ..HighlightingRuleSet::default()
                });
            }
        }
        let reference = |di: usize, name: &str| -> Result<usize, HighlightingError> {
            let (target, name) = if let Some((definition, name)) = name.rsplit_once('/') {
                (
                    *definitions
                        .get(definition)
                        .ok_or_else(|| HighlightingError::Reference(definition.into()))?,
                    name,
                )
            } else {
                (di, name)
            };
            names
                .get(&(target, name.to_owned()))
                .copied()
                .ok_or_else(|| HighlightingError::Reference(name.into()))
        };
        let resolve_color = |di: usize,
                             node: roxmltree::Node<'_, '_>|
         -> Result<Option<HighlightingColor>, HighlightingError> {
            if documents[di]
                .root_element()
                .children()
                .any(|n| n.has_tag_name("RuleSets"))
            {
                return Ok(Some(HighlightingColor {
                    foreground: node.attribute("color").map(str::to_owned),
                    background: node.attribute("bgcolor").map(str::to_owned),
                    bold: node.attribute("bold").map(|v| v == "true"),
                    italic: node.attribute("italic").map(|v| v == "true"),
                    ..HighlightingColor::default()
                }));
            }
            let named = if let Some(name) = node.attribute("color") {
                let (target, name) = if let Some((definition, name)) = name.rsplit_once('/') {
                    (
                        *definitions
                            .get(definition)
                            .ok_or_else(|| HighlightingError::Reference(definition.into()))?,
                        name,
                    )
                } else {
                    (di, name)
                };
                Some(
                    palettes[target]
                        .get(name)
                        .cloned()
                        .ok_or_else(|| HighlightingError::Reference(name.into()))?,
                )
            } else {
                None
            };
            let inline = parse_color(node)?;
            if inline == HighlightingColor::default() {
                Ok(named)
            } else {
                Ok(Some(named.unwrap_or_default().merge_with(&inline)))
            }
        };
        for (di, document) in documents.iter().enumerate() {
            let legacy = document
                .root_element()
                .children()
                .any(|n| n.has_tag_name("RuleSets"));
            for node in document.descendants().filter(|n| n.has_tag_name("RuleSet")) {
                let index = node_sets[&(di, node.id())];
                let ignore_case = node
                    .ancestors()
                    .find_map(|n| {
                        n.attribute("ignoreCase")
                            .or_else(|| n.attribute("ignorecase"))
                    })
                    .is_some_and(|s| s == "true");
                if legacy {
                    if let Some(digits) = document
                        .root_element()
                        .children()
                        .find(|n| n.has_tag_name("Digits"))
                    {
                        sets[index].rules.push(HighlightingRule{regex:compile(r"\b0[xX][0-9a-fA-F]+|(\b\d+(\.[0-9]+)?|\.[0-9]+)([eE][+-]?[0-9]+)?",ignore_case,false)?,color:resolve_color(di,digits)?});
                    }
                }
                for child in node.children().filter(|n| n.is_element()) {
                    match child.tag_name().name() {
                        "Import" => {
                            let name = child.attribute("ruleSet").ok_or_else(|| {
                                HighlightingError::InvalidDefinition(
                                    "Import needs a ruleSet".into(),
                                )
                            })?;
                            sets[index].imports.push(reference(di, name)?);
                        }
                        "Rule" => {
                            let pattern = element_text(child);
                            sets[index].rules.push(HighlightingRule {
                                regex: compile(&pattern, ignore_case, true)?,
                                color: resolve_color(di, child)?,
                            });
                        }
                        "Keywords" | "KeyWords" => {
                            let words: Vec<_> = child
                                .children()
                                .filter(|n| n.has_tag_name("Word") || n.has_tag_name("Key"))
                                .map(|word| {
                                    word.attribute("word")
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| element_text(word))
                                })
                                .filter(|s| !s.is_empty())
                                .map(|word| {
                                    let before = word
                                        .chars()
                                        .next()
                                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
                                    let after = word
                                        .chars()
                                        .next_back()
                                        .is_some_and(|c| c.is_alphanumeric() || c == '_');
                                    format!(
                                        "{}{}{}",
                                        if before { r"\b" } else { "" },
                                        regex::escape(&word),
                                        if after { r"\b" } else { "" }
                                    )
                                })
                                .collect();
                            if words.is_empty() {
                                continue;
                            }
                            let pattern = format!(r"(?:{})", words.join("|"));
                            sets[index].rules.push(HighlightingRule {
                                regex: compile(&pattern, ignore_case, false)?,
                                color: resolve_color(di, child)?,
                            });
                        }
                        "Span" => {
                            let begin = child.children().find(|n| n.has_tag_name("Begin"));
                            let end = child.children().find(|n| n.has_tag_name("End"));
                            let begin_pattern = child
                                .attribute("begin")
                                .map(str::to_owned)
                                .or_else(|| begin.map(element_text))
                                .ok_or_else(|| {
                                    HighlightingError::InvalidDefinition(
                                        "Span needs begin or Begin".into(),
                                    )
                                })?;
                            let end_pattern = child
                                .attribute("end")
                                .map(str::to_owned)
                                .or_else(|| end.map(element_text))
                                .unwrap_or_else(|| "$".into());
                            let begin_pattern = if legacy {
                                regex::escape(&begin_pattern)
                            } else {
                                begin_pattern
                            };
                            let end_pattern = if legacy && end.is_some() {
                                regex::escape(&end_pattern)
                            } else {
                                end_pattern
                            };
                            let rule_set = if let Some(name) = child
                                .attribute("ruleSet")
                                .or_else(|| child.attribute("rule"))
                            {
                                Some(reference(di, name)?)
                            } else {
                                child
                                    .children()
                                    .find(|n| n.has_tag_name("RuleSet"))
                                    .map(|n| node_sets[&(di, n.id())])
                            };
                            sets[index].spans.push(HighlightingSpan {
                                start_expression: compile(
                                    &begin_pattern,
                                    ignore_case,
                                    begin.is_some(),
                                )?,
                                end_expression: compile(&end_pattern, ignore_case, end.is_some())?,
                                rule_set,
                                span_color: resolve_color(di, child)?,
                                start_color: begin
                                    .map(|n| resolve_color(di, n))
                                    .transpose()?
                                    .flatten(),
                                end_color: end.map(|n| resolve_color(di, n)).transpose()?.flatten(),
                                multiline: if legacy {
                                    child.attribute("stopateol") != Some("true")
                                } else {
                                    child.attribute("multiline") == Some("true")
                                },
                            });
                        }
                        "Delimiters" if legacy => {}
                        "MarkFollowing" | "MarkPrevious" if legacy => {
                            let marker = regex::escape(&element_text(child));
                            let include = child.attribute("markmarker") == Some("true");
                            let pattern = match (child.has_tag_name("MarkFollowing"), include) {
                                (true, true) => format!(r"{marker}\s*[\d\w_]+"),
                                (true, false) => format!(r"(?<={marker}\s*)[\d\w_]+"),
                                (false, true) => format!(r"[\d\w_]+\s*{marker}"),
                                (false, false) => format!(r"[\d\w_]+(?=\s*{marker})"),
                            };
                            sets[index].rules.push(HighlightingRule {
                                regex: compile(&pattern, ignore_case, false)?,
                                color: resolve_color(di, child)?,
                            });
                        }
                        name => {
                            return Err(HighlightingError::InvalidDefinition(format!(
                                "Unsupported RuleSet element {name}"
                            )))
                        }
                    }
                }
            }
        }
        let root = documents[0].root_element();
        let properties = root
            .children()
            .filter(|n| n.has_tag_name("Property"))
            .filter_map(|n| {
                n.attribute("name").map(|name| {
                    (
                        name.to_owned(),
                        n.attribute("value")
                            .map(str::to_owned)
                            .unwrap_or_else(|| element_text(n)),
                    )
                })
            })
            .collect();
        Ok(Self {
            name: root.attribute("name").unwrap_or("").into(),
            extensions: root
                .attribute("extensions")
                .unwrap_or("")
                .split(';')
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect(),
            properties,
            colors: palettes.into_iter().next().unwrap_or_default(),
            rule_sets: sets,
            main_rule_set: main.ok_or_else(|| {
                HighlightingError::InvalidDefinition("A main RuleSet is required".into())
            })?,
        })
    }
}
fn parse_color(node: roxmltree::Node<'_, '_>) -> Result<HighlightingColor, HighlightingError> {
    let font_size = node
        .attribute("fontSize")
        .map(str::parse::<f64>)
        .transpose()
        .map_err(|_| {
            HighlightingError::InvalidDefinition("fontSize must be a finite positive number".into())
        })?;
    if font_size.is_some_and(|n| !n.is_finite() || n <= 0.0) {
        return Err(HighlightingError::InvalidDefinition(
            "fontSize must be a finite positive number".into(),
        ));
    }
    Ok(HighlightingColor {
        name: if node.has_tag_name("Color") {
            node.attribute("name").map(str::to_owned)
        } else {
            None
        },
        foreground: node.attribute("foreground").map(str::to_owned),
        background: node.attribute("background").map(str::to_owned),
        bold: node
            .attribute("fontWeight")
            .map(|v| v.eq_ignore_ascii_case("bold")),
        italic: node
            .attribute("fontStyle")
            .map(|v| v.eq_ignore_ascii_case("italic")),
        underline: node
            .attribute("underline")
            .map(|v| v.eq_ignore_ascii_case("true")),
        font_size,
    })
}
fn element_text(node: roxmltree::Node<'_, '_>) -> String {
    node.children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect()
}
fn compile(
    pattern: &str,
    ignore_case: bool,
    ignore_whitespace: bool,
) -> Result<Regex, HighlightingError> {
    RegexBuilder::new(pattern)
        .case_insensitive(ignore_case)
        .ignore_whitespace(ignore_whitespace)
        .backtrack_limit(1_000_000)
        .delegate_size_limit(16 * 1024 * 1024)
        .build()
        .map_err(|e| HighlightingError::Pattern(e.to_string()))
}
