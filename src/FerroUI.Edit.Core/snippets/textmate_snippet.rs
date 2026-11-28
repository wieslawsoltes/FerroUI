use super::textmate_transform::TextMateTransform;
use super::{Snippet, SnippetElement, SnippetError, SnippetTextTransform};
use std::collections::{BTreeMap, BTreeSet};
use std::iter::Peekable;
use std::str::CharIndices;

/// Compiled package snippet syntax. Binding a snippet to the selection is
/// separate from parsing so completion lists can reuse the compiled body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextMateSnippet {
    nodes: Vec<Node>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Node {
    Text(String),
    TabStop(usize, Option<FieldDefault>),
    TransformedTabStop(usize, TextMateTransform),
    Variable(String, Option<Vec<Node>>),
    TransformedVariable(String, TextMateTransform),
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FieldDefault {
    Content(Vec<Node>),
    Choices(Vec<String>),
}

impl FieldDefault {
    fn nodes(&self) -> &[Node] {
        match self {
            Self::Content(nodes) => nodes,
            Self::Choices(_) => &[],
        }
    }
}

impl TextMateSnippet {
    /// Parses tab stops, nested defaults, choice lists, variables, transforms and escapes.
    pub fn parse(source: &str) -> Result<Self, SnippetError> {
        Ok(Self {
            nodes: parse_nodes(&mut source.char_indices().peekable(), None, 0)?,
        })
    }

    pub fn instantiate(&self, selected_text: &str) -> Result<Snippet, SnippetError> {
        self.instantiate_with_variables(selected_text, |_| Ok(None))
    }

    /// Supplies values from the host when the snippet is expanded. `None`
    /// uses the variable's default, an empty standard variable, or an editable
    /// placeholder for an unknown name. A supplied string is literal text.
    pub fn instantiate_with_variables(
        &self,
        selected_text: &str,
        mut resolve: impl FnMut(&str) -> Result<Option<String>, SnippetError>,
    ) -> Result<Snippet, SnippetError> {
        let mut last_id = max_id(&self.nodes);
        let resolved = resolve_variables(
            &self.nodes,
            selected_text,
            &mut resolve,
            &mut last_id,
            &mut BTreeMap::new(),
        )?;
        let mut defaults = BTreeMap::new();
        let mut plain_stops = BTreeSet::new();
        collect(&resolved, &mut defaults, &mut plain_stops);
        Ok(Snippet {
            elements: lower(&resolved, &defaults, &plain_stops, &mut BTreeSet::new())?,
        })
    }
}

fn flush(text: &mut String, nodes: &mut Vec<Node>) {
    if !text.is_empty() {
        nodes.push(Node::Text(std::mem::take(text)));
    }
}

fn parse_nodes(
    input: &mut Peekable<CharIndices<'_>>,
    opener: Option<usize>,
    depth: usize,
) -> Result<Vec<Node>, SnippetError> {
    if depth >= 256 {
        return Err(SnippetError::InvalidExpansion(
            "snippet parser recursion limit exceeded",
        ));
    }
    let mut nodes = Vec::new();
    let mut text = String::new();
    while let Some((offset, ch)) = input.next() {
        match ch {
            '}' if opener.is_some() => {
                flush(&mut text, &mut nodes);
                return Ok(nodes);
            }
            '\\' if input
                .peek()
                .is_some_and(|&(_, next)| matches!(next, '$' | '}' | '\\')) =>
            {
                text.push(input.next().unwrap().1);
            }
            '$' => {
                if let Some(node) = parse_marker(input, offset, depth)? {
                    flush(&mut text, &mut nodes);
                    nodes.push(node);
                } else {
                    text.push('$');
                }
            }
            _ => text.push(ch),
        }
    }
    if let Some(offset) = opener {
        return Err(SnippetError::InvalidSyntax(offset));
    }
    flush(&mut text, &mut nodes);
    Ok(nodes)
}

fn parse_marker(
    input: &mut Peekable<CharIndices<'_>>,
    offset: usize,
    depth: usize,
) -> Result<Option<Node>, SnippetError> {
    let braced = input.peek().is_some_and(|&(_, ch)| ch == '{');
    if braced {
        input.next();
    }
    let identifier = |ch: char| ch.is_ascii_alphabetic() || ch == '_';
    let Some(&(_, first)) = input
        .peek()
        .filter(|(_, ch)| ch.is_ascii_digit() || identifier(*ch))
    else {
        return if braced {
            Err(SnippetError::InvalidSyntax(offset))
        } else {
            Ok(None)
        };
    };
    let number = first.is_ascii_digit();
    let mut name = String::new();
    while input
        .peek()
        .is_some_and(|&(_, ch)| ch.is_ascii_digit() || (!number && identifier(ch)))
    {
        name.push(input.next().unwrap().1);
    }
    let default = if braced {
        match input.next() {
            Some((_, '}')) => None,
            Some((_, ':')) => Some(parse_nodes(input, Some(offset), depth + 1)?),
            Some((_, '/')) => {
                let transform = TextMateTransform::parse(input, offset)?;
                return Ok(Some(if number {
                    Node::TransformedTabStop(
                        name.parse()
                            .map_err(|_| SnippetError::InvalidSyntax(offset))?,
                        transform,
                    )
                } else {
                    Node::TransformedVariable(name, transform)
                }));
            }
            Some((_, '|')) if number => {
                let id = name
                    .parse()
                    .map_err(|_| SnippetError::InvalidSyntax(offset))?;
                if id == 0 {
                    return Err(SnippetError::InvalidSyntax(offset));
                }
                let choices = parse_choices(input, offset)?;
                return Ok(Some(Node::TabStop(
                    id,
                    Some(FieldDefault::Choices(choices)),
                )));
            }
            _ => return Err(SnippetError::InvalidSyntax(offset)),
        }
    } else {
        None
    };
    let node = if number {
        Node::TabStop(
            name.parse()
                .map_err(|_| SnippetError::InvalidSyntax(offset))?,
            default.map(FieldDefault::Content),
        )
    } else {
        Node::Variable(name, default)
    };
    Ok(Some(node))
}

fn parse_choices(
    input: &mut Peekable<CharIndices<'_>>,
    offset: usize,
) -> Result<Vec<String>, SnippetError> {
    let mut choices = Vec::new();
    let mut value = String::new();
    while let Some((_, ch)) = input.next() {
        match ch {
            '\\' if input
                .peek()
                .is_some_and(|&(_, next)| matches!(next, ',' | '|' | '\\')) =>
            {
                value.push(input.next().unwrap().1);
            }
            ',' if !value.is_empty() => choices.push(std::mem::take(&mut value)),
            '|' if !value.is_empty() && input.next().is_some_and(|(_, next)| next == '}') => {
                choices.push(value);
                return Ok(choices);
            }
            ',' | '|' => break,
            _ => value.push(ch),
        }
    }
    Err(SnippetError::InvalidSyntax(offset))
}

fn max_id(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| match node {
            Node::TabStop(id, default) => {
                (*id).max(default.as_ref().map_or(0, |value| max_id(value.nodes())))
            }
            Node::TransformedTabStop(id, _) => *id,
            Node::Variable(_, default) => default.as_deref().map_or(0, max_id),
            Node::Text(_) | Node::TransformedVariable(..) => 0,
        })
        .max()
        .unwrap_or(0)
}

fn resolve_variables(
    nodes: &[Node],
    selected: &str,
    resolve: &mut impl FnMut(&str) -> Result<Option<String>, SnippetError>,
    last_id: &mut usize,
    unknown: &mut BTreeMap<String, usize>,
) -> Result<Vec<Node>, SnippetError> {
    let mut resolved = Vec::new();
    for node in nodes {
        match node {
            Node::Text(text) => resolved.push(Node::Text(text.clone())),
            Node::TransformedTabStop(..) => resolved.push(node.clone()),
            Node::TransformedVariable(name, transform) => {
                let value = variable_value(name, selected, resolve)?.unwrap_or_default();
                resolved.push(Node::Text(transform.apply(&value)?));
            }
            Node::TabStop(id, default) => resolved.push(Node::TabStop(
                *id,
                match default {
                    Some(FieldDefault::Content(nodes)) => Some(FieldDefault::Content(
                        resolve_variables(nodes, selected, resolve, last_id, unknown)?,
                    )),
                    other => other.clone(),
                },
            )),
            Node::Variable(name, fallback) => {
                let value = variable_value(name, selected, resolve)?;
                if let Some(value) = value {
                    resolved.push(Node::Text(value));
                } else if let Some(fallback) = fallback {
                    resolved.extend(resolve_variables(
                        fallback, selected, resolve, last_id, unknown,
                    )?);
                } else if !is_standard_variable(name) {
                    let id = if let Some(id) = unknown.get(name) {
                        *id
                    } else {
                        *last_id = last_id
                            .checked_add(1)
                            .ok_or(SnippetError::InvalidExpansion(
                                "snippet tab stop index overflow",
                            ))?;
                        unknown.insert(name.clone(), *last_id);
                        *last_id
                    };
                    resolved.push(Node::TabStop(
                        id,
                        Some(FieldDefault::Content(vec![Node::Text(name.clone())])),
                    ));
                }
            }
        }
    }
    Ok(resolved)
}

fn variable_value(
    name: &str,
    selected: &str,
    resolve: &mut impl FnMut(&str) -> Result<Option<String>, SnippetError>,
) -> Result<Option<String>, SnippetError> {
    match name {
        "TM_SELECTED_TEXT" | "SELECTION" => Ok((!selected.is_empty()).then(|| selected.to_owned())),
        _ => resolve(name),
    }
}

fn is_standard_variable(name: &str) -> bool {
    matches!(
        name,
        "SELECTION"
            | "TM_SELECTED_TEXT"
            | "TM_CURRENT_LINE"
            | "TM_CURRENT_WORD"
            | "TM_LINE_INDEX"
            | "TM_LINE_NUMBER"
            | "TM_FILENAME"
            | "TM_FILENAME_BASE"
            | "TM_DIRECTORY"
            | "TM_DIRECTORY_BASE"
            | "TM_FILEPATH"
            | "RELATIVE_FILEPATH"
            | "CURSOR_INDEX"
            | "CURSOR_NUMBER"
            | "LINE_COMMENT"
            | "BLOCK_COMMENT_START"
            | "BLOCK_COMMENT_END"
            | "CLIPBOARD"
            | "WORKSPACE_NAME"
            | "WORKSPACE_FOLDER"
            | "CURRENT_YEAR"
            | "CURRENT_YEAR_SHORT"
            | "CURRENT_MONTH"
            | "CURRENT_MONTH_NAME"
            | "CURRENT_MONTH_NAME_SHORT"
            | "CURRENT_DATE"
            | "CURRENT_DAY_NAME"
            | "CURRENT_DAY_NAME_SHORT"
            | "CURRENT_HOUR"
            | "CURRENT_MINUTE"
            | "CURRENT_SECOND"
            | "CURRENT_MILLISECOND"
            | "CURRENT_SECONDS_UNIX"
            | "CURRENT_MILLISECONDS_UNIX"
            | "CURRENT_TIMEZONE_OFFSET"
            | "CURRENT_TIMEZONE_NAME"
            | "RANDOM"
            | "RANDOM_HEX"
            | "UUID"
    )
}

fn collect<'a>(
    nodes: &'a [Node],
    defaults: &mut BTreeMap<usize, &'a FieldDefault>,
    plain_stops: &mut BTreeSet<usize>,
) {
    for node in nodes {
        if let Node::TabStop(id, default) = node {
            plain_stops.insert(*id);
            if let Some(children) = default {
                if *id != 0 {
                    defaults.entry(*id).or_insert(children);
                }
                collect(children.nodes(), defaults, plain_stops);
            }
        }
    }
}

/// The nodes and bytes one default may expand to. Defaults can refer to each other, so a short
/// snippet can otherwise stand for an exponential expansion.
const MAX_DEFAULT_EXPANSION: usize = 1 << 20;

fn spend_expansion(budget: &mut usize, amount: usize) -> Result<(), SnippetError> {
    *budget = budget
        .checked_sub(amount)
        .ok_or(SnippetError::InvalidExpansion("snippet default expansion limit exceeded"))?;
    Ok(())
}

fn initial_text(
    nodes: &[Node],
    defaults: &BTreeMap<usize, &FieldDefault>,
    active: &mut BTreeSet<usize>,
    depth: usize,
    budget: &mut usize,
    text: &mut String,
) -> Result<(), SnippetError> {
    if depth >= 256 {
        return Err(SnippetError::InvalidExpansion(
            "snippet default recursion limit exceeded",
        ));
    }
    for node in nodes {
        spend_expansion(budget, 1)?;
        match node {
            Node::Text(value) => {
                spend_expansion(budget, value.len())?;
                text.push_str(value);
            }
            Node::TabStop(0, children) => initial_text(
                children
                    .as_ref()
                    .map(FieldDefault::nodes)
                    .unwrap_or_default(),
                defaults,
                active,
                depth + 1,
                budget,
                text,
            )?,
            Node::TabStop(id, _) | Node::TransformedTabStop(id, _) => match defaults.get(id) {
                Some(FieldDefault::Choices(choices)) => {
                    spend_expansion(budget, choices[0].len())?;
                    text.push_str(&choices[0]);
                }
                Some(FieldDefault::Content(children)) => {
                    if !active.insert(*id) {
                        return Err(SnippetError::InvalidExpansion("cyclic snippet default"));
                    }
                    initial_text(children, defaults, active, depth + 1, budget, text)?;
                    active.remove(id);
                }
                None => {}
            },
            Node::Variable(..) | Node::TransformedVariable(..) => {
                unreachable!("variables were resolved first")
            }
        }
    }
    Ok(())
}

fn lower(
    nodes: &[Node],
    defaults: &BTreeMap<usize, &FieldDefault>,
    plain_stops: &BTreeSet<usize>,
    declared: &mut BTreeSet<usize>,
) -> Result<Vec<SnippetElement>, SnippetError> {
    let mut elements = Vec::new();
    for node in nodes {
        match node {
            Node::Text(value) => elements.push(SnippetElement::Text(value.clone())),
            Node::TabStop(0, default) => {
                elements.push(SnippetElement::Caret);
                elements.extend(lower(
                    default
                        .as_ref()
                        .map(FieldDefault::nodes)
                        .unwrap_or_default(),
                    defaults,
                    plain_stops,
                    declared,
                )?);
            }
            Node::TransformedTabStop(0, _) => elements.push(SnippetElement::Caret),
            Node::TransformedTabStop(id, transform) => {
                let editable = !plain_stops.contains(id) && declared.insert(*id);
                let transform = transform.clone();
                elements.push(SnippetElement::TransformOnExit {
                    id: *id,
                    editable,
                    transform: SnippetTextTransform::try_new(move |text| transform.apply(text)),
                });
            }
            Node::TabStop(id, _) if declared.insert(*id) => {
                let default = defaults.get(id).copied();
                if let Some(FieldDefault::Choices(choices)) = default {
                    elements.push(SnippetElement::Choice {
                        id: *id,
                        choices: choices.clone(),
                    });
                    continue;
                }
                let default = default.map(FieldDefault::nodes).unwrap_or_default();
                let mut text = String::new();
                let mut budget = MAX_DEFAULT_EXPANSION;
                initial_text(default, defaults, &mut BTreeSet::from([*id]), 0, &mut budget, &mut text)?;
                if default
                    .iter()
                    .any(|node| matches!(node, Node::TabStop(..) | Node::TransformedTabStop(..)))
                {
                    elements.push(SnippetElement::NestedReplaceable {
                        id: *id,
                        text,
                        children: lower(default, defaults, plain_stops, declared)?,
                    });
                } else {
                    elements.push(SnippetElement::Replaceable { id: *id, text });
                }
            }
            Node::TabStop(id, _) => elements.push(SnippetElement::Bound { id: *id }),
            Node::Variable(..) | Node::TransformedVariable(..) => {
                unreachable!("variables were resolved first")
            }
        }
    }
    Ok(elements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{DocumentError, TextDocument};
    use std::rc::Rc;

    #[test]
    fn defaults_that_refer_to_each_other_have_an_expansion_limit() {
        for last in ["${40:x}", "$40"] {
            let mut source = String::new();
            for id in 1..40 {
                let next = id + 1;
                source.push_str(&format!("${{{id}:${next}${next}}}"));
            }
            source.push_str(last);
            let result = TextMateSnippet::parse(&source).and_then(|snippet| snippet.instantiate(""));
            assert!(matches!(result, Err(SnippetError::InvalidExpansion(_))), "{last}");
        }
    }

    #[test]
    fn variable_transforms_support_captures_conditions_flags_and_utf16() {
        let source = r"${TM_FILENAME/(.*)\..+$/${1:/pascalcase}/} | ${TM_SELECTED_TEXT/./x/g}/${TM_SELECTED_TEXT/./x/gu}/${TM_SELECTED_TEXT/./$0/g} | ${COUNT/(\d+)|(x)/${1:+count}${2:?yes:no}:${1:-missing}/g} | ${ABSENT/not-present/${1:-fallback}/} | ${GREEK/[\p{Script=Greek}&&\p{Uppercase}]/_/gv} | ${STICKY/\d/_/gy}";
        let snippet = TextMateSnippet::parse(source)
            .unwrap()
            .instantiate_with_variables("😀", |name| {
                Ok(match name {
                    "TM_FILENAME" => Some("main.window.rs".to_owned()),
                    "COUNT" => Some("7x".to_owned()),
                    "GREEK" => Some("AΩωΒ".to_owned()),
                    "STICKY" => Some("12x3".to_owned()),
                    _ => None,
                })
            })
            .unwrap();
        assert_eq!(
            snippet.prepare("").unwrap().text(),
            "MainWindow | xx/x/😀 | countno:7yes:missing | fallback | A_ω_ | __x3"
        );
        assert!(TextMateSnippet::parse("${X/./$0/ii}").is_err());
        assert!(TextMateSnippet::parse("${X/[/$0/g}").is_err());
    }

    #[test]
    fn variable_transform_formats_support_case_operations_and_escaped_slashes() {
        let source = r"${TM_SELECTED_TEXT/(.*)/${1:/downcase}|${1:/capitalize}|${1:/camelcase}|${1:/snakecase}|${1:/kebabcase}/} | ${PATH/\//\\\//g}";
        let snippet = TextMateSnippet::parse(source)
            .unwrap()
            .instantiate_with_variables("arc XMLServer", |name| {
                Ok((name == "PATH").then(|| "a/b/c".to_owned()))
            })
            .unwrap();
        assert_eq!(
            snippet.prepare("").unwrap().text(),
            r"arc xmlserver|Arc XMLServer|arcXMLServer|arc_xmlserver|arc-xml-server | a\/b\/c"
        );
    }

    #[test]
    fn placeholder_transforms_update_on_exit_and_propagate_into_enclosing_mirrors() {
        let snippet = TextMateSnippet::parse(
            r"${1:wrap(${2/(.*)/${1:/upcase}/}, ${2:name}, ${2/^/tag-/})} = $1 / ${3:end}$0",
        )
        .unwrap()
        .instantiate("")
        .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        session.try_next_element(false).unwrap().unwrap();
        session.replace_active("worker").unwrap();
        let before = "wrap(worker, worker, worker) = wrap(worker, worker, worker) / end";
        assert_eq!(document.text(), before);
        let next = session.try_next_element(false).unwrap().unwrap();
        assert_eq!(document.get_text(next.offset, next.length).unwrap(), "end");
        let after = "wrap(WORKER, worker, tag-worker) = wrap(WORKER, worker, tag-worker) / end";
        assert_eq!(document.text(), after);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), before);
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), after);
    }

    #[test]
    fn protected_transform_blocks_navigation_before_any_edits() {
        let snippet =
            TextMateSnippet::parse(r"${1:word}/${1/(.*)/${1:/upcase}/}/${1/^/tag-/}/${2:next}$0")
                .unwrap()
                .instantiate("")
                .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        session.set_edit_validator(Some(Rc::new(|edits| {
            if edits.iter().any(|edit| edit.offset == 10) {
                Err(DocumentError::ReadOnly)
            } else {
                Ok(())
            }
        })));
        let version = document.version();
        assert!(matches!(
            session.try_next_element(false),
            Err(SnippetError::Document(DocumentError::ReadOnly))
        ));
        assert_eq!(document.version(), version);
        assert_eq!(session.active_placeholder(), Some(1));
        session.set_edit_validator(None);
        session.try_next_element(false).unwrap().unwrap();
        assert_eq!(document.text(), "word/WORD/tag-word/next");
    }

    #[test]
    fn a_transform_only_stop_remains_editable() {
        let snippet = TextMateSnippet::parse(r"${1/(.*)/${1:/upcase}/}$0")
            .unwrap()
            .instantiate("")
            .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        session.replace_active("word").unwrap();
        assert_eq!(document.text(), "word");
        assert_eq!(
            session.try_next_element(true).unwrap(),
            session.active_segment()
        );
        assert_eq!(document.text(), "word");
        assert!(session.try_next_element(false).unwrap().is_none());
        assert_eq!(document.text(), "WORD");
        assert_eq!(session.caret_offset(), Some(4));
    }

    #[test]
    fn choices_preserve_escapes_order_and_nested_mirrors_through_history() {
        let snippet = TextMateSnippet::parse(
            r"$1 / ${1:call(${2|item,a\,b,😀\|pipe,back\\slash|})} / $2 / ${3:end}$0",
        )
        .unwrap()
        .instantiate("")
        .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        let before = "call(item) / call(item) / item / end";
        assert_eq!(document.text(), before);
        session.next_element(false).unwrap();
        assert_eq!(
            &*session.active_choices().unwrap(),
            &["item", "a,b", "😀|pipe", r"back\slash"]
        );
        session.choose_active(2).unwrap();
        let after = "call(😀|pipe) / call(😀|pipe) / 😀|pipe / end";
        assert_eq!(document.text(), after);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), before);
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), after);
        assert_eq!(session.active_placeholder(), Some(2));
        assert!(TextMateSnippet::parse("${1|missing terminator}").is_err());
        assert!(TextMateSnippet::parse("${0|a,b|}").is_err());
    }

    #[test]
    fn a_protected_enclosing_mirror_rejects_choice_before_changing_text() {
        let snippet = TextMateSnippet::parse("${1:call(${2|item,other|})} / $1 / $2")
            .unwrap()
            .instantiate("")
            .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        session.next_element(false).unwrap();
        session.set_edit_validator(Some(Rc::new(|edits| {
            if edits.iter().any(|edit| edit.offset == 13) {
                Err(DocumentError::ReadOnly)
            } else {
                Ok(())
            }
        })));
        let version = document.version();
        assert!(matches!(
            session.choose_active(1),
            Err(SnippetError::Document(DocumentError::ReadOnly))
        ));
        assert_eq!(document.version(), version);
        assert_eq!(document.text(), "call(item) / call(item) / item");
        assert_eq!(session.active_placeholder(), Some(2));
    }

    #[test]
    fn a_mirror_inside_a_parent_contributes_to_that_parents_mirror() {
        let snippet = TextMateSnippet::parse("${1:call(${2|item,other|}, $2)} / $1")
            .unwrap()
            .instantiate("")
            .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        session.next_element(false).unwrap();
        session.choose_active(1).unwrap();
        assert_eq!(document.text(), "call(other, other) / call(other, other)");
    }

    #[test]
    fn selected_text_resolves_nested_defaults_and_preserves_literal_dollars() {
        let snippet = TextMateSnippet::parse(
            r#"\$this = $${1:value}; [${TM_SELECTED_TEXT:${2:text}}] ${3:\Throwable}$0"#,
        )
        .unwrap();
        let without = snippet.instantiate("").unwrap().prepare("").unwrap();
        assert_eq!(without.text(), r#"$this = $value; [text] \Throwable"#);
        let selected = snippet
            .instantiate("😀 label")
            .unwrap()
            .prepare("")
            .unwrap();
        assert_eq!(selected.text(), r#"$this = $value; [😀 label] \Throwable"#);
        assert!(TextMateSnippet::parse("${1:unterminated").is_err());
        assert!(TextMateSnippet::parse("${1:$2} ${2:$1}")
            .unwrap()
            .instantiate("")
            .is_err());
    }

    #[test]
    fn variable_values_are_literal_and_unset_standards_keep_their_defaults() {
        let template = TextMateSnippet::parse(
            "${WORKSPACE_NAME:$SHOULD_NOT_RUN}/${CLIPBOARD:${1:clipboard}}/$TM_FILEPATH/$UNKNOWN/$UNKNOWN",
        )
        .unwrap();
        let body = template
            .instantiate_with_variables("", |name| match name {
                "WORKSPACE_NAME" => Ok(Some("$7😀".to_owned())),
                "SHOULD_NOT_RUN" => panic!("an overridden default was evaluated"),
                _ => Ok(None),
            })
            .unwrap();
        let document = TextDocument::new();
        let session = body.insert(document.clone(), 0, "").unwrap();
        assert_eq!(document.text(), "$7😀/clipboard//UNKNOWN/UNKNOWN");
        let unknown = session.next_element(false).unwrap();
        document
            .replace(unknown.offset, unknown.length, "custom")
            .unwrap();
        assert_eq!(document.text(), "$7😀/clipboard//custom/custom");
    }

    #[test]
    fn nested_fields_mirror_child_edits_and_restore_removed_children_on_undo() {
        let snippet = TextMateSnippet::parse("${1:extends ${2:Base}} / $1 / $2 / ${3:Body}$0")
            .unwrap()
            .instantiate("")
            .unwrap();
        let document = TextDocument::new();
        let session = snippet.insert(document.clone(), 0, "").unwrap();
        assert_eq!(document.text(), "extends Base / extends Base / Base / Body");
        let child = session.next_element(false).unwrap();
        assert_eq!(session.active_placeholder(), Some(2));
        document
            .replace(child.offset, child.length, "Other😀")
            .unwrap();
        let updated = "extends Other😀 / extends Other😀 / Other😀 / Body";
        assert_eq!(document.text(), updated);

        let outer = session.next_element(true).unwrap();
        document
            .replace(outer.offset, outer.length, "none")
            .unwrap();
        assert_eq!(document.text(), "none / none / Other😀 / Body");
        assert!(session.replaceable_element(2).is_none());
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), updated);
        assert_eq!(session.next_element(false).unwrap().offset, child.offset);
        assert_eq!(session.active_placeholder(), Some(2));
        assert_eq!(
            session.replaceable_element(2).unwrap().text().unwrap(),
            "Other😀"
        );
        assert!(document.redo().unwrap());
        assert_eq!(document.text(), "none / none / Other😀 / Body");
        assert!(session.replaceable_element(2).is_none());
        assert!(session.next_element(false).is_some());
        assert_eq!(session.active_placeholder(), Some(3));
    }
}
