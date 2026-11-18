//! Match a multiline XAML container's indentation without reformatting its contents.
use crate::{Ast, Node, NodeId, NodeKind};

pub(super) fn inline_or_indented(
    source: &str,
    ast: &Ast,
    parent: &Node,
    children: &[NodeId],
    index: usize,
    offset: usize,
    fragment: &str,
) -> (String, usize) {
    let Some(indent) = line_indent(source, offset).filter(|_| can_format(ast, parent, fragment))
    else {
        return (fragment.to_owned(), 0);
    };
    let newline = newline_at(source, offset);
    if index < children.len() {
        return (format!("{fragment}{newline}{indent}"), 0);
    }
    let extra = children
        .iter()
        .rev()
        .filter_map(|id| ast.node(*id))
        .filter_map(|child| line_indent(source, child.span.start))
        .filter_map(|child| child.strip_prefix(indent))
        .find(|extra| !extra.is_empty())
        .unwrap_or_else(|| indent_unit(source, ast, parent, indent));
    (format!("{extra}{fragment}{newline}{indent}"), extra.len())
}

pub(super) fn expanded(source: &str, ast: &Ast, parent: &Node, fragment: &str) -> (String, usize) {
    if source.contains('\n') && can_format(ast, parent, fragment) {
        if let Some(indent) = line_indent(source, parent.span.start) {
            let newline = newline_at(source, parent.span.start);
            let extra = indent_unit(source, ast, parent, indent);
            let head = format!(">{newline}{indent}{extra}");
            let offset = head.len();
            return (
                format!("{head}{fragment}{newline}{indent}</{}>", parent.name),
                offset,
            );
        }
    }
    (format!(">{fragment}</{}>", parent.name), 1)
}

fn can_format(ast: &Ast, parent: &Node, fragment: &str) -> bool {
    if !fragment.starts_with('<')
        || !fragment.ends_with('>')
        || parent
            .children
            .iter()
            .filter_map(|id| ast.node(*id))
            .any(|child| {
                matches!(child.kind, NodeKind::Text | NodeKind::CData)
                    && child
                        .text
                        .as_ref()
                        .is_some_and(|text| !text.trim().is_empty())
            })
    {
        return false;
    }
    std::iter::successors(Some(parent), |node| node.parent.and_then(|id| ast.node(id)))
        .find_map(|node| node.attribute("xml:space"))
        != Some("preserve")
}

fn line_indent(source: &str, offset: usize) -> Option<&str> {
    let line = source[..offset].rsplit('\n').next()?;
    line.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
        .then_some(line)
}

fn indent_unit<'a>(source: &'a str, ast: &Ast, parent: &Node, indent: &'a str) -> &'a str {
    parent
        .parent
        .and_then(|id| ast.node(id))
        .and_then(|owner| line_indent(source, owner.span.start))
        .and_then(|prefix| indent.strip_prefix(prefix))
        .filter(|extra| !extra.is_empty())
        .unwrap_or(if indent.ends_with('\t') { "\t" } else { "    " })
}

fn newline_at(source: &str, offset: usize) -> &'static str {
    if source[..offset].rsplit_once('\n').map_or_else(
        || source.contains("\r\n"),
        |(prefix, _)| prefix.ends_with('\r'),
    ) {
        "\r\n"
    } else {
        "\n"
    }
}
