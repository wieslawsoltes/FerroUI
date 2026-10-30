use crate::MarkupValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod parser;
mod reconciliation;
mod validation;

pub(crate) use parser::validate_xml;

#[cfg(test)]
mod tests;

/// Stable document-local identity. IDs are never reused, including after undo.
pub type NodeId = u64;

/// Half-open UTF-8 byte range into the corresponding source snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}
impl SourceSpan {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
    pub fn contains(self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }
    pub fn range(self) -> std::ops::Range<usize> {
        self.start..self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    Element,
    PropertyElement,
    Text,
    Comment,
    CData,
    ProcessingInstruction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribute {
    /// Lexical qualified name, including namespace prefix or attached owner.
    pub name: String,
    /// XML-decoded value. `value_span` retains the original lexical spelling.
    pub value: String,
    pub namespace_uri: Option<String>,
    /// Range from the name to the closing quote; leading whitespace is excluded.
    pub span: SourceSpan,
    pub name_span: SourceSpan,
    pub value_span: SourceSpan,
    pub quote: char,
}
impl Attribute {
    pub fn markup(&self) -> Result<MarkupValue, String> {
        MarkupValue::parse(&self.value)
    }
    pub fn is_namespace(&self) -> bool {
        self.name == "xmlns" || self.name.starts_with("xmlns:")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
    /// Lexical qualified name for elements, empty for text and trivia.
    pub name: String,
    pub namespace_uri: Option<String>,
    pub parent: Option<NodeId>,
    /// All syntax children in source order, including whitespace and comments.
    pub children: Vec<NodeId>,
    pub attributes: Vec<Attribute>,
    pub span: SourceSpan,
    pub name_span: SourceSpan,
    pub opening_span: SourceSpan,
    pub closing_span: Option<SourceSpan>,
    pub self_closing: bool,
    /// Decoded text or unwrapped comment, CDATA or processing-instruction text.
    pub text: Option<String>,
}
impl Node {
    pub fn is_element(&self) -> bool {
        matches!(self.kind, NodeKind::Element | NodeKind::PropertyElement)
    }
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }
    pub fn attribute_info(&self, name: &str) -> Option<&Attribute> {
        self.attributes.iter().find(|a| a.name == name)
    }
    pub fn local_name(&self) -> &str {
        self.name.rsplit(':').next().unwrap_or(&self.name)
    }
    /// XAML name in its resolved namespace, with unprefixed Name as fallback.
    pub fn xaml_name(&self) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| {
                a.name.rsplit(':').next() == Some("Name")
                    && a.namespace_uri.as_deref()
                        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
            })
            .map(|a| a.value.as_str())
            .or_else(|| self.attribute("Name"))
    }
}

/// Lossless syntax index. Serialization is the owning document's source string,
/// rather than a pretty printer that would erase syntax trivia.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Ast {
    root: NodeId,
    roots: Vec<NodeId>,
    nodes: BTreeMap<NodeId, Node>,
}
impl Ast {
    pub fn root(&self) -> NodeId {
        self.root
    }
    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }
    pub fn elements(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values().filter(|n| n.is_element())
    }
    /// Visit syntax nodes in lexical order, independently of stable node IDs.
    /// Moving an element changes this order without changing its identity.
    pub fn nodes_in_source_order(&self) -> impl Iterator<Item = &Node> {
        self.walk(self.roots.iter().rev().copied().collect())
    }
    /// Visit syntax nodes and their containing XAML name scopes in source order.
    /// Scope propagation visits each node once instead of walking its ancestors
    /// for every name or reference in a language-service indexing pass.
    pub fn nodes_with_name_scopes(&self) -> impl Iterator<Item = (&Node, NodeId)> {
        let mut pending: Vec<_> = self.roots.iter().rev().map(|id| (*id, self.root)).collect();
        std::iter::from_fn(move || {
            let (id, scope) = pending.pop()?;
            let node = self.node(id)?;
            let child_scope = if defines_name_scope(node) { id } else { scope };
            pending.extend(
                node.children
                    .iter()
                    .rev()
                    .map(|child| (*child, child_scope)),
            );
            Some((node, scope))
        })
    }
    /// Visit a node and all its syntax descendants in source order. An unknown
    /// root produces an empty iterator; unrelated document nodes are not visited.
    pub fn subtree(&self, root: NodeId) -> impl Iterator<Item = &Node> {
        self.walk(self.node(root).map(|_| vec![root]).unwrap_or_default())
    }
    fn walk(&self, mut pending: Vec<NodeId>) -> impl Iterator<Item = &Node> {
        std::iter::from_fn(move || {
            let node = self.node(pending.pop()?)?;
            pending.extend(node.children.iter().rev().copied());
            Some(node)
        })
    }
    pub fn element_children(&self, parent: NodeId) -> Vec<NodeId> {
        self.node(parent)
            .map(|n| {
                n.children
                    .iter()
                    .copied()
                    .filter(|id| self.node(*id).is_some_and(Node::is_element))
                    .collect()
            })
            .unwrap_or_default()
    }
    pub fn find_name(&self, name: &str) -> Option<NodeId> {
        self.elements()
            .find(|n| n.xaml_name() == Some(name))
            .map(|n| n.id)
    }
    /// The containing XAML name scope. Template contents have their own scope;
    /// a template's declaration belongs to its parent's scope. Unknown nodes and
    /// top-level declarations use the document root.
    pub fn name_scope(&self, node: NodeId) -> NodeId {
        let mut current = self.node(node).and_then(|node| node.parent);
        while let Some(id) = current {
            let Some(ancestor) = self.node(id) else {
                break;
            };
            if defines_name_scope(ancestor) {
                return id;
            }
            current = ancestor.parent;
        }
        self.root()
    }
    pub fn node_at(&self, byte_offset: usize) -> Option<NodeId> {
        self.containing_node(byte_offset, false)
    }
    pub fn element_at(&self, byte_offset: usize) -> Option<NodeId> {
        self.containing_node(byte_offset, true)
    }
    /// Siblings are ordered, non-overlapping source intervals. Descend only the
    /// containing branch, using binary search at each level instead of scanning
    /// the entire document on every caret or pointer movement.
    fn containing_node(&self, byte_offset: usize, elements_only: bool) -> Option<NodeId> {
        let mut siblings = self.roots.as_slice();
        let mut result = None;
        loop {
            let end = siblings.partition_point(|id| self.nodes[id].span.start <= byte_offset);
            let Some(id) = end.checked_sub(1).and_then(|index| siblings.get(index)) else {
                break;
            };
            let node = &self.nodes[id];
            if !node.span.contains(byte_offset) {
                break;
            }
            if !elements_only || node.is_element() {
                result = Some(node.id);
            }
            siblings = &node.children;
        }
        result
    }
    pub fn is_descendant_of(&self, node: NodeId, ancestor: NodeId) -> bool {
        let mut current = self.node(node).and_then(|n| n.parent);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.node(id).and_then(|n| n.parent);
        }
        false
    }
    /// Namespaces in scope at an element, nearest declaration wins.
    pub fn namespaces(&self, node: NodeId) -> BTreeMap<String, String> {
        let mut result = BTreeMap::new();
        let mut current = Some(node);
        while let Some(id) = current {
            let Some(node) = self.node(id) else {
                break;
            };
            for attr in &node.attributes {
                if attr.name == "xmlns" {
                    result
                        .entry(String::new())
                        .or_insert_with(|| attr.value.clone());
                } else if let Some(prefix) = attr.name.strip_prefix("xmlns:") {
                    result
                        .entry(prefix.to_owned())
                        .or_insert_with(|| attr.value.clone());
                }
            }
            current = node.parent;
        }
        result
            .entry("xml".into())
            .or_insert_with(|| "http://www.w3.org/XML/1998/namespace".into());
        result
    }
}

fn defines_name_scope(node: &Node) -> bool {
    matches!(
        node.local_name(),
        "ControlTemplate" | "DataTemplate" | "ItemsPanelTemplate" | "FuncTemplate"
    )
}
