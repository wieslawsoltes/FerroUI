use super::model::*;
use super::writer::XSHD_NAMESPACE;
use crate::highlighting::HighlightingError;
use roxmltree::Node;

/// Loads an editable syntax tree. References and import cycles are resolved
/// during compilation, so incomplete programmatic definitions remain editable.
/// Namespace-free v2 files and the historical RuleSets format are accepted.
pub fn load_xshd(xml: &str) -> Result<XshdSyntaxDefinition, HighlightingError> {
    let document = roxmltree::Document::parse(xml)
        .map_err(|error| HighlightingError::Xml(error.to_string()))?;
    let root = document.root_element();
    let parser = Parser::new(xml, root.tag_name().namespace());
    if root.tag_name().name() != "SyntaxDefinition" {
        return Err(parser.error(root, "Expected a SyntaxDefinition root element"));
    }
    if let Some(namespace) = root.tag_name().namespace() {
        if namespace != XSHD_NAMESPACE {
            return Err(parser.error(
                root,
                format!("Unknown syntax-definition namespace {namespace}"),
            ));
        }
    }
    if root.children().any(|node| node.has_tag_name("RuleSets")) {
        return super::legacy::load_legacy(&parser, root);
    }
    Ok(XshdSyntaxDefinition {
        name: root.attribute("name").map(str::to_owned),
        extensions: root
            .attribute("extensions")
            .map(|value| value.split(';').map(str::to_owned).collect())
            .unwrap_or_default(),
        elements: parser.elements(root)?,
    })
}

pub(super) struct Parser<'input> {
    input: &'input str,
    line_starts: Vec<usize>,
    namespace: Option<&'input str>,
}

impl<'input> Parser<'input> {
    fn new(input: &'input str, namespace: Option<&'input str>) -> Self {
        let mut line_starts = vec![0];
        let bytes = input.as_bytes();
        for (index, byte) in bytes.iter().copied().enumerate() {
            if byte == b'\n' || (byte == b'\r' && bytes.get(index + 1) != Some(&b'\n')) {
                line_starts.push(index + 1);
            }
        }
        Self {
            input,
            line_starts,
            namespace,
        }
    }

    pub(super) fn position(&self, node: Node<'_, '_>) -> XshdPosition {
        let offset = node.range().start;
        let line = self.line_starts.partition_point(|start| *start <= offset);
        let start = self.line_starts[line.saturating_sub(1)];
        XshdPosition::new(line, self.input[start..offset].encode_utf16().count() + 1)
    }

    pub(super) fn error(
        &self,
        node: Node<'_, '_>,
        message: impl std::fmt::Display,
    ) -> HighlightingError {
        self.position(node).error(message)
    }

    fn owns(&self, node: Node<'_, '_>) -> bool {
        node.tag_name().namespace() == self.namespace
    }

    pub(super) fn boolean(
        &self,
        node: Node<'_, '_>,
        name: &str,
    ) -> Result<Option<bool>, HighlightingError> {
        node.attribute(name)
            .map(|value| match value {
                "true" | "1" => Ok(true),
                "false" | "0" => Ok(false),
                _ => Err(self.error(node, format!("{name} must be true, false, 1 or 0"))),
            })
            .transpose()
    }

    fn name(&self, node: Node<'_, '_>) -> Result<Option<String>, HighlightingError> {
        let name = node.attribute("name");
        if name.is_some_and(|name| name.is_empty() || name.contains('/')) {
            return Err(self.error(
                node,
                "Element names must be nonempty and cannot contain '/'",
            ));
        }
        Ok(name.map(str::to_owned))
    }

    fn elements(&self, parent: Node<'_, '_>) -> Result<Vec<XshdElement>, HighlightingError> {
        parent
            .children()
            .filter(|node| node.is_element() && self.owns(*node))
            .map(|node| self.element(node))
            .collect()
    }

    fn element(&self, node: Node<'_, '_>) -> Result<XshdElement, HighlightingError> {
        let position = self.position(node);
        Ok(match node.tag_name().name() {
            "RuleSet" => XshdElement::RuleSet(self.rule_set(node)?),
            "Color" => {
                let mut color = self.color(node)?;
                color.name = self.name(node)?;
                color.example_text = node.attribute("exampleText").map(str::to_owned);
                XshdElement::Color(color)
            }
            "Property" => XshdElement::Property(XshdProperty {
                position,
                name: node
                    .attribute("name")
                    .ok_or_else(|| self.error(node, "Property needs a name"))?
                    .into(),
                value: node
                    .attribute("value")
                    .map(str::to_owned)
                    .unwrap_or_else(|| element_text(node)),
            }),
            "Keywords" => {
                let mut words = Vec::new();
                for word in node
                    .children()
                    .filter(|node| node.is_element() && self.owns(*node))
                {
                    if word.tag_name().name() != "Word" {
                        return Err(self.error(word, "Keywords can only contain Word elements"));
                    }
                    self.text_only(word)?;
                    words.push(element_text(word));
                }
                XshdElement::Keywords(XshdKeywords {
                    position,
                    color: self.color_reference(node)?,
                    words,
                })
            }
            "Span" => XshdElement::Span(self.span(node)?),
            "Import" => {
                if node
                    .children()
                    .any(|child| child.is_element() && self.owns(child))
                {
                    return Err(self.error(
                        node,
                        "Import references a rule set through its ruleSet attribute",
                    ));
                }
                XshdElement::Import(XshdImport {
                    position,
                    rule_set: node
                        .attribute("ruleSet")
                        .map(XshdReference::from_attribute)
                        .ok_or_else(|| self.error(node, "Import needs a ruleSet reference"))?,
                })
            }
            "Rule" => {
                self.text_only(node)?;
                XshdElement::Rule(XshdRule {
                    position,
                    regex: Some(element_text(node)),
                    regex_type: XshdRegexType::IgnorePatternWhitespace,
                    color: self.color_reference(node)?,
                })
            }
            name => return Err(self.error(node, format!("Unknown XSHD element {name}"))),
        })
    }

    fn text_only(&self, node: Node<'_, '_>) -> Result<(), HighlightingError> {
        if node.children().any(|child| child.is_element()) {
            return Err(self.error(
                node,
                format!("{} must contain text only", node.tag_name().name()),
            ));
        }
        Ok(())
    }

    fn rule_set(&self, node: Node<'_, '_>) -> Result<XshdRuleSet, HighlightingError> {
        Ok(XshdRuleSet {
            position: self.position(node),
            name: self.name(node)?,
            ignore_case: self.boolean(node, "ignoreCase")?,
            elements: self.elements(node)?,
        })
    }

    fn span(&self, node: Node<'_, '_>) -> Result<XshdSpan, HighlightingError> {
        let mut span = XshdSpan {
            position: self.position(node),
            begin: node.attribute("begin").map(str::to_owned),
            end: node.attribute("end").map(str::to_owned),
            multiline: self.boolean(node, "multiline")?.unwrap_or(false),
            rule_set: node
                .attribute("ruleSet")
                .map(XshdReference::from_attribute)
                .unwrap_or_default(),
            span_color: self.color_reference(node)?,
            ..XshdSpan::default()
        };
        for child in node
            .children()
            .filter(|child| child.is_element() && self.owns(*child))
        {
            match child.tag_name().name() {
                "Begin" => {
                    if span.begin.is_some() {
                        return Err(self.error(child, "Duplicate Begin regex"));
                    }
                    self.text_only(child)?;
                    span.begin = Some(element_text(child));
                    span.begin_type = XshdRegexType::IgnorePatternWhitespace;
                    span.begin_color = self.color_reference(child)?;
                }
                "End" => {
                    if span.end.is_some() {
                        return Err(self.error(child, "Duplicate End regex"));
                    }
                    self.text_only(child)?;
                    span.end = Some(element_text(child));
                    span.end_type = XshdRegexType::IgnorePatternWhitespace;
                    span.end_color = self.color_reference(child)?;
                }
                "RuleSet" => {
                    if !span.rule_set.is_none() {
                        return Err(self.error(child, "A span cannot have multiple rule sets or both inline and named rule sets"));
                    }
                    span.rule_set = XshdReference::inline(self.rule_set(child)?);
                }
                name => return Err(self.error(child, format!("Unknown Span element {name}"))),
            }
        }
        Ok(span)
    }

    fn color_reference(
        &self,
        node: Node<'_, '_>,
    ) -> Result<XshdReference<XshdColor>, HighlightingError> {
        if let Some(reference) = node.attribute("color") {
            Ok(XshdReference::from_attribute(reference))
        } else {
            let color = self.color(node)?;
            Ok(if color.has_style() {
                XshdReference::inline(color)
            } else {
                XshdReference::None
            })
        }
    }

    fn color(&self, node: Node<'_, '_>) -> Result<XshdColor, HighlightingError> {
        let font_size = node
            .attribute("fontSize")
            .map(|value| {
                let value: f64 = value
                    .parse()
                    .map_err(|_| self.error(node, "fontSize must be a finite positive number"))?;
                if value.is_finite() && value > 0.0 {
                    Ok(value)
                } else {
                    Err(self.error(node, "fontSize must be a finite positive number"))
                }
            })
            .transpose()?;
        let font_weight = node
            .attribute("fontWeight")
            .map(|value| {
                let lower = value.to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "thin"
                        | "extralight"
                        | "ultralight"
                        | "light"
                        | "semilight"
                        | "normal"
                        | "regular"
                        | "medium"
                        | "demibold"
                        | "semibold"
                        | "bold"
                        | "extrabold"
                        | "ultrabold"
                        | "black"
                        | "heavy"
                        | "extrablack"
                        | "ultrablack"
                ) || value.parse::<u16>().is_ok()
                {
                    Ok(value.to_owned())
                } else {
                    Err(self.error(node, format!("Unknown fontWeight {value}")))
                }
            })
            .transpose()?;
        let font_style = node
            .attribute("fontStyle")
            .map(|value| {
                if matches!(
                    value.to_ascii_lowercase().as_str(),
                    "normal" | "italic" | "oblique" | "0" | "1" | "2"
                ) {
                    Ok(value.to_owned())
                } else {
                    Err(self.error(node, format!("Unknown fontStyle {value}")))
                }
            })
            .transpose()?;
        Ok(XshdColor {
            position: self.position(node),
            foreground: node
                .attribute("foreground")
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            background: node
                .attribute("background")
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            font_family: node
                .attribute("fontFamily")
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            font_size,
            font_weight,
            font_style,
            underline: self.boolean(node, "underline")?,
            strikethrough: self.boolean(node, "strikethrough")?,
            ..XshdColor::default()
        })
    }
}

pub(super) fn element_text(node: Node<'_, '_>) -> String {
    node.children()
        .filter(|node| node.is_text())
        .filter_map(|node| node.text())
        .collect()
}
