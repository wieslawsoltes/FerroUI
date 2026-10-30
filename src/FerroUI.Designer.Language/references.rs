use crate::{Result, require};
use ferroui_designer_core::{Document, NodeId, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameOccurrence {
    pub name: String,
    pub span: SourceSpan,
    pub node: NodeId,
    pub scope: NodeId,
    pub declaration: bool,
}
pub fn name_occurrences(document: &Document) -> Vec<NameOccurrence> {
    if !document.is_valid() {
        return Vec::new();
    }
    let mut occurrences = Vec::new();
    static SCHEMA: std::sync::OnceLock<crate::Schema> = std::sync::OnceLock::new();
    let schema = SCHEMA.get_or_init(crate::Schema::ferroui);
    for (node, scope) in document
        .ast()
        .nodes_with_name_scopes()
        .filter(|(node, _)| node.is_element())
    {
        let namespaces = document.ast().namespaces(node.id);
        let native_type = |name: &str, namespace: Option<&str>| {
            namespace.is_none_or(|namespace| namespace == crate::schema::FRAMEWORK_NAMESPACE)
                || schema
                    .type_info(name, namespace)
                    .is_some_and(|info| info.namespace == crate::schema::FRAMEWORK_NAMESPACE)
        };
        let native_owner = native_type(node.local_name(), node.namespace_uri.as_deref());
        for attribute in &node.attributes {
            let raw = &document.source()[attribute.value_span.range()];
            let offset = attribute.value_span.start;
            let local = attribute.name.rsplit([':', '.']).next().unwrap();
            let declaration = (attribute.name == "Name"
                || local == "Name"
                    && attribute.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml"))
                && !raw.is_empty();
            let relative_reference =
                attribute
                    .name
                    .rsplit_once('.')
                    .is_some_and(|(owner, property)| {
                        let (prefix, name) = owner.split_once(':').unwrap_or(("", owner));
                        let namespace = namespaces.get(prefix).map(String::as_str);
                        (prefix.is_empty() || namespace.is_some())
                            && schema.type_info(name, namespace).is_some_and(|info| {
                                info.namespace == crate::schema::FRAMEWORK_NAMESPACE
                                    && info.name == "RelativePanel"
                                    && info.properties.contains_key(property)
                                    && !property.ends_with("Panel")
                            })
                    });
            let bare_reference = relative_reference
                || native_owner
                    && native_type(node.local_name(), attribute.namespace_uri.as_deref())
                    && matches!(
                        local,
                        "ElementName"
                            | "TargetName"
                            | "SourceName"
                            | "TargetObject"
                            | "SourceObject"
                    );
            if declaration || bare_reference && identifier(&attribute.value) {
                occurrences.push(NameOccurrence {
                    name: attribute.value.clone(),
                    span: attribute.value_span,
                    node: node.id,
                    scope,
                    declaration,
                });
            }
            if raw.starts_with("{}") {
                continue;
            }
            for argument in markup_arguments(raw) {
                let (prefix, extension_name) = argument
                    .extension
                    .split_once(':')
                    .unwrap_or(("", argument.extension.as_str()));
                let namespace = namespaces.get(prefix).map(String::as_str);
                let native = if prefix.is_empty() || namespace.is_some() {
                    schema
                        .type_info(extension_name, namespace)
                        .filter(|info| info.namespace == crate::schema::FRAMEWORK_NAMESPACE)
                } else {
                    None
                };
                // Binding is the runtime loader's alias for ReflectionBindingExtension,
                // so it is present in extension completion but not the generated type table.
                let framework_alias = (prefix.is_empty() && namespace.is_none()
                    || namespace == Some(crate::schema::FRAMEWORK_NAMESPACE))
                    && matches!(extension_name, "Binding" | "BindingExtension");
                let binding = framework_alias
                    || native.is_some_and(|info| {
                        matches!(
                            info.name.strip_suffix("Extension").unwrap_or(&info.name),
                            "Binding" | "ReflectionBinding" | "CompiledBinding"
                        )
                    });
                let reference_parameter = (binding || native.is_some())
                    && argument.name.as_deref().is_some_and(|name| {
                        matches!(
                            name.rsplit('.').next().unwrap_or(name),
                            "ElementName"
                                | "TargetName"
                                | "SourceName"
                                | "TargetObject"
                                | "SourceObject"
                        )
                    });
                let reference_extension =
                    matches!(extension_name, "Reference" | "ReferenceExtension")
                        && namespace == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                        && argument.name.as_deref().is_none_or(|name| name == "Name");
                if (reference_parameter || reference_extension) && identifier(&argument.value) {
                    occurrences.push(NameOccurrence {
                        name: argument.value.clone(),
                        span: SourceSpan::new(
                            offset + argument.span.start,
                            offset + argument.span.end,
                        ),
                        node: node.id,
                        scope,
                        declaration: false,
                    });
                }
                if binding && argument.name.as_deref().is_none_or(|name| name == "Path") {
                    push_hash_references(
                        &mut occurrences,
                        raw,
                        argument.span,
                        offset,
                        node.id,
                        scope,
                    );
                }
            }
            if native_owner && attribute.name == "Selector" {
                push_hash_references(
                    &mut occurrences,
                    raw,
                    SourceSpan::new(0, raw.len()),
                    offset,
                    node.id,
                    scope,
                );
            }
        }
    }
    occurrences.sort_by_key(|o| (o.span.start, o.span.end));
    occurrences.dedup_by_key(|o| (o.span.start, o.span.end));
    occurrences
}
/// The containing XAML name scope. A template's own declaration belongs to its
/// parent scope, while nodes inside the template belong to the template scope.
pub fn scope_of(document: &Document, node: NodeId) -> NodeId {
    document.ast().name_scope(node)
}
fn push_hash_references(
    out: &mut Vec<NameOccurrence>,
    raw: &str,
    span: SourceSpan,
    offset: usize,
    node: NodeId,
    scope: NodeId,
) {
    let decoded = Decoded::new(&raw[span.range()]);
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in decoded.text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if quote == Some(ch) {
            quote = None;
            continue;
        }
        if quote.is_some() {
            continue;
        }
        if matches!(ch, '\'' | '"') {
            quote = Some(ch);
            continue;
        }
        if ch == '#' {
            let end = identifier_end(&decoded.text, index + 1);
            if end > index + 1 {
                out.push(NameOccurrence {
                    name: decoded.text[index + 1..end].into(),
                    span: SourceSpan::new(
                        offset + span.start + decoded.boundaries[index + 1],
                        offset + span.start + decoded.boundaries[end],
                    ),
                    node,
                    scope,
                    declaration: false,
                });
            }
        }
    }
}
/// Whether a string is a valid identifier for the designer's XAML name/reference grammar.
pub fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
}
fn identifier_end(source: &str, start: usize) -> usize {
    source[start..]
        .char_indices()
        .take_while(|(_, c)| c.is_alphanumeric() || matches!(c, '_' | '-'))
        .last()
        .map_or(start, |(i, c)| start + i + c.len_utf8())
}
fn skip_space(source: &str, start: usize) -> usize {
    source[start..]
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(source.len(), |(i, _)| start + i)
}
/// A value token in the nested markup grammar, with its original XML byte range.
/// Entity decoding must happen before markup lexing: &quot; may delimit a quoted key.
#[derive(Clone, Debug)]
pub(crate) struct Argument {
    pub extension: String,
    pub name: Option<String>,
    pub value: String,
    pub span: SourceSpan,
    pub quote: Option<char>,
}
pub(crate) fn markup_arguments(source: &str) -> Vec<Argument> {
    let decoded = Decoded::new(source);
    if !decoded.text.starts_with('{') || decoded.text.starts_with("{}") {
        return Vec::new();
    }
    let mut arguments = Vec::new();
    parse_extension(&decoded, 0, 0, &mut arguments);
    arguments.sort_by_key(|argument| argument.span.start);
    arguments
}
fn parse_extension(source: &Decoded, open: usize, depth: usize, out: &mut Vec<Argument>) -> usize {
    if depth >= 128 {
        return source.text.len();
    }
    let text = &source.text;
    let start = skip_space(text, open + 1);
    let end = text[start..]
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | '}' | '{'))
        .map_or(text.len(), |i| start + i);
    let extension = &text[start..end];
    let mut at = skip_space(text, end);
    let mut segment = at;
    let mut equal = None;
    let mut quote = None;
    let mut escaped = false;
    while at < text.len() {
        let ch = text[at..].chars().next().unwrap();
        let next = at + ch.len_utf8();
        if escaped {
            escaped = false;
            at = next;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            at = next;
            continue;
        }
        if quote == Some(ch) {
            quote = None;
            at = next;
            continue;
        }
        if quote.is_some() {
            at = next;
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '{' => {
                at = parse_extension(source, at, depth + 1, out);
                continue;
            }
            '=' => {
                if equal.is_none() {
                    equal = Some(at);
                }
            }
            ',' | '}' => {
                push_argument(source, extension, segment, at, equal, out);
                if ch == '}' {
                    return next;
                }
                segment = skip_space(text, next);
                equal = None;
            }
            _ => {}
        }
        at = next;
    }
    push_argument(source, extension, segment, at, equal, out);
    at
}
fn push_argument(
    source: &Decoded,
    extension: &str,
    start: usize,
    end: usize,
    equal: Option<usize>,
    out: &mut Vec<Argument>,
) {
    let text = &source.text;
    let name = equal.map(|equal| text[start..equal].trim().to_owned());
    let mut start = skip_space(text, equal.map_or(start, |equal| equal + 1));
    let mut end = end;
    while end > start
        && text[..end]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        end -= text[..end].chars().next_back().unwrap().len_utf8();
    }
    if start >= end || text.as_bytes()[start] == b'{' {
        return;
    }
    let quote = text[start..end]
        .chars()
        .next()
        .filter(|c| matches!(c, '\'' | '"'));
    if let Some(quote) = quote {
        if !text[start + 1..end].ends_with(quote) {
            return;
        }
        start += 1;
        end -= 1;
    }
    if start >= end {
        return;
    }
    let mut value = String::new();
    let mut escaped = false;
    for ch in text[start..end].chars() {
        if escaped {
            value.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            value.push(ch);
        }
    }
    if escaped {
        value.push('\\');
    }
    out.push(Argument {
        extension: extension.into(),
        name,
        value,
        span: SourceSpan::new(source.boundaries[start], source.boundaries[end]),
        quote,
    });
}
struct Decoded {
    text: String,
    boundaries: Vec<usize>,
}
impl Decoded {
    fn new(source: &str) -> Self {
        let mut text = String::with_capacity(source.len());
        let mut boundaries = vec![0];
        let mut at = 0;
        while at < source.len() {
            let mut ch = source[at..].chars().next().unwrap();
            let mut end = at + ch.len_utf8();
            if ch == '&' {
                if let Some(close) = source[at..].find(';') {
                    let entity = &source[at + 1..at + close];
                    let decoded = match entity {
                        "amp" => Some('&'),
                        "lt" => Some('<'),
                        "gt" => Some('>'),
                        "quot" => Some('"'),
                        "apos" => Some('\''),
                        _ => entity
                            .strip_prefix("#x")
                            .and_then(|n| u32::from_str_radix(n, 16).ok())
                            .or_else(|| entity.strip_prefix('#').and_then(|n| n.parse().ok()))
                            .and_then(char::from_u32),
                    };
                    if let Some(decoded) = decoded {
                        ch = decoded;
                        end = at + close + 1;
                    }
                }
            } else if matches!(ch, '\r' | '\n' | '\t') {
                if ch == '\r' && source.as_bytes().get(end) == Some(&b'\n') {
                    end += 1;
                }
                ch = ' ';
            }
            text.push(ch);
            boundaries.extend(std::iter::repeat_n(at, ch.len_utf8() - 1));
            boundaries.push(end);
            at = end;
        }
        Self { text, boundaries }
    }
}
/// Rewrite declarations and only references that resolve to a declaration within this source.
/// The full-document occurrence API is preferable for copied fragments with inherited prefixes.
pub fn rewrite_names(source: &str, renames: &BTreeMap<String, String>) -> Result<String> {
    let document = Document::new(source)?;
    for value in renames.values() {
        require(
            identifier(value),
            "Replacement name must be a XAML identifier",
        )?;
    }
    let occurrences = name_occurrences(&document);
    let declarations: BTreeMap<_, _> = occurrences
        .iter()
        .filter(|o| o.declaration)
        .map(|o| ((o.scope, o.name.clone()), o))
        .collect();
    let mut replacements = Vec::new();
    for occurrence in &occurrences {
        if declarations.contains_key(&(occurrence.scope, occurrence.name.clone())) {
            if let Some(value) = renames.get(&occurrence.name) {
                replacements.push((occurrence.span, value));
            }
        }
    }
    let mut text = source.to_owned();
    for (span, value) in replacements.into_iter().rev() {
        text.replace_range(span.range(), value);
    }
    Document::new(&text)?;
    Ok(text)
}

pub(crate) struct ActiveExtension {
    pub start: usize,
    pub name: String,
    pub argument: String,
}
/// Find the innermost extension still open at the cursor, ignoring literal/quoted braces.
pub(crate) fn active_extension(source: &str) -> Option<ActiveExtension> {
    let decoded = Decoded::new(source);
    if decoded.text.starts_with("{}") {
        return None;
    }
    let mut stack = Vec::<(usize, usize)>::new();
    let mut quote = None;
    let mut escaped = false;
    for (index, ch) in decoded.text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if quote == Some(ch) {
            quote = None;
            continue;
        }
        if quote.is_some() {
            continue;
        }
        match ch {
            '\'' | '"' if !stack.is_empty() => quote = Some(ch),
            '{' => stack.push((index, index + 1)),
            '}' => {
                stack.pop();
            }
            ',' => {
                if let Some((_, argument)) = stack.last_mut() {
                    *argument = index + 1;
                }
            }
            _ => {}
        }
    }
    let (open, argument) = stack.pop()?;
    let start = skip_space(&decoded.text, open + 1);
    let end = decoded.text[start..]
        .find(|ch: char| ch.is_whitespace() || ch == ',')
        .map_or(decoded.text.len(), |index| start + index);
    Some(ActiveExtension {
        start: decoded.boundaries[open],
        name: decoded.text[start..end].into(),
        argument: decoded.text[argument.max(end)..].trim().into(),
    })
}
