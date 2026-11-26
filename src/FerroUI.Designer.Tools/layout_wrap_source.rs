//! Source splices keep the existing controls, scopes and value tokens in place.
use super::SelectionWrap;
use crate::layout_attributes::framework_namespace;
use crate::{error, require, Result, ToolError};
use ferroui_designer_core::{Ast, Document, Node, NodeKind, SourceSpan, TextPatch};
use std::collections::BTreeSet;

impl SelectionWrap {
    /// An empty canonical wrapper used by the declarative Rust mapper.
    pub fn wrapper_fragment(&self) -> String {
        format!("{}/>", self.opening(self.kind.control(), "", None))
    }

    /// Plan disjoint insertions and attribute edits so the document can retain the
    /// wrapped children's identities and commit the operation as one transaction.
    pub fn xaml_patches(&self, document: &Document) -> Result<Vec<TextPatch>> {
        let ast = document.ast();
        let first = self
            .children
            .first()
            .and_then(|&id| ast.node(id))
            .ok_or_else(|| ToolError("The first wrapped control changed".into()))?;
        let last = self
            .children
            .last()
            .and_then(|&id| ast.node(id))
            .ok_or_else(|| ToolError("The last wrapped control changed".into()))?;
        let owner = ast
            .node(self.container)
            .ok_or_else(|| ToolError("The authored layout slot changed".into()))?;
        require(
            first.parent == Some(owner.id)
                && last.parent == Some(owner.id)
                && first.span.start < last.span.end,
            "The wrapped source range changed",
        )?;
        let namespaces = ast.namespaces(owner.id);
        let (prefix, declaration) = if framework_namespace(namespaces.get("").map(String::as_str)) {
            (String::new(), None)
        } else if let Some((prefix, _)) = namespaces
            .iter()
            .find(|(prefix, uri)| !prefix.is_empty() && framework_namespace(Some(uri)))
        {
            (format!("{prefix}:"), None)
        } else {
            let prefix = std::iter::once("f".to_owned())
                .chain((2..).map(|index| format!("f{index}")))
                .find(|prefix| !namespaces.contains_key(prefix))
                .unwrap();
            let declaration = format!("xmlns:{prefix}=\"https://github.com/ferroui\"");
            (format!("{prefix}:"), Some(declaration))
        };
        let tag = format!("{prefix}{}", self.kind.control());
        let mut open = format!("{}>", self.opening(&tag, &prefix, declaration.as_deref()));
        let mut close = format!("</{tag}>");
        let mut patches = document
            .plan_attribute_patches(self.edits.clone())
            .map_err(error)?;
        let text = document.source();
        for patch in &mut patches {
            let before = &text[..patch.span.start];
            if patch.replacement.is_empty() {
                patch.span.start = before.trim_end_matches([' ', '\t']).len();
            } else if patch.span.is_empty() {
                let offset = before.trim_end_matches([' ', '\t']).len();
                patch.span = SourceSpan::new(offset, offset);
            }
        }
        if let Some(indent) =
            line_indent(text, first.span.start).filter(|_| format_children(ast, owner))
        {
            let unit = line_indent(text, owner.span.start)
                .and_then(|outer| indent.strip_prefix(outer))
                .filter(|unit| !unit.is_empty())
                .unwrap_or(if indent.ends_with('\t') { "\t" } else { "    " });
            let eol = newline_at(text, first.span.start);
            open.push_str(&format!("{eol}{indent}{unit}"));
            close.insert_str(0, &format!("{eol}{indent}"));
            let mut offsets = structural_lines(ast, owner, first, last);
            offsets.remove(&first.span.start);
            let indents: Vec<_> = offsets
                .into_iter()
                .filter(|&offset| {
                    line_indent(text, offset).is_some()
                        && !patches
                            .iter()
                            .any(|patch| patch.span.start <= offset && offset < patch.span.end)
                })
                .map(|offset| insert(offset, unit.to_owned()))
                .collect();
            patches.extend(indents);
        }
        patches.push(insert(first.span.start, open));
        patches.push(insert(last.span.end, close));
        Ok(patches)
    }

    fn opening(&self, name: &str, prefix: &str, declaration: Option<&str>) -> String {
        let mut tag = format!("<{name}");
        if let Some(declaration) = declaration {
            tag.push(' ');
            tag.push_str(declaration);
        }
        for (name, value) in &self.wrapper_properties {
            let attribute_prefix = if name.contains('.') { prefix } else { "" };
            let value = value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('"', "&quot;");
            tag.push_str(&format!(" {attribute_prefix}{name}=\"{value}\""));
        }
        tag
    }
}

fn structural_lines(ast: &Ast, owner: &Node, first: &Node, last: &Node) -> BTreeSet<usize> {
    let mut offsets = BTreeSet::new();
    for root in owner
        .children
        .iter()
        .filter_map(|id| ast.node(*id))
        .filter(|node| first.span.start <= node.span.start && node.span.end <= last.span.end)
    {
        for node in ast
            .subtree(root.id)
            .filter(|node| !matches!(node.kind, NodeKind::Text | NodeKind::CData))
        {
            if node
                .parent
                .and_then(|id| ast.node(id))
                .is_some_and(|parent| format_children(ast, parent))
            {
                offsets.insert(node.span.start);
            }
            offsets.extend(
                node.attributes
                    .iter()
                    .map(|attribute| attribute.name_span.start),
            );
            if let Some(span) = node.closing_span.filter(|_| format_children(ast, node)) {
                offsets.insert(span.start);
            }
        }
    }
    offsets
}

fn format_children(ast: &Ast, node: &Node) -> bool {
    !node
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
        && std::iter::successors(Some(node), |node| node.parent.and_then(|id| ast.node(id)))
            .find_map(|node| node.attribute("xml:space"))
            != Some("preserve")
}

fn insert(offset: usize, replacement: String) -> TextPatch {
    TextPatch {
        span: SourceSpan::new(offset, offset),
        replacement,
    }
}

fn line_indent(source: &str, offset: usize) -> Option<&str> {
    let line = source[..offset].rsplit('\n').next()?;
    line.bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
        .then_some(line)
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
