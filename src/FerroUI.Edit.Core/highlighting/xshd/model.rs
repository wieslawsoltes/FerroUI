use super::visitor::{XshdVisitor, XshdVisitorMut};
use crate::highlighting::{HighlightingDefinition, HighlightingError};
use std::collections::BTreeSet;

/// One-based XML source coordinates. Zero means a programmatically created node.
/// Columns count UTF-16 code units, consistently with document locations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct XshdPosition {
    pub line: usize,
    pub column: usize,
}

impl XshdPosition {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    pub(crate) fn error(self, message: impl std::fmt::Display) -> HighlightingError {
        HighlightingError::InvalidDefinition(self.message(message))
    }

    pub(crate) fn reference_error(self, message: impl std::fmt::Display) -> HighlightingError {
        HighlightingError::Reference(self.message(message))
    }

    pub(crate) fn pattern_error(self, message: impl std::fmt::Display) -> HighlightingError {
        HighlightingError::Pattern(self.message(message))
    }

    fn message(self, message: impl std::fmt::Display) -> String {
        if self.line == 0 {
            message.to_string()
        } else {
            format!(
                "Error at line {}, column {}: {message}",
                self.line, self.column
            )
        }
    }
}

/// A reference remains unresolved until the syntax definition is compiled.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum XshdReference<T> {
    #[default]
    None,
    Named {
        definition: Option<String>,
        element: String,
    },
    Inline(Box<T>),
}

impl<T> XshdReference<T> {
    pub fn local(element: impl Into<String>) -> Self {
        Self::Named {
            definition: None,
            element: element.into(),
        }
    }

    pub fn external(definition: impl Into<String>, element: impl Into<String>) -> Self {
        Self::Named {
            definition: Some(definition.into()),
            element: element.into(),
        }
    }

    pub fn inline(element: T) -> Self {
        Self::Inline(Box::new(element))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    pub fn inline_element(&self) -> Option<&T> {
        match self {
            Self::Inline(element) => Some(element),
            _ => None,
        }
    }

    pub fn inline_element_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Inline(element) => Some(element),
            _ => None,
        }
    }

    pub(crate) fn from_attribute(value: &str) -> Self {
        match value.rsplit_once('/') {
            Some((definition, element)) => Self::external(definition, element),
            None => Self::local(value),
        }
    }

    pub(crate) fn attribute_value(&self) -> Option<String> {
        match self {
            Self::Named {
                definition: Some(definition),
                element,
            } => Some(format!("{definition}/{element}")),
            Self::Named {
                definition: None,
                element,
            } => Some(element.clone()),
            _ => None,
        }
    }
}

/// Attribute patterns preserve whitespace; element patterns permit whitespace
/// and comments outside character classes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum XshdRegexType {
    #[default]
    Default,
    IgnorePatternWhitespace,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdSyntaxDefinition {
    pub name: Option<String>,
    pub extensions: Vec<String>,
    pub elements: Vec<XshdElement>,
}

impl XshdSyntaxDefinition {
    pub fn load(xml: &str) -> Result<Self, HighlightingError> {
        super::load_xshd(xml)
    }

    pub fn compile(&self) -> Result<HighlightingDefinition, HighlightingError> {
        super::compile_xshd(self, &[])
    }

    pub fn compile_with_dependencies(
        &self,
        dependencies: &[&XshdSyntaxDefinition],
    ) -> Result<HighlightingDefinition, HighlightingError> {
        super::compile_xshd(self, dependencies)
    }

    pub fn to_xml(&self) -> Result<String, HighlightingError> {
        super::SaveXshdVisitor::save(self)
    }

    /// Names referenced by external colors and rule sets, including references
    /// nested inside spans and programmatically supplied inline imports.
    pub fn external_definitions(&self) -> BTreeSet<&str> {
        let mut names = BTreeSet::new();
        collect_external_definitions(&self.elements, &mut names);
        names
    }

    pub fn accept_elements<V: XshdVisitor + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        for element in &self.elements {
            element.accept(visitor)?;
        }
        Ok(())
    }

    pub fn accept_elements_mut<V: XshdVisitorMut + ?Sized>(
        &mut self,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        for element in &mut self.elements {
            element.accept_mut(visitor)?;
        }
        Ok(())
    }
}

fn collect_external_definitions<'a>(elements: &'a [XshdElement], names: &mut BTreeSet<&'a str>) {
    fn color<'a>(reference: &'a XshdReference<XshdColor>, names: &mut BTreeSet<&'a str>) {
        if let XshdReference::Named {
            definition: Some(name),
            ..
        } = reference
        {
            names.insert(name);
        }
    }
    fn rule_set<'a>(reference: &'a XshdReference<XshdRuleSet>, names: &mut BTreeSet<&'a str>) {
        match reference {
            XshdReference::Named {
                definition: Some(name),
                ..
            } => {
                names.insert(name);
            }
            XshdReference::Inline(set) => collect_external_definitions(&set.elements, names),
            _ => {}
        }
    }
    for element in elements {
        match element {
            XshdElement::RuleSet(set) => collect_external_definitions(&set.elements, names),
            XshdElement::Keywords(keywords) => color(&keywords.color, names),
            XshdElement::Rule(rule) => color(&rule.color, names),
            XshdElement::Span(span) => {
                color(&span.span_color, names);
                color(&span.begin_color, names);
                color(&span.end_color, names);
                rule_set(&span.rule_set, names);
            }
            XshdElement::Import(import) => rule_set(&import.rule_set, names),
            XshdElement::Color(_) | XshdElement::Property(_) => {}
        }
    }
}

/// Ordered syntax nodes. Properties participate in visitation and serialization.
#[derive(Clone, Debug, PartialEq)]
pub enum XshdElement {
    RuleSet(XshdRuleSet),
    Color(XshdColor),
    Property(XshdProperty),
    Keywords(XshdKeywords),
    Span(XshdSpan),
    Import(XshdImport),
    Rule(XshdRule),
}

impl XshdElement {
    pub fn position(&self) -> XshdPosition {
        match self {
            Self::RuleSet(element) => element.position,
            Self::Color(element) => element.position,
            Self::Property(element) => element.position,
            Self::Keywords(element) => element.position,
            Self::Span(element) => element.position,
            Self::Import(element) => element.position,
            Self::Rule(element) => element.position,
        }
    }

    pub fn accept<V: XshdVisitor + ?Sized>(&self, visitor: &mut V) -> Result<V::Output, V::Error> {
        match self {
            Self::RuleSet(element) => visitor.visit_rule_set(element),
            Self::Color(element) => visitor.visit_color(element),
            Self::Property(element) => visitor.visit_property(element),
            Self::Keywords(element) => visitor.visit_keywords(element),
            Self::Span(element) => visitor.visit_span(element),
            Self::Import(element) => visitor.visit_import(element),
            Self::Rule(element) => visitor.visit_rule(element),
        }
    }

    pub fn accept_mut<V: XshdVisitorMut + ?Sized>(
        &mut self,
        visitor: &mut V,
    ) -> Result<V::Output, V::Error> {
        match self {
            Self::RuleSet(element) => visitor.visit_rule_set(element),
            Self::Color(element) => visitor.visit_color(element),
            Self::Property(element) => visitor.visit_property(element),
            Self::Keywords(element) => visitor.visit_keywords(element),
            Self::Span(element) => visitor.visit_span(element),
            Self::Import(element) => visitor.visit_import(element),
            Self::Rule(element) => visitor.visit_rule(element),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdRuleSet {
    pub position: XshdPosition,
    pub name: Option<String>,
    pub ignore_case: Option<bool>,
    pub elements: Vec<XshdElement>,
}

impl XshdRuleSet {
    pub fn accept_elements<V: XshdVisitor + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        for element in &self.elements {
            element.accept(visitor)?;
        }
        Ok(())
    }

    pub fn accept_elements_mut<V: XshdVisitorMut + ?Sized>(
        &mut self,
        visitor: &mut V,
    ) -> Result<(), V::Error> {
        for element in &mut self.elements {
            element.accept_mut(visitor)?;
        }
        Ok(())
    }
}

/// Complete XML color metadata. Brush and font strings stay independent of a
/// rendering backend; a host may resolve them through its own theme resources.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdColor {
    pub position: XshdPosition,
    pub name: Option<String>,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<String>,
    pub font_style: Option<String>,
    pub underline: Option<bool>,
    pub strikethrough: Option<bool>,
    pub example_text: Option<String>,
}

impl XshdColor {
    pub fn has_style(&self) -> bool {
        self.foreground.is_some()
            || self.background.is_some()
            || self.font_family.is_some()
            || self.font_size.is_some()
            || self.font_weight.is_some()
            || self.font_style.is_some()
            || self.underline.is_some()
            || self.strikethrough.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdProperty {
    pub position: XshdPosition,
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdKeywords {
    pub position: XshdPosition,
    pub color: XshdReference<XshdColor>,
    pub words: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdSpan {
    pub position: XshdPosition,
    pub begin: Option<String>,
    pub begin_type: XshdRegexType,
    pub end: Option<String>,
    pub end_type: XshdRegexType,
    pub multiline: bool,
    pub rule_set: XshdReference<XshdRuleSet>,
    pub span_color: XshdReference<XshdColor>,
    pub begin_color: XshdReference<XshdColor>,
    pub end_color: XshdReference<XshdColor>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdImport {
    pub position: XshdPosition,
    pub rule_set: XshdReference<XshdRuleSet>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct XshdRule {
    pub position: XshdPosition,
    pub regex: Option<String>,
    pub regex_type: XshdRegexType,
    pub color: XshdReference<XshdColor>,
}

impl XshdReference<XshdColor> {
    pub fn accept<V: XshdVisitor + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<Option<V::Output>, V::Error> {
        self.inline_element()
            .map(|color| visitor.visit_color(color))
            .transpose()
    }

    pub fn accept_mut<V: XshdVisitorMut + ?Sized>(
        &mut self,
        visitor: &mut V,
    ) -> Result<Option<V::Output>, V::Error> {
        self.inline_element_mut()
            .map(|color| visitor.visit_color(color))
            .transpose()
    }
}

impl XshdReference<XshdRuleSet> {
    pub fn accept<V: XshdVisitor + ?Sized>(
        &self,
        visitor: &mut V,
    ) -> Result<Option<V::Output>, V::Error> {
        self.inline_element()
            .map(|set| visitor.visit_rule_set(set))
            .transpose()
    }

    pub fn accept_mut<V: XshdVisitorMut + ?Sized>(
        &mut self,
        visitor: &mut V,
    ) -> Result<Option<V::Output>, V::Error> {
        self.inline_element_mut()
            .map(|set| visitor.visit_rule_set(set))
            .transpose()
    }
}
