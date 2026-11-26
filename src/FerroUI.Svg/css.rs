//! The `<style>` sheets of a document, with simple selectors: a type, `*`, `.class`, `#id` and
//! their compounds, in comma-separated lists. A selector with a combinator, an attribute or a
//! pseudo-class, and an at-rule, are not applied and are reported. Nothing is ever fetched: an
//! `@import` is reported as an external reference.

use crate::document::SvgError;
use roxmltree::{Document, Node, NodeId};
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;

/// The most rules of a document, each selector of a list counting as one.
const MOST_RULES: usize = 8_192;

/// The most rule candidates tried against elements: stops a document made of many universal rules
/// and many elements.
const MOST_MATCH_STEPS: usize = 4_000_000;

/// One `name: value` of a rule.
struct Declaration {
    name: String,
    value: String,
    is_important: bool,
}

#[derive(Default, PartialEq, Debug)]
struct Selector {
    element: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
}

impl Selector {
    /// What decides between rules: ids, then classes, then types.
    fn specificity(&self) -> (usize, usize, usize) {
        (usize::from(self.id.is_some()), self.classes.len(), usize::from(self.element.is_some()))
    }

    fn matches(&self, node: Node<'_, '_>) -> bool {
        if self.element.as_deref().is_some_and(|name| name != node.tag_name().name()) {
            return false;
        }
        if self.id.as_deref().is_some_and(|id| node.attribute("id") != Some(id)) {
            return false;
        }
        self.classes.iter().all(|class| classes_of(node).any(|own| own == class))
    }
}

fn classes_of<'a>(node: Node<'a, '_>) -> impl Iterator<Item = &'a str> {
    node.attribute("class").unwrap_or("").split_ascii_whitespace()
}

struct Rule {
    selector: Selector,
    declarations: Rc<[Declaration]>,
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

/// One compound selector; `None` for any other form.
fn parse_selector(text: &str) -> Option<Selector> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let mut selector = Selector::default();
    let mut rest = text;
    if let Some(after) = rest.strip_prefix('*') {
        rest = after;
    } else if !rest.starts_with(['.', '#']) {
        let end = rest.find(|c| !is_name_char(c)).unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        selector.element = Some(rest[..end].to_owned());
        rest = &rest[end..];
    }
    while let Some(marker) = rest.chars().next() {
        if marker != '.' && marker != '#' {
            return None;
        }
        let after = &rest[1..];
        let end = after.find(|c| !is_name_char(c)).unwrap_or(after.len());
        if end == 0 {
            return None;
        }
        let name = after[..end].to_owned();
        if marker == '.' {
            selector.classes.push(name);
        } else if selector.id.replace(name).is_some() {
            return None;
        }
        rest = &after[end..];
    }
    Some(selector)
}

/// The text without its comments.
fn without_comments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        result.push_str(&rest[..start]);
        result.push(' ');
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return result,
        }
    }
    result.push_str(rest);
    result
}

/// The index of the first `wanted` outside quotes and parentheses, as in `url(data:a;b)`.
fn find_outside(text: &str, wanted: &[char]) -> Option<usize> {
    let mut quote = None;
    let mut depth = 0usize;
    let mut escaped = false;
    for (index, c) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match quote {
            Some(open) => {
                if c == '\\' {
                    escaped = true;
                } else if c == open {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' => quote = Some(c),
                '\\' => escaped = true,
                '(' => depth += 1,
                ')' => depth = depth.saturating_sub(1),
                _ if depth == 0 && wanted.contains(&c) => return Some(index),
                _ => {}
            },
        }
    }
    None
}

/// The `name: value` pairs of a declaration block, with whether each is `!important`.
pub(crate) fn declarations(text: &str) -> impl Iterator<Item = (&str, &str, bool)> {
    let mut rest = Some(text);
    std::iter::from_fn(move || loop {
        let current = rest?;
        let part = match find_outside(current, &[';']) {
            Some(end) => {
                rest = Some(&current[end + 1..]);
                &current[..end]
            }
            None => {
                rest = None;
                current
            }
        };
        let Some(colon) = part.find(':') else { continue };
        let name = part[..colon].trim();
        if name.is_empty() {
            continue;
        }
        let value = part[colon + 1..].trim();
        return Some(match value.rfind('!') {
            Some(mark) if value[mark + 1..].trim().eq_ignore_ascii_case("important") => (name, value[..mark].trim(), true),
            _ => (name, value, false),
        });
    })
}

/// The last declaration of `name` in a `style` attribute, as the later one wins.
fn inline_declaration<'a>(style: &'a str, name: &str) -> Option<(&'a str, bool)> {
    let mut found: Option<(&'a str, bool)> = None;
    for (key, value, is_important) in declarations(style) {
        if key.eq_ignore_ascii_case(name) && (is_important || !found.is_some_and(|(_, important)| important)) {
            found = Some((value, is_important));
        }
    }
    found
}

fn note(found: &mut BTreeSet<String>, most: usize, feature: String) {
    if found.len() < most {
        found.insert(feature);
    }
}

/// The rules of one sheet, added to `rules`.
fn parse_sheet(text: &str, rules: &mut Vec<Rule>, found: &mut BTreeSet<String>, most: usize) -> Result<(), SvgError> {
    let text = without_comments(text);
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        if rest.starts_with('@') {
            let end = find_outside(rest, &[';', '{']).unwrap_or(rest.len());
            let name: String = rest[1..end].chars().take_while(|c| is_name_char(*c)).take(32).collect();
            if name.eq_ignore_ascii_case("import") {
                note(found, most, "external reference".to_owned());
            } else {
                note(found, most, format!("CSS at-rule @{name}"));
            }
            if rest[end..].starts_with('{') {
                rest = after_block(&rest[end..]);
            } else {
                rest = rest.get(end + 1..).unwrap_or("");
            }
            rest = rest.trim_start();
            continue;
        }
        let Some(open) = find_outside(rest, &['{']) else { break };
        let prelude = &rest[..open];
        let body_start = open + 1;
        let Some(length) = find_outside(&rest[body_start..], &['}']) else { break };
        let body = &rest[body_start..body_start + length];
        let block: Rc<[Declaration]> = declarations(body)
            .map(|(name, value, is_important)| Declaration {
                name: name.to_ascii_lowercase(),
                value: value.to_owned(),
                is_important,
            })
            .collect();
        for text in prelude.split(',') {
            match parse_selector(text) {
                Some(selector) => {
                    if rules.len() >= MOST_RULES {
                        return Err(SvgError::LimitExceeded("style rules"));
                    }
                    rules.push(Rule { selector, declarations: block.clone() });
                }
                None => note(found, most, format!("CSS selector {}", text.trim().chars().take(48).collect::<String>())),
            }
        }
        rest = rest[body_start + length + 1..].trim_start();
    }
    Ok(())
}

/// What follows the `{ ... }` block `text` starts with.
fn after_block(text: &str) -> &str {
    let mut depth = 0usize;
    let mut at = 0;
    loop {
        let Some(next) = find_outside(&text[at..], &['{', '}']) else { return "" };
        let index = at + next;
        if text[index..].starts_with('{') {
            depth += 1;
        } else {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return &text[index + 1..];
            }
        }
        at = index + 1;
    }
}

/// The rules that apply to each element of a document.
#[derive(Default)]
pub(crate) struct Cascade {
    /// Per element, the blocks of the rules that match it, the weakest first.
    matched: HashMap<NodeId, Vec<Rc<[Declaration]>>>,
    /// The properties the sheets set, for the report of those that are not applied.
    names: BTreeSet<String>,
}

impl Cascade {
    /// Reads the `<style>` elements of `document`. What is not applied is added to `found`, up to
    /// `most` entries.
    pub(crate) fn build(document: &Document<'_>, found: &mut BTreeSet<String>, most: usize) -> Result<Self, SvgError> {
        let mut rules = Vec::new();
        for node in document.descendants().filter(|node| node.is_element() && node.tag_name().name() == "style") {
            if node.attribute("type").is_some_and(|kind| !kind.trim().eq_ignore_ascii_case("text/css")) {
                note(found, most, "style sheet of another type".to_owned());
                continue;
            }
            let text: String = node.children().filter(Node::is_text).filter_map(|child| child.text()).collect();
            parse_sheet(&text, &mut rules, found, most)?;
        }
        let mut cascade = Cascade::default();
        if rules.is_empty() {
            return Ok(cascade);
        }
        for rule in &rules {
            cascade.names.extend(rule.declarations.iter().map(|declaration| declaration.name.clone()));
        }

        // Rules are looked up by the most selective part they name.
        let mut by_id: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut by_class: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut by_element: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut universal = Vec::new();
        for (index, rule) in rules.iter().enumerate() {
            let selector = &rule.selector;
            if let Some(id) = &selector.id {
                by_id.entry(id).or_default().push(index);
            } else if let Some(class) = selector.classes.first() {
                by_class.entry(class).or_default().push(index);
            } else if let Some(element) = &selector.element {
                by_element.entry(element).or_default().push(index);
            } else {
                universal.push(index);
            }
        }

        let mut steps = 0usize;
        let mut candidates = Vec::new();
        for node in document.descendants().filter(Node::is_element) {
            candidates.clear();
            candidates.extend_from_slice(&universal);
            if let Some(list) = node.attribute("id").and_then(|id| by_id.get(id)) {
                candidates.extend_from_slice(list);
            }
            if let Some(list) = by_element.get(node.tag_name().name()) {
                candidates.extend_from_slice(list);
            }
            for class in classes_of(node) {
                if let Some(list) = by_class.get(class) {
                    candidates.extend_from_slice(list);
                }
            }
            steps += candidates.len();
            if steps > MOST_MATCH_STEPS {
                return Err(SvgError::LimitExceeded("style rules"));
            }
            // A class written twice on an element lists its rules twice.
            candidates.sort_unstable();
            candidates.dedup();
            candidates.retain(|index| rules[*index].selector.matches(node));
            if candidates.is_empty() {
                continue;
            }
            // The later of two rules of the same weight wins; the sort is stable over the order.
            candidates.sort_by_key(|index| rules[*index].selector.specificity());
            cascade
                .matched
                .insert(node.id(), candidates.iter().map(|index| rules[*index].declarations.clone()).collect());
        }
        Ok(cascade)
    }

    /// The properties the sheets of the document set.
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(String::as_str)
    }

    fn from_sheets<'c>(&'c self, node: Node<'_, '_>, name: &str, is_important: bool) -> Option<&'c str> {
        self.matched.get(&node.id())?.iter().rev().find_map(|block| {
            block
                .iter()
                .rev()
                .find(|declaration| declaration.is_important == is_important && declaration.name.eq_ignore_ascii_case(name))
                .map(|declaration| declaration.value.as_str())
        })
    }

    /// The value of a property of `node`, in the order of the cascade: `!important` in its
    /// `style` attribute, then in the sheets, then its `style` attribute, the sheets and last its
    /// presentation attribute.
    pub(crate) fn get<'c>(&'c self, node: Node<'c, '_>, name: &str) -> Option<&'c str> {
        self.from_css(node, name).or_else(|| node.attribute(name).map(str::trim))
    }

    /// The value of a property set by CSS, for a property that is not a presentation attribute.
    pub(crate) fn from_css<'c>(&'c self, node: Node<'c, '_>, name: &str) -> Option<&'c str> {
        let inline = node.attribute("style").and_then(|style| inline_declaration(style, name));
        if let Some((value, true)) = inline {
            return Some(value);
        }
        if let Some(value) = self.from_sheets(node, name, true) {
            return Some(value);
        }
        if let Some((value, _)) = inline {
            return Some(value);
        }
        self.from_sheets(node, name, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cascade_of<'a>(document: &Document<'a>) -> (Cascade, BTreeSet<String>) {
        let mut found = BTreeSet::new();
        let cascade = Cascade::build(document, &mut found, 64).unwrap();
        (cascade, found)
    }

    #[test]
    fn selectors_are_parsed_or_refused() {
        let parsed = parse_selector("rect.a.b#c").unwrap();
        assert_eq!(parsed.element.as_deref(), Some("rect"));
        assert_eq!(parsed.classes, ["a", "b"]);
        assert_eq!(parsed.id.as_deref(), Some("c"));
        assert_eq!(parsed.specificity(), (1, 2, 1));
        assert_eq!(parse_selector("*").unwrap(), Selector::default());
        for text in ["g rect", "g > rect", "a:hover", "[x=y]", "#a#b", ".", ""] {
            assert!(parse_selector(text).is_none(), "{text}");
        }
    }

    #[test]
    fn a_semicolon_inside_a_url_does_not_end_the_declaration() {
        let list: Vec<_> = declarations("fill: url(data:a;b) ; stroke:red !important").collect();
        assert_eq!(list, [("fill", "url(data:a;b)", false), ("stroke", "red", true)]);
    }

    #[test]
    fn the_cascade_orders_attribute_sheet_and_style() {
        let text = r#"<svg xmlns="http://www.w3.org/2000/svg">
            <style>/* a comment */ rect { fill: red } .a { fill: green; stroke: blue !important } #b { fill: white }</style>
            <rect id="attribute" fill="black"/>
            <rect id="b" class="a" fill="black" style="stroke: gray"/>
            <rect id="inline" class="a" style="fill: yellow"/>
            <circle id="none" fill="black"/>
        </svg>"#;
        let document = Document::parse(text).unwrap();
        let (cascade, _) = cascade_of(&document);
        let by_id = |id: &str| document.descendants().find(|node| node.attribute("id") == Some(id)).unwrap();
        assert_eq!(cascade.get(by_id("attribute"), "fill"), Some("red"), "a sheet is over an attribute");
        assert_eq!(cascade.get(by_id("b"), "fill"), Some("white"), "the id is over the class");
        assert_eq!(cascade.get(by_id("b"), "stroke"), Some("blue"), "important is over the style attribute");
        assert_eq!(cascade.get(by_id("inline"), "fill"), Some("yellow"), "the style attribute is over a sheet");
        assert_eq!(cascade.get(by_id("none"), "fill"), Some("black"));
    }

    #[test]
    fn what_is_not_applied_is_reported_and_nothing_is_fetched() {
        let text = r#"<svg xmlns="http://www.w3.org/2000/svg"><style>
            @import url(https://example.invalid/a.css);
            @media print { rect { fill: red } }
            g rect { fill: blue }
            rect { fill: green }
        </style><rect id="r"/></svg>"#;
        let document = Document::parse(text).unwrap();
        let (cascade, found) = cascade_of(&document);
        assert!(found.contains("external reference"), "{found:?}");
        assert!(found.contains("CSS at-rule @media"), "{found:?}");
        assert!(found.contains("CSS selector g rect"), "{found:?}");
        let rect = document.descendants().find(|node| node.attribute("id") == Some("r")).unwrap();
        assert_eq!(cascade.get(rect, "fill"), Some("green"), "the block of @media is skipped");
    }

    #[test]
    fn a_document_of_many_universal_rules_stops_at_the_limit() {
        let rules = "* { fill: red } ".repeat(MOST_RULES);
        let elements = "<g/>".repeat(1_000);
        let text = format!(r#"<svg xmlns="http://www.w3.org/2000/svg"><style>{rules}</style>{elements}</svg>"#);
        let document = Document::parse(&text).unwrap();
        let mut found = BTreeSet::new();
        assert!(matches!(
            Cascade::build(&document, &mut found, 64),
            Err(SvgError::LimitExceeded("style rules"))
        ));
    }
}
