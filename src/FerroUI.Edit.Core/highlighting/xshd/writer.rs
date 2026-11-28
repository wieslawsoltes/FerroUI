use super::model::*;
use super::visitor::XshdVisitor;
use crate::highlighting::HighlightingError;
use std::collections::HashSet;

pub const XSHD_NAMESPACE: &str = "http://icsharpcode.net/sharpdevelop/syntaxdefinition/2008";

/// Serializes an editable tree as standard v2 XML. Complete color metadata and
/// properties are retained. XML source coordinates are intentionally not saved.
/// Programmatic inline imports are hoisted to named top-level rule sets.
#[derive(Default)]
pub struct SaveXshdVisitor {
    output: String,
    depth: usize,
    ignore_case: bool,
    used_names: HashSet<String>,
    next_name: usize,
    pending_sets: Vec<XshdRuleSet>,
    pending_colors: Vec<XshdColor>,
}

impl SaveXshdVisitor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save(definition: &XshdSyntaxDefinition) -> Result<String, HighlightingError> {
        let mut writer = Self::new();
        writer.write_definition(definition)?;
        Ok(writer.into_string())
    }

    /// Replaces the buffer only after the complete definition is serialized.
    pub fn write_definition(
        &mut self,
        definition: &XshdSyntaxDefinition,
    ) -> Result<(), HighlightingError> {
        let mut next = Self::new();
        collect_names(&definition.elements, &mut next.used_names);
        next.start("SyntaxDefinition");
        next.optional_attribute("name", definition.name.as_deref())?;
        next.attribute("extensions", &definition.extensions.join(";"))?;
        next.content();
        definition.accept_elements(&mut next)?;
        let mut index = 0;
        while index < next.pending_sets.len() {
            // Visiting a hoisted set may discover more inline imports.
            let set = next.pending_sets[index].clone();
            next.visit_rule_set(&set)?;
            index += 1;
        }
        for color in std::mem::take(&mut next.pending_colors) {
            next.visit_color(&color)?;
        }
        next.end("SyntaxDefinition");
        *self = next;
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.output
    }

    pub fn into_string(self) -> String {
        self.output
    }

    fn start(&mut self, name: &str) {
        self.output.push('<');
        self.output.push_str(name);
        if self.depth == 0 {
            self.output.push_str(" xmlns=\"");
            self.output.push_str(XSHD_NAMESPACE);
            self.output.push('"');
        }
        self.depth += 1;
    }

    fn content(&mut self) {
        self.output.push('>');
    }

    fn empty_end(&mut self) {
        self.output.push_str("/>");
        self.depth -= 1;
    }

    fn end(&mut self, name: &str) {
        self.output.push_str("</");
        self.output.push_str(name);
        self.output.push('>');
        self.depth -= 1;
    }

    fn attribute(&mut self, name: &str, value: &str) -> Result<(), HighlightingError> {
        self.output.push(' ');
        self.output.push_str(name);
        self.output.push_str("=\"");
        escape_xml(&mut self.output, value, true)?;
        self.output.push('"');
        Ok(())
    }

    fn optional_attribute(
        &mut self,
        name: &str,
        value: Option<&str>,
    ) -> Result<(), HighlightingError> {
        if let Some(value) = value {
            self.attribute(name, value)?;
        }
        Ok(())
    }

    fn boolean(&mut self, name: &str, value: Option<bool>) -> Result<(), HighlightingError> {
        if let Some(value) = value {
            self.attribute(name, if value { "true" } else { "false" })?;
        }
        Ok(())
    }

    fn text(&mut self, text: &str) -> Result<(), HighlightingError> {
        escape_xml(&mut self.output, text, false)
    }

    fn color_attributes(&mut self, color: &XshdColor) -> Result<(), HighlightingError> {
        self.optional_attribute("foreground", color.foreground.as_deref())?;
        self.optional_attribute("background", color.background.as_deref())?;
        self.optional_attribute("fontFamily", color.font_family.as_deref())?;
        if let Some(size) = color.font_size {
            if !size.is_finite() || size <= 0.0 {
                return Err(color
                    .position
                    .error("fontSize must be a finite positive number"));
            }
            self.attribute("fontSize", &size.to_string())?;
        }
        self.optional_attribute("fontWeight", color.font_weight.as_deref())?;
        self.optional_attribute("fontStyle", color.font_style.as_deref())?;
        self.boolean("underline", color.underline)?;
        self.boolean("strikethrough", color.strikethrough)?;
        Ok(())
    }

    fn color_reference(
        &mut self,
        reference: &XshdReference<XshdColor>,
    ) -> Result<(), HighlightingError> {
        match reference {
            XshdReference::None => Ok(()),
            XshdReference::Named { .. } => self.attribute(
                "color",
                &reference.attribute_value().expect("named reference"),
            ),
            XshdReference::Inline(color) => {
                if let Some(name) = color.name.as_deref() {
                    self.attribute("color", name)?;
                    self.pending_colors.push((**color).clone());
                    Ok(())
                } else {
                    self.color_attributes(color)
                }
            }
        }
    }

    fn rule_reference(
        &mut self,
        reference: &XshdReference<XshdRuleSet>,
    ) -> Result<(), HighlightingError> {
        if let Some(value) = reference.attribute_value() {
            self.attribute("ruleSet", &value)?;
        }
        Ok(())
    }

    fn pattern_element(
        &mut self,
        name: &str,
        regex: Option<&str>,
        kind: XshdRegexType,
        color: &XshdReference<XshdColor>,
        position: XshdPosition,
    ) -> Result<(), HighlightingError> {
        let Some(regex) = regex else {
            if !color.is_none() {
                return Err(position.error(format!("{name} color has no expression")));
            }
            return Ok(());
        };
        self.start(name);
        self.color_reference(color)?;
        self.content();
        if kind == XshdRegexType::Default {
            self.text("(?-x)")?;
        }
        self.text(regex)?;
        self.end(name);
        Ok(())
    }

    fn next_rule_name(&mut self) -> String {
        loop {
            let name = format!("__InlineImport{}", self.next_name);
            self.next_name += 1;
            if self.used_names.insert(name.clone()) {
                return name;
            }
        }
    }
}

impl XshdVisitor for SaveXshdVisitor {
    type Output = ();
    type Error = HighlightingError;

    fn visit_rule_set(&mut self, set: &XshdRuleSet) -> Result<(), HighlightingError> {
        self.start("RuleSet");
        self.optional_attribute("name", set.name.as_deref())?;
        self.boolean("ignoreCase", set.ignore_case)?;
        self.content();
        let previous_case = self.ignore_case;
        self.ignore_case = set.ignore_case.unwrap_or(previous_case);
        let result = set.accept_elements(self);
        self.ignore_case = previous_case;
        result?;
        self.end("RuleSet");
        Ok(())
    }

    fn visit_color(&mut self, color: &XshdColor) -> Result<(), HighlightingError> {
        self.start("Color");
        self.optional_attribute("name", color.name.as_deref())?;
        self.color_attributes(color)?;
        self.optional_attribute("exampleText", color.example_text.as_deref())?;
        self.empty_end();
        Ok(())
    }

    fn visit_property(&mut self, property: &XshdProperty) -> Result<(), HighlightingError> {
        self.start("Property");
        self.attribute("name", &property.name)?;
        self.attribute("value", &property.value)?;
        self.empty_end();
        Ok(())
    }

    fn visit_keywords(&mut self, keywords: &XshdKeywords) -> Result<(), HighlightingError> {
        self.start("Keywords");
        self.color_reference(&keywords.color)?;
        self.content();
        for word in &keywords.words {
            self.start("Word");
            self.content();
            self.text(word)?;
            self.end("Word");
        }
        self.end("Keywords");
        Ok(())
    }

    fn visit_span(&mut self, span: &XshdSpan) -> Result<(), HighlightingError> {
        self.start("Span");
        self.color_reference(&span.span_color)?;
        let begin_attribute =
            span.begin_type == XshdRegexType::Default && span.begin_color.is_none();
        let end_attribute = span.end_type == XshdRegexType::Default && span.end_color.is_none();
        if begin_attribute {
            self.optional_attribute("begin", span.begin.as_deref())?;
        }
        if end_attribute {
            self.optional_attribute("end", span.end.as_deref())?;
        }
        self.rule_reference(&span.rule_set)?;
        if span.multiline {
            self.attribute("multiline", "true")?;
        }
        self.content();
        if !begin_attribute {
            self.pattern_element(
                "Begin",
                span.begin.as_deref(),
                span.begin_type,
                &span.begin_color,
                span.position,
            )?;
        }
        if !end_attribute {
            self.pattern_element(
                "End",
                span.end.as_deref(),
                span.end_type,
                &span.end_color,
                span.position,
            )?;
        }
        if let XshdReference::Inline(set) = &span.rule_set {
            self.visit_rule_set(set)?;
        }
        self.end("Span");
        Ok(())
    }

    fn visit_import(&mut self, import: &XshdImport) -> Result<(), HighlightingError> {
        self.start("Import");
        match &import.rule_set {
            XshdReference::None => return Err(import.position.error("Import needs a rule set")),
            XshdReference::Named { .. } => self.rule_reference(&import.rule_set)?,
            XshdReference::Inline(set) => {
                let mut set = (**set).clone();
                let name = set.name.clone().unwrap_or_else(|| self.next_rule_name());
                set.name = Some(name.clone());
                set.ignore_case = Some(set.ignore_case.unwrap_or(self.ignore_case));
                self.pending_sets.push(set);
                self.attribute("ruleSet", &name)?;
            }
        }
        self.empty_end();
        Ok(())
    }

    fn visit_rule(&mut self, rule: &XshdRule) -> Result<(), HighlightingError> {
        self.start("Rule");
        self.color_reference(&rule.color)?;
        self.content();
        let regex = rule
            .regex
            .as_deref()
            .ok_or_else(|| rule.position.error("Regex missing"))?;
        // Element expressions normally enable extended mode. A leading option
        // change preserves a programmatically supplied attribute-style regex.
        if rule.regex_type == XshdRegexType::Default {
            self.text("(?-x)")?;
        }
        self.text(regex)?;
        self.end("Rule");
        Ok(())
    }
}

fn escape_xml(output: &mut String, input: &str, attribute: bool) -> Result<(), HighlightingError> {
    for character in input.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' if attribute => output.push_str("&quot;"),
            '\r' => output.push_str("&#xD;"),
            '\n' if attribute => output.push_str("&#xA;"),
            '\t' if attribute => output.push_str("&#x9;"),
            '\u{0}'..='\u{8}'
            | '\u{b}'
            | '\u{c}'
            | '\u{e}'..='\u{1f}'
            | '\u{fffe}'
            | '\u{ffff}' => {
                return Err(HighlightingError::Xml(format!(
                    "U+{:04X} is not permitted in XML 1.0",
                    character as u32
                )));
            }
            _ => output.push(character),
        }
    }
    Ok(())
}

fn collect_names(elements: &[XshdElement], names: &mut HashSet<String>) {
    for element in elements {
        match element {
            XshdElement::RuleSet(set) => collect_set_names(set, names),
            XshdElement::Span(span) => {
                if let XshdReference::Inline(set) = &span.rule_set {
                    collect_set_names(set, names);
                }
            }
            XshdElement::Import(import) => {
                if let XshdReference::Inline(set) = &import.rule_set {
                    collect_set_names(set, names);
                }
            }
            _ => {}
        }
    }
}

fn collect_set_names(set: &XshdRuleSet, names: &mut HashSet<String>) {
    if let Some(name) = &set.name {
        names.insert(name.clone());
    }
    collect_names(&set.elements, names);
}
