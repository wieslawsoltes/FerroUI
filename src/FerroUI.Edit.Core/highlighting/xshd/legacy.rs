//! Compatibility import for the historical RuleSets XML representation.
//! The imported result uses the same editable v2 tree as modern definitions.

use super::loader::{element_text, Parser};
use super::model::*;
use crate::highlighting::HighlightingError;
use roxmltree::Node;

pub(super) fn load_legacy(
    parser: &Parser<'_>,
    root: Node<'_, '_>,
) -> Result<XshdSyntaxDefinition, HighlightingError> {
    let mut syntax = XshdSyntaxDefinition {
        name: root.attribute("name").map(str::to_owned),
        extensions: root
            .attribute("extensions")
            .map(|value| value.split([';', '|']).map(str::to_owned).collect())
            .unwrap_or_default(),
        elements: Vec::new(),
    };
    let digits = root.children().find(|node| node.has_tag_name("Digits"));
    let mut main = None;
    for node in root
        .descendants()
        .filter(|node| node.has_tag_name("RuleSet"))
    {
        let mut set = import_set(parser, node)?;
        if let Some(digits) = digits {
            set.elements.push(XshdElement::Rule(XshdRule {
                position: parser.position(digits),
                regex: Some(
                    r"\b0[xX][0-9a-fA-F]+|(\b\d+(\.[0-9]+)?|\.[0-9]+)([eE][+-]?[0-9]+)?".into(),
                ),
                regex_type: XshdRegexType::IgnorePatternWhitespace,
                color: color_reference(parser, digits)?,
            }));
        }
        if set.name.is_none() {
            main = Some(syntax.elements.len());
        }
        syntax.elements.push(XshdElement::RuleSet(set));
    }
    if let (Some(extends), Some(main)) = (root.attribute("extends"), main) {
        if let XshdElement::RuleSet(set) = &mut syntax.elements[main] {
            set.elements.push(XshdElement::Import(XshdImport {
                position: parser.position(root),
                rule_set: XshdReference::external(extends, ""),
            }));
        }
    }
    Ok(syntax)
}

fn import_set(parser: &Parser<'_>, node: Node<'_, '_>) -> Result<XshdRuleSet, HighlightingError> {
    let mut set = XshdRuleSet {
        position: parser.position(node),
        name: node.attribute("name").map(str::to_owned),
        ignore_case: parser.boolean(node, "ignorecase")?,
        elements: Vec::new(),
    };
    let escape = escape_character(parser, node)?;
    if let Some(reference) = node.attribute("reference") {
        set.elements.push(XshdElement::Import(XshdImport {
            position: parser.position(node),
            rule_set: XshdReference::external(reference, ""),
        }));
    }
    // The legacy importer processes categories in this order, independently of
    // their XML interleaving. Ordering inside each category is preserved.
    for keywords in node
        .descendants()
        .filter(|node| node.has_tag_name("KeyWords"))
    {
        let words: Vec<_> = keywords
            .descendants()
            .filter(|node| node.has_tag_name("Key"))
            .filter_map(|node| node.attribute("word"))
            .filter(|word| !word.is_empty())
            .map(str::to_owned)
            .collect();
        if !words.is_empty() {
            set.elements.push(XshdElement::Keywords(XshdKeywords {
                position: parser.position(keywords),
                color: color_reference(parser, keywords)?,
                words,
            }));
        }
    }
    for span in node.descendants().filter(|node| node.has_tag_name("Span")) {
        set.elements
            .push(XshdElement::Span(import_span(parser, span, escape)?));
    }
    for kind in ["MarkPrevious", "MarkFollowing"] {
        for marker in node.descendants().filter(|node| node.has_tag_name(kind)) {
            let following = kind == "MarkFollowing";
            let include_marker = parser.boolean(marker, "markmarker")?.unwrap_or(false);
            let marker_text = escape_literal(&element_text(marker));
            let pattern = match (following, include_marker) {
                (true, true) => format!(r"{marker_text}\s*[\d\w_]+"),
                (true, false) => format!(r"(?<={marker_text}\s*)[\d\w_]+"),
                (false, true) => format!(r"[\d\w_]+\s*{marker_text}"),
                (false, false) => format!(r"[\d\w_]+(?=\s*{marker_text})"),
            };
            set.elements.push(XshdElement::Rule(XshdRule {
                position: parser.position(marker),
                regex: Some(pattern),
                regex_type: XshdRegexType::IgnorePatternWhitespace,
                color: color_reference(parser, marker)?,
            }));
        }
    }
    Ok(set)
}

fn escape_character(
    parser: &Parser<'_>,
    node: Node<'_, '_>,
) -> Result<Option<char>, HighlightingError> {
    node.attribute("escapecharacter")
        .map(|value| {
            value
                .chars()
                .next()
                .ok_or_else(|| parser.error(node, "escapecharacter cannot be empty"))
        })
        .transpose()
}

fn color_reference(
    parser: &Parser<'_>,
    node: Node<'_, '_>,
) -> Result<XshdReference<XshdColor>, HighlightingError> {
    let color = XshdColor {
        position: parser.position(node),
        foreground: node.attribute("color").map(str::to_owned),
        background: node.attribute("bgcolor").map(str::to_owned),
        font_weight: parser
            .boolean(node, "bold")?
            .map(|bold| if bold { "bold" } else { "normal" }.into()),
        font_style: parser
            .boolean(node, "italic")?
            .map(|italic| if italic { "italic" } else { "normal" }.into()),
        ..XshdColor::default()
    };
    Ok(if color.has_style() {
        XshdReference::inline(color)
    } else {
        XshdReference::None
    })
}

fn import_span(
    parser: &Parser<'_>,
    node: Node<'_, '_>,
    inherited_escape: Option<char>,
) -> Result<XshdSpan, HighlightingError> {
    let begin = node
        .children()
        .find(|child| child.has_tag_name("Begin"))
        .ok_or_else(|| parser.error(node, "Legacy Span needs Begin"))?;
    let end = node.children().find(|child| child.has_tag_name("End"));
    let begin_text = element_text(begin);
    let end_text = end.map(element_text);
    let mut span = XshdSpan {
        position: parser.position(node),
        begin: Some(import_regex(
            &begin_text,
            parser.boolean(begin, "singleword")?.unwrap_or(false),
            parser.boolean(begin, "startofline")?,
            parser.position(begin),
        )?),
        begin_type: XshdRegexType::IgnorePatternWhitespace,
        end: end
            .map(|end| {
                import_regex(
                    end_text.as_deref().unwrap_or(""),
                    parser.boolean(end, "singleword")?.unwrap_or(false),
                    None,
                    parser.position(end),
                )
            })
            .transpose()?,
        end_type: XshdRegexType::IgnorePatternWhitespace,
        multiline: !parser.boolean(node, "stopateol")?.unwrap_or(false),
        rule_set: node
            .attribute("rule")
            .map(XshdReference::local)
            .unwrap_or_default(),
        span_color: color_reference(parser, node)?,
        begin_color: color_reference(parser, begin)?,
        end_color: end
            .map(|end| color_reference(parser, end))
            .transpose()?
            .unwrap_or_default(),
    };
    if let Some(escape) = escape_character(parser, node)?.or(inherited_escape) {
        if escape != '\0' {
            let doubled_end = end_text.as_deref().is_some_and(|text| {
                let mut chars = text.chars();
                chars.next() == Some(escape) && chars.next().is_none()
            });
            let escaping = XshdSpan {
                begin: Some(if doubled_end {
                    escape_literal(&format!("{escape}{escape}"))
                } else {
                    escape_literal(&escape.to_string())
                }),
                end: Some(if doubled_end {
                    String::new()
                } else {
                    ".".into()
                }),
                ..XshdSpan::default()
            };
            let mut rules = XshdRuleSet {
                position: parser.position(node),
                elements: vec![XshdElement::Span(escaping)],
                ..XshdRuleSet::default()
            };
            if !span.rule_set.is_none() {
                rules.elements.push(XshdElement::Import(XshdImport {
                    position: parser.position(node),
                    rule_set: std::mem::take(&mut span.rule_set),
                }));
            }
            span.rule_set = XshdReference::inline(rules);
        }
    }
    Ok(span)
}

/// The legacy mini-language quotes ordinary characters and expands only its
/// documented @ escapes; it is not an arbitrary regex pass-through.
pub(super) fn import_regex(
    expression: &str,
    single_word: bool,
    start_of_line: Option<bool>,
    position: XshdPosition,
) -> Result<String, HighlightingError> {
    let mut result = match start_of_line {
        Some(true) => r"(?<=^\s*)".to_owned(),
        Some(false) => r"(?<!^\s*)".to_owned(),
        None if single_word => r"\b".to_owned(),
        None => String::new(),
    };
    let mut chars = expression.chars();
    while let Some(character) = chars.next() {
        if character != '@' {
            result.push_str(&escape_literal(&character.to_string()));
            continue;
        }
        match chars
            .next()
            .ok_or_else(|| position.error("Unexpected end of @ sequence; use @@ for a literal @"))?
        {
            'C' => result.push_str(r"[^\w\d_]"),
            '@' => result.push('@'),
            kind @ ('!' | '-') => {
                let mut literal = String::new();
                for character in chars.by_ref() {
                    if character == '@' {
                        break;
                    }
                    literal.push(character);
                }
                result.push_str(if kind == '!' { "(?!" } else { "(?<!" });
                result.push_str(&escape_literal(&literal));
                result.push(')');
            }
            character => return Err(position.error(format!("Unknown @{character} sequence"))),
        }
    }
    if single_word {
        result.push_str(r"\b");
    }
    Ok(result)
}

fn escape_literal(text: &str) -> String {
    let mut result = String::new();
    for character in text.chars() {
        match character {
            ' ' => result.push_str(r"\x20"),
            '\t' => result.push_str(r"\t"),
            '\n' => result.push_str(r"\n"),
            '\r' => result.push_str(r"\r"),
            '\u{000b}' => result.push_str(r"\x0B"),
            '\u{000c}' => result.push_str(r"\x0C"),
            '#' => result.push_str(r"\#"),
            _ => result.push_str(&regex::escape(&character.to_string())),
        }
    }
    result
}
