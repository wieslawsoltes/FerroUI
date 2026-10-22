use fancy_regex::Regex;
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
    /// Parses an editable syntax tree and compiles its main rule set.
    pub fn load(xml: &str) -> Result<Self, HighlightingError> {
        super::xshd::load_xshd(xml)?.compile()
    }

    /// Resolves named colors, local/inline rule sets and cross-definition
    /// `Definition/RuleSet` references against explicitly supplied dependencies.
    /// Imports retain their position relative to surrounding rules and spans.
    pub fn load_with_dependencies(
        xml: &str,
        dependencies: &[&str],
    ) -> Result<Self, HighlightingError> {
        let syntax = super::xshd::load_xshd(xml)?;
        let dependencies = dependencies
            .iter()
            .map(|text| super::xshd::load_xshd(text))
            .collect::<Result<Vec<_>, _>>()?;
        syntax.compile_with_dependencies(&dependencies.iter().collect::<Vec<_>>())
    }
}
