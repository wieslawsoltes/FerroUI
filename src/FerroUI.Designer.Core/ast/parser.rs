use super::{Ast, Attribute, Node, NodeId, NodeKind, SourceSpan};
use crate::DesignerError;
use std::collections::{BTreeMap, HashMap};

pub(super) const MAX_SYNTAX_NODES: usize = 1_000_000;

impl Ast {
    pub(crate) fn parse(source: &str) -> Result<Self, DesignerError> {
        Self::parse_with_limit(source, MAX_SYNTAX_NODES)
    }

    pub(super) fn parse_with_limit(
        source: &str,
        syntax_limit: usize,
    ) -> Result<Self, DesignerError> {
        validate_xml(source)?;
        let mut ast = Self {
            root: 0,
            roots: Vec::new(),
            nodes: BTreeMap::new(),
        };
        let mut stack = Vec::<NodeId>::new();
        let mut namespace_scope = HashMap::from([(
            "xml".to_owned(),
            "http://www.w3.org/XML/1998/namespace".to_owned(),
        )]);
        let mut namespace_stack = Vec::<Vec<(String, Option<String>)>>::new();
        let mut offset = 0;
        let mut id = 1;
        while offset < source.len() {
            let remaining = &source[offset..];
            if remaining.starts_with("</") {
                let end = offset + remaining.find('>').expect("validated XML closing tag") + 1;
                let node_id = stack.pop().expect("validated XML closing element");
                let node = ast.nodes.get_mut(&node_id).unwrap();
                node.closing_span = Some(SourceSpan::new(offset, end));
                node.span.end = end;
                for (prefix, previous) in namespace_stack.pop().unwrap().into_iter().rev() {
                    if let Some(uri) = previous {
                        namespace_scope.insert(prefix, uri);
                    } else {
                        namespace_scope.remove(&prefix);
                    }
                }
                offset = end;
                continue;
            }
            // The validating XML DOM merges adjacent text/CDATA. Our lossless
            // tree keeps them distinct, so its allocation budget must also be
            // enforced here before allocating another syntax token.
            if ast.nodes.len() >= syntax_limit {
                return Err(DesignerError::Parse {
                    message: format!(
                        "The document exceeds the limit of {syntax_limit} syntax nodes"
                    ),
                    line: 1,
                    column: 1,
                });
            }
            let parent = stack.last().copied();
            let (kind, end, content) = if remaining.starts_with("<!--") {
                let end = offset + remaining.find("-->").unwrap() + 3;
                (
                    NodeKind::Comment,
                    end,
                    Some(normalize_line_endings(&source[offset + 4..end - 3])),
                )
            } else if remaining.starts_with("<![CDATA[") {
                let end = offset + remaining.find("]]>").unwrap() + 3;
                (
                    NodeKind::CData,
                    end,
                    Some(normalize_line_endings(&source[offset + 9..end - 3])),
                )
            } else if remaining.starts_with("<?") {
                let end = offset + remaining.find("?>").unwrap() + 2;
                (
                    NodeKind::ProcessingInstruction,
                    end,
                    Some(normalize_line_endings(&source[offset + 2..end - 2])),
                )
            } else if remaining.starts_with('<') {
                (NodeKind::Element, tag_end(source, offset), None)
            } else {
                let end = offset + remaining.find('<').unwrap_or(remaining.len());
                (
                    NodeKind::Text,
                    end,
                    Some(decode_xml(&source[offset..end], false)),
                )
            };
            let mut node = Node {
                id,
                kind,
                name: String::new(),
                namespace_uri: None,
                parent,
                children: Vec::new(),
                attributes: Vec::new(),
                span: SourceSpan::new(offset, end),
                name_span: SourceSpan::default(),
                opening_span: SourceSpan::new(offset, end),
                closing_span: None,
                self_closing: false,
                text: content,
            };
            if kind == NodeKind::Element {
                let mut cursor = offset + 1;
                let name_start = cursor;
                while cursor < end
                    && !is_space(source.as_bytes()[cursor])
                    && !matches!(source.as_bytes()[cursor], b'/' | b'>')
                {
                    cursor += 1;
                }
                node.name = source[name_start..cursor].to_owned();
                node.name_span = SourceSpan::new(name_start, cursor);
                if node.local_name().contains('.') {
                    node.kind = NodeKind::PropertyElement;
                }
                node.self_closing = source.as_bytes()[end - 2] == b'/';
                while cursor < end {
                    while cursor < end && is_space(source.as_bytes()[cursor]) {
                        cursor += 1;
                    }
                    if cursor >= end || matches!(source.as_bytes()[cursor], b'/' | b'>') {
                        break;
                    }
                    let name_start = cursor;
                    while !is_space(source.as_bytes()[cursor]) && source.as_bytes()[cursor] != b'='
                    {
                        cursor += 1;
                    }
                    let name_end = cursor;
                    while is_space(source.as_bytes()[cursor]) {
                        cursor += 1;
                    }
                    cursor += 1; // equals
                    while is_space(source.as_bytes()[cursor]) {
                        cursor += 1;
                    }
                    let quote = source.as_bytes()[cursor];
                    cursor += 1;
                    let value_start = cursor;
                    while source.as_bytes()[cursor] != quote {
                        cursor += 1;
                    }
                    let value_end = cursor;
                    cursor += 1;
                    node.attributes.push(Attribute {
                        name: source[name_start..name_end].to_owned(),
                        value: decode_xml(&source[value_start..value_end], true),
                        namespace_uri: None,
                        span: SourceSpan::new(name_start, cursor),
                        name_span: SourceSpan::new(name_start, name_end),
                        value_span: SourceSpan::new(value_start, value_end),
                        quote: quote as char,
                    });
                }
                let mut changes = Vec::new();
                for attr in &node.attributes {
                    let prefix = if attr.name == "xmlns" {
                        Some("")
                    } else {
                        attr.name.strip_prefix("xmlns:")
                    };
                    if let Some(prefix) = prefix {
                        let previous = namespace_scope.insert(prefix.into(), attr.value.clone());
                        changes.push((prefix.to_owned(), previous));
                    }
                }
                let prefix = node.name.split_once(':').map_or("", |(prefix, _)| prefix);
                node.namespace_uri = namespace_scope
                    .get(prefix)
                    .filter(|uri| !uri.is_empty())
                    .cloned();
                for attr in &mut node.attributes {
                    attr.namespace_uri = if attr.is_namespace() {
                        Some("http://www.w3.org/2000/xmlns/".into())
                    } else {
                        attr.name
                            .split_once(':')
                            .and_then(|(prefix, _)| namespace_scope.get(prefix))
                            .cloned()
                    };
                }
                if node.self_closing {
                    for (prefix, previous) in changes.into_iter().rev() {
                        if let Some(uri) = previous {
                            namespace_scope.insert(prefix, uri);
                        } else {
                            namespace_scope.remove(&prefix);
                        }
                    }
                } else {
                    namespace_stack.push(changes);
                }
                if ast.root == 0 {
                    ast.root = id;
                }
            }
            if let Some(parent) = parent {
                ast.nodes.get_mut(&parent).unwrap().children.push(id);
            } else {
                ast.roots.push(id);
            }
            let is_open = node.is_element() && !node.self_closing;
            ast.nodes.insert(id, node);
            if is_open {
                stack.push(id);
            }
            id += 1;
            offset = end;
        }
        Ok(ast)
    }
}

/// Shared validation policy for complete documents and wrapped child fragments.
/// No syntax indexing is needed when validating only a transaction boundary.
pub(crate) fn validate_xml(source: &str) -> Result<(), DesignerError> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_SYNTAX_NODES as u32,
    };
    roxmltree::Document::parse_with_options(source, options)
        .map(|_| ())
        .map_err(|error| {
            let pos = error.pos();
            DesignerError::Parse {
                message: error.to_string(),
                line: pos.row,
                column: pos.col,
            }
        })
}

fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\r' | b'\n')
}
fn tag_end(source: &str, start: usize) -> usize {
    let mut quote = None;
    for (offset, byte) in source.as_bytes()[start + 1..].iter().enumerate() {
        if let Some(q) = quote {
            if *byte == q {
                quote = None;
            }
        } else if matches!(*byte, b'\'' | b'"') {
            quote = Some(*byte);
        } else if *byte == b'>' {
            return start + 1 + offset + 1;
        }
    }
    unreachable!("validated XML opening tag")
}

pub(crate) fn decode_xml(source: &str, attribute: bool) -> String {
    if !source.contains(['&', '\r']) && (!attribute || !source.contains(['\n', '\t'])) {
        return source.to_owned();
    }
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0;
    while cursor < source.len() {
        let remaining = &source[cursor..];
        if remaining.starts_with('&') {
            let end = remaining.find(';').expect("validated XML reference");
            let name = &remaining[1..end];
            let ch = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                _ if name.starts_with("#x") => {
                    char::from_u32(u32::from_str_radix(&name[2..], 16).unwrap()).unwrap()
                }
                _ if name.starts_with('#') => char::from_u32(name[1..].parse().unwrap()).unwrap(),
                _ => unreachable!("DTD is disabled"),
            };
            out.push(ch);
            cursor += end + 1;
        } else {
            let ch = remaining.chars().next().unwrap();
            if ch == '\r' {
                cursor += 1;
                if source.as_bytes().get(cursor) == Some(&b'\n') {
                    cursor += 1;
                }
                out.push(if attribute { ' ' } else { '\n' });
            } else {
                cursor += ch.len_utf8();
                out.push(if attribute && matches!(ch, '\n' | '\t') {
                    ' '
                } else {
                    ch
                });
            }
        }
    }
    out
}

/// XML end-of-line normalization applies to every lexical text token, including
/// CDATA, comments and processing instructions. Entity references are decoded
/// separately because a character reference to CR must remain a CR.
fn normalize_line_endings(source: &str) -> String {
    if !source.contains('\r') {
        return source.to_owned();
    }
    let mut output = String::with_capacity(source.len());
    let mut remaining = source;
    while let Some(offset) = remaining.find('\r') {
        output.push_str(&remaining[..offset]);
        output.push('\n');
        remaining = &remaining[offset + 1..];
        if let Some(after_lf) = remaining.strip_prefix('\n') {
            remaining = after_lf;
        }
    }
    output.push_str(remaining);
    output
}
