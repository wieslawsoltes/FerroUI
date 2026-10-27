//! Validate serialized syntax indexes before exposing structural queries.
//!
//! Source text is not part of this wire format, so decoding cannot establish XML
//! spelling or UTF-8 boundaries. It can and must establish tree ownership,
//! acyclicity and ordered, bounded source intervals used by AST traversal.

use super::{Ast, Attribute, Node, NodeId, SourceSpan};
use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, HashSet};

impl<'de> Deserialize<'de> for Ast {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct WireAst {
            root: NodeId,
            roots: Vec<NodeId>,
            nodes: BTreeMap<NodeId, Node>,
        }
        let wire = WireAst::deserialize(deserializer)?;
        let ast = Ast {
            root: wire.root,
            roots: wire.roots,
            nodes: wire.nodes,
        };
        ast.validate_index().map_err(serde::de::Error::custom)?;
        Ok(ast)
    }
}

impl Ast {
    fn validate_index(&self) -> Result<(), String> {
        if self.nodes.len() > super::parser::MAX_SYNTAX_NODES {
            return Err("Serialized AST exceeds the syntax-node limit".into());
        }
        let root = self.node(self.root).ok_or("AST root is missing")?;
        if !root.is_element() || root.parent.is_some() {
            return Err("AST root must be a top-level element".into());
        }
        let end = self
            .roots
            .last()
            .and_then(|id| self.node(*id))
            .ok_or("AST has no complete root list")?
            .span
            .end;
        let mut owned = HashSet::with_capacity(self.nodes.len());
        self.validate_children(&self.roots, None, SourceSpan::new(0, end), &mut owned)?;
        let element_roots = self
            .roots
            .iter()
            .filter(|id| self.nodes[id].is_element())
            .count();
        if element_roots != 1 {
            return Err("AST must contain exactly one top-level element".into());
        }
        for (&id, node) in &self.nodes {
            if id == 0 || id != node.id {
                return Err(format!("AST node key {id} disagrees with its identity"));
            }
            let content = validate_node(node)?;
            self.validate_children(&node.children, Some(id), content, &mut owned)?;
        }
        if owned.len() != self.nodes.len() {
            return Err("AST contains unreachable nodes".into());
        }
        Ok(())
    }

    /// A node belongs to exactly one ordered sibling list. The parent backlink
    /// must agree, and content intervals are a contiguous partition of the
    /// enclosing source range. Strictly smaller element contents also rule out
    /// disconnected cycles without recursive validation.
    fn validate_children(
        &self,
        children: &[NodeId],
        parent: Option<NodeId>,
        content: SourceSpan,
        owned: &mut HashSet<NodeId>,
    ) -> Result<(), String> {
        let mut offset = content.start;
        for &id in children {
            let child = self
                .node(id)
                .ok_or_else(|| format!("AST child {id} is missing"))?;
            if !owned.insert(id) {
                return Err(format!("AST node {id} has multiple owners or a cycle"));
            }
            if child.parent != parent {
                return Err(format!("AST node {id} has an inconsistent parent"));
            }
            if child.span.start != offset
                || child.span.end <= offset
                || child.span.end > content.end
            {
                return Err(format!(
                    "AST node {id} has unordered or out-of-bounds source spans"
                ));
            }
            offset = child.span.end;
        }
        if offset != content.end {
            return Err("AST children do not cover their source content interval".into());
        }
        Ok(())
    }
}

fn encloses(outer: SourceSpan, inner: SourceSpan) -> bool {
    outer.start <= inner.start && inner.start <= inner.end && inner.end <= outer.end
}

/// Return the exact child-content interval after validating token metadata.
fn validate_node(node: &Node) -> Result<SourceSpan, String> {
    if node.span.start >= node.span.end
        || node.opening_span.start != node.span.start
        || node.opening_span.is_empty()
        || !encloses(node.span, node.opening_span)
    {
        return Err(format!("AST node {} has an invalid opening span", node.id));
    }
    if !node.is_element() {
        if !node.children.is_empty()
            || !node.attributes.is_empty()
            || !node.name.is_empty()
            || node.namespace_uri.is_some()
            || node.closing_span.is_some()
            || node.self_closing
            || node.opening_span != node.span
            || node.name_span != SourceSpan::default()
            || node.text.is_none()
        {
            return Err(format!("AST trivia node {} has element metadata", node.id));
        }
        return Ok(SourceSpan::new(node.span.end, node.span.end));
    }
    if node.name.is_empty()
        || node.text.is_some()
        || node.name_span.is_empty()
        || !encloses(node.opening_span, node.name_span)
        || node.name_span.start <= node.opening_span.start
        || node.name_span.end >= node.opening_span.end
        || node.name_span.len() != node.name.len()
    {
        return Err(format!("AST element {} has an invalid name span", node.id));
    }
    let mut previous = node.name_span.end;
    for attribute in &node.attributes {
        validate_attribute(attribute, node.opening_span, previous)?;
        previous = attribute.span.end;
    }
    if node.self_closing {
        if node.closing_span.is_some()
            || node.opening_span != node.span
            || !node.children.is_empty()
        {
            return Err(format!("Self-closing AST element {} has content", node.id));
        }
        return Ok(SourceSpan::new(node.span.end, node.span.end));
    }
    let closing = node
        .closing_span
        .ok_or_else(|| format!("AST element {} has no closing span", node.id))?;
    if closing.is_empty()
        || !encloses(node.span, closing)
        || closing.start < node.opening_span.end
        || closing.end != node.span.end
    {
        return Err(format!(
            "AST element {} has an invalid closing span",
            node.id
        ));
    }
    Ok(SourceSpan::new(node.opening_span.end, closing.start))
}

fn validate_attribute(
    attribute: &Attribute,
    opening: SourceSpan,
    previous_end: usize,
) -> Result<(), String> {
    if attribute.name.is_empty()
        || !matches!(attribute.quote, '\'' | '"')
        || attribute.span.is_empty()
        || !encloses(opening, attribute.span)
        || attribute.span.start <= previous_end
        || attribute.span.end >= opening.end
        || attribute.name_span.start != attribute.span.start
        || attribute.name_span.len() != attribute.name.len()
        || !encloses(attribute.span, attribute.name_span)
        || !encloses(attribute.span, attribute.value_span)
        || attribute.value_span.start <= attribute.name_span.end
        || attribute.value_span.end >= attribute.span.end
    {
        return Err(format!(
            "AST attribute {} has invalid source spans",
            attribute.name
        ));
    }
    Ok(())
}
