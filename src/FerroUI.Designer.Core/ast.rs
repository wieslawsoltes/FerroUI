use crate::{DesignerError, MarkupValue};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};

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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn node_at(&self, byte_offset: usize) -> Option<NodeId> {
        self.nodes()
            .filter(|n| n.span.contains(byte_offset))
            .min_by_key(|n| n.span.len())
            .map(|n| n.id)
    }
    pub fn element_at(&self, byte_offset: usize) -> Option<NodeId> {
        self.elements()
            .filter(|n| n.span.contains(byte_offset))
            .min_by_key(|n| n.span.len())
            .map(|n| n.id)
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

    pub(crate) fn parse(source: &str) -> Result<Self, DesignerError> {
        let options = roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 1_000_000,
        };
        roxmltree::Document::parse_with_options(source, options).map_err(|error| {
            let pos = error.pos();
            DesignerError::Parse {
                message: error.to_string(),
                line: pos.row,
                column: pos.col,
            }
        })?;
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
            let parent = stack.last().copied();
            let (kind, end, content) = if remaining.starts_with("<!--") {
                let end = offset + remaining.find("-->").unwrap() + 3;
                (
                    NodeKind::Comment,
                    end,
                    Some(source[offset + 4..end - 3].to_owned()),
                )
            } else if remaining.starts_with("<![CDATA[") {
                let end = offset + remaining.find("]]>").unwrap() + 3;
                (
                    NodeKind::CData,
                    end,
                    Some(source[offset + 9..end - 3].to_owned()),
                )
            } else if remaining.starts_with("<?") {
                let end = offset + remaining.find("?>").unwrap() + 2;
                (
                    NodeKind::ProcessingInstruction,
                    end,
                    Some(source[offset + 2..end - 2].to_owned()),
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

    /// Match named nodes globally, then unchanged subtree fingerprints, then
    /// sibling position under matched parents. Explicit edits may pin identities
    /// by source offsets, preventing ambiguity when duplicate siblings move.
    pub(crate) fn reconcile(
        &mut self,
        old: &Ast,
        old_source: &str,
        source: &str,
        next_id: &mut NodeId,
        pins: &[(usize, NodeId)],
    ) {
        let mut mapped = HashMap::<NodeId, NodeId>::new();
        let mut used = HashSet::new();
        let by_offset: HashMap<_, _> = self
            .nodes()
            .map(|node| (node.span.start, node.id))
            .collect();
        for &(offset, old_id) in pins {
            if let (Some(old_node), Some(&new_id)) = (old.node(old_id), by_offset.get(&offset)) {
                let node = self.node(new_id).unwrap();
                if compatible(node, old_node)
                    && !mapped.contains_key(&new_id)
                    && used.insert(old_id)
                {
                    mapped.insert(new_id, old_id);
                }
            }
        }
        let mut old_named = HashMap::<(Option<String>, String, String), Vec<NodeId>>::new();
        let mut new_named = HashMap::<(Option<String>, String, String), Vec<NodeId>>::new();
        for node in old.elements() {
            if let Some(name) = node.xaml_name() {
                old_named
                    .entry((
                        node.namespace_uri.clone(),
                        node.local_name().into(),
                        name.into(),
                    ))
                    .or_default()
                    .push(node.id);
            }
        }
        for node in self.elements() {
            if let Some(name) = node.xaml_name() {
                new_named
                    .entry((
                        node.namespace_uri.clone(),
                        node.local_name().into(),
                        name.into(),
                    ))
                    .or_default()
                    .push(node.id);
            }
        }
        for (key, new_ids) in new_named {
            if let Some(old_ids) = old_named.get(&key) {
                if new_ids.len() == 1
                    && old_ids.len() == 1
                    && !mapped.contains_key(&new_ids[0])
                    && used.insert(old_ids[0])
                {
                    mapped.insert(new_ids[0], old_ids[0]);
                }
            }
        }
        if !mapped.contains_key(&self.root)
            && compatible(self.node(self.root).unwrap(), old.node(old.root).unwrap())
            && used.insert(old.root)
        {
            mapped.insert(self.root, old.root);
        }
        // Match sibling groups in source order. Fingerprints are assembled bottom
        // up from each token exactly once; even deeply nested source has linear
        // hashing cost. They only select identity candidates, never validate XML.
        let old_fingerprints = fingerprints(old, old_source);
        let new_fingerprints = fingerprints(self, source);
        reconcile_siblings(
            &self.roots,
            &old.roots,
            self,
            old,
            &new_fingerprints,
            &old_fingerprints,
            &mut mapped,
            &mut used,
            next_id,
        );
        let mut parents: Vec<_> = self
            .elements()
            .map(|node| (node.span.start, node.id))
            .collect();
        parents.sort_unstable();
        for (_, parent) in parents {
            let new_children = &self.node(parent).unwrap().children;
            let old_children = mapped
                .get(&parent)
                .and_then(|id| old.node(*id))
                .map(|node| node.children.as_slice())
                .unwrap_or(&[]);
            reconcile_siblings(
                new_children,
                old_children,
                self,
                old,
                &new_fingerprints,
                &old_fingerprints,
                &mut mapped,
                &mut used,
                next_id,
            );
        }
        self.remap(&mapped);
    }
    pub(crate) fn allocate(&mut self, next_id: &mut NodeId) {
        let map = self
            .nodes
            .keys()
            .map(|old| {
                let id = *next_id;
                *next_id += 1;
                (*old, id)
            })
            .collect();
        self.remap(&map);
    }
    fn remap(&mut self, map: &HashMap<NodeId, NodeId>) {
        self.root = map[&self.root];
        self.roots.iter_mut().for_each(|id| *id = map[id]);
        self.nodes = std::mem::take(&mut self.nodes)
            .into_values()
            .map(|mut node| {
                node.id = map[&node.id];
                node.parent = node.parent.map(|id| map[&id]);
                node.children.iter_mut().for_each(|id| *id = map[id]);
                (node.id, node)
            })
            .collect();
    }
}

fn compatible(a: &Node, b: &Node) -> bool {
    a.kind == b.kind && a.local_name() == b.local_name() && a.namespace_uri == b.namespace_uri
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

// A matching key is semantic with respect to namespace prefix spelling. Renaming
// a prefix does not change the object that the editor selected.
type MatchKey = (NodeKind, Option<String>, String);
fn match_key(node: &Node) -> MatchKey {
    (
        node.kind,
        node.namespace_uri.clone(),
        node.local_name().into(),
    )
}

#[allow(clippy::too_many_arguments)]
fn reconcile_siblings(
    new_children: &[NodeId],
    old_children: &[NodeId],
    new: &Ast,
    old: &Ast,
    new_hashes: &HashMap<NodeId, u64>,
    old_hashes: &HashMap<NodeId, u64>,
    mapped: &mut HashMap<NodeId, NodeId>,
    used: &mut HashSet<NodeId>,
    next_id: &mut NodeId,
) {
    let mut exact = HashMap::<u64, VecDeque<NodeId>>::new();
    let mut kinds = HashMap::<MatchKey, VecDeque<NodeId>>::new();
    for &id in old_children {
        if used.contains(&id) {
            continue;
        }
        exact.entry(old_hashes[&id]).or_default().push_back(id);
        kinds
            .entry(match_key(old.node(id).unwrap()))
            .or_default()
            .push_back(id);
    }
    // First reserve every unchanged sibling before assigning changed siblings,
    // so an insertion cannot steal an existing sibling's identity.
    for &id in new_children {
        if mapped.contains_key(&id) {
            continue;
        }
        if let Some(candidates) = exact.get_mut(&new_hashes[&id]) {
            while let Some(old_id) = candidates.pop_front() {
                if !used.contains(&old_id)
                    && compatible(new.node(id).unwrap(), old.node(old_id).unwrap())
                {
                    mapped.insert(id, old_id);
                    used.insert(old_id);
                    break;
                }
            }
        }
    }
    for &id in new_children {
        if mapped.contains_key(&id) {
            continue;
        }
        if let Some(candidates) = kinds.get_mut(&match_key(new.node(id).unwrap())) {
            while let Some(old_id) = candidates.pop_front() {
                if used.insert(old_id) {
                    mapped.insert(id, old_id);
                    break;
                }
            }
        }
        mapped.entry(id).or_insert_with(|| {
            let id = *next_id;
            *next_id += 1;
            id
        });
    }
}

fn fingerprints(ast: &Ast, source: &str) -> HashMap<NodeId, u64> {
    let mut order: Vec<_> = ast.nodes().map(|node| (node.span.start, node.id)).collect();
    order.sort_unstable_by(|a, b| b.cmp(a));
    let mut hashes = HashMap::<NodeId, u64>::with_capacity(order.len());
    for (_, id) in order {
        let node = ast.node(id).unwrap();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        node.kind.hash(&mut hash);
        node.namespace_uri.hash(&mut hash);
        node.local_name().hash(&mut hash);
        if node.is_element() {
            source[node.opening_span.range()].hash(&mut hash);
            for child in &node.children {
                hashes[child].hash(&mut hash);
            }
            if let Some(closing) = node.closing_span {
                source[closing.range()].hash(&mut hash);
            }
        } else {
            source[node.span.range()].hash(&mut hash);
        }
        hashes.insert(id, hash.finish());
    }
    hashes
}
