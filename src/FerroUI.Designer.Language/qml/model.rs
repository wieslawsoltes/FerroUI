//! An index of authored declarations and exact property reads. Object handles are
//! local to one parse, which keeps scopes isolated across edits and documents.
use ferroui_qml_compiler::editor::{script_references, ScriptReference};
use ferroui_qml_compiler::{
    editor::member_name_span, Binding, Document, Expr, Member, MemberKind as M, Object, SourceSpan,
    Stmt,
};
use std::collections::BTreeMap;

pub(crate) struct Symbol {
    pub name: String,
    pub detail: String,
    pub kind: u32,
    pub span: SourceSpan,
    pub selection: SourceSpan,
    pub children: Vec<Symbol>,
}

// An inline component is nested in the source, but its root has no outer
// QML parent. Keep both relationships without a second index per object.
#[derive(Clone, Copy)]
enum Parent {
    File,
    Object(usize),
    Inline(usize),
}

pub(crate) struct Node {
    pub ty: String,
    pub span: SourceSpan,
    pub type_span: SourceSpan,
    pub scope: usize,
    parent: Parent,
    pub declarations: BTreeMap<String, (String, SourceSpan)>,
    pub id: Option<(String, SourceSpan)>,
    reads: Vec<ScriptReference>,
    indexed_reads: bool,
}

pub(crate) struct Model {
    pub imports: Vec<ferroui_qml_compiler::Import>,
    pub nodes: Vec<Node>,
    pub outline: Symbol,
    // Handles only: ID strings and declaration spans remain on the nodes.
    ids: Vec<usize>,
}

impl Model {
    pub fn new(source: &str, document: Document) -> Self {
        let mut nodes = Vec::new();
        let outline = object(source, &document.root, Parent::File, None, &mut nodes);
        let mut ids: Vec<_> = (0..nodes.len())
            .filter(|&i| nodes[i].id.is_some())
            .collect();
        ids.sort_unstable_by(|&a, &b| nodes[a].id_key().cmp(&nodes[b].id_key()).then(a.cmp(&b)));
        Self {
            imports: document.imports,
            nodes,
            outline,
            ids,
        }
    }

    pub fn owner(&self, at: usize) -> Option<usize> {
        if self.nodes.len() <= 16 {
            return self
                .nodes
                .iter()
                .rposition(|n| n.span.start <= at && at < n.span.end);
        }
        // Nodes are emitted in source preorder. Start at the last object that
        // begins before the cursor and climb out of any already closed scopes.
        let mut index = self
            .nodes
            .partition_point(|n| n.span.start <= at)
            .checked_sub(1)?;
        loop {
            let node = &self.nodes[index];
            if at < node.span.end {
                return Some(index);
            }
            index = match node.parent {
                Parent::File => return None,
                Parent::Object(i) | Parent::Inline(i) => i,
            };
        }
    }

    pub fn id(&self, owner: usize, name: &str) -> Option<usize> {
        let key = (self.nodes[owner].scope, name);
        let pos = self.ids.partition_point(|&i| self.nodes[i].id_key() < key);
        self.ids
            .get(pos)
            .copied()
            .filter(|&i| self.nodes[i].id_key() == key)
    }

    pub fn scoped_ids(&self, scope: usize) -> impl Iterator<Item = &Node> {
        let start = self.ids.partition_point(|&i| self.nodes[i].scope < scope);
        self.ids[start..]
            .iter()
            .map(|&i| &self.nodes[i])
            .take_while(move |n| n.scope == scope)
    }

    pub fn receiver(&self, owner: usize, name: &str) -> Option<usize> {
        match name {
            "this" => Some(owner),
            "parent" => match self.nodes[owner].parent {
                Parent::Object(parent) => Some(parent),
                _ => None,
            },
            id => self.id(owner, id),
        }
    }

    /// Resolve the token to a QML receiver. Declarations and assignment names
    /// have an explicit receiver; free script identifiers may also use the root.
    pub fn target(&self, owner: usize, name: &str, at: usize) -> Option<Target> {
        let node = &self.nodes[owner];
        let contains = |s: SourceSpan| s.start <= at && at < s.end;
        if node
            .id
            .as_ref()
            .is_some_and(|(id, s)| id == name && contains(*s))
        {
            return Some(Target::Id(owner));
        }
        if node
            .declarations
            .get(name)
            .is_some_and(|(_, s)| contains(*s))
        {
            return Some(Target::Member {
                owner,
                lexical: false,
            });
        }
        let read = node.read(at, name)?;
        if let Some(receiver) = &read.receiver {
            Some(Target::Member {
                owner: self.receiver(owner, receiver)?,
                lexical: false,
            })
        } else if let Some(id) = self.id(owner, name) {
            Some(Target::Id(id))
        } else {
            Some(Target::Member {
                owner,
                lexical: true,
            })
        }
    }
}

impl Node {
    fn id_key(&self) -> (usize, &str) {
        (self.scope, self.id.as_ref().expect("indexed ID").0.as_str())
    }

    fn index_reads(&mut self) {
        // Small declarations are cheaper to scan. Leave overlapping fallback
        // spans in AST order to retain their original lookup precedence.
        if self.reads.len() <= 16 {
            return;
        }
        if !self
            .reads
            .windows(2)
            .all(|w| w[0].span.end <= w[1].span.start)
        {
            // Some AST visitors report an assignment's left side after its
            // value. Check for overlapping fallback spans before reordering.
            let mut spans: Vec<_> = self.reads.iter().map(|r| r.span).collect();
            spans.sort_unstable_by_key(|s| s.start);
            if spans.windows(2).any(|w| w[0].end > w[1].start) {
                return;
            }
            self.reads.sort_unstable_by_key(|r| r.span.start);
        }
        self.indexed_reads = true;
    }

    fn read(&self, at: usize, name: &str) -> Option<&ScriptReference> {
        let matches = |r: &ScriptReference| r.span.start <= at && at < r.span.end && r.name == name;
        if !self.indexed_reads {
            return self.reads.iter().find(|r| matches(r));
        }
        let after = self.reads.partition_point(|r| r.span.start <= at);
        let r = &self.reads[after.checked_sub(1)?];
        matches(r).then_some(r)
    }
}

pub(super) enum Target {
    Id(usize),
    Member { owner: usize, lexical: bool },
}

fn object(
    source: &str,
    obj: &Object,
    parent: Parent,
    scope: Option<usize>,
    nodes: &mut Vec<Node>,
) -> Symbol {
    let index = nodes.len();
    let header = ferroui_qml_compiler::editor::object_type_span(source, obj).unwrap_or(obj.span);
    let scope = scope.unwrap_or(index);
    nodes.push(Node {
        ty: obj.type_name.clone(),
        span: obj.span,
        type_span: header,
        scope,
        parent,
        declarations: BTreeMap::new(),
        reads: Vec::new(),
        indexed_reads: false,
        id: None,
    });
    let children = members(source, &obj.members, index, nodes);
    nodes[index].index_reads();
    Symbol {
        name: nodes[index].id.as_ref().map_or_else(
            || obj.type_name.clone(),
            |(id, _)| format!("{} ({id})", obj.type_name),
        ),
        detail: obj.type_name.clone(),
        kind: 5,
        span: obj.span,
        selection: header,
        children,
    }
}

fn members(source: &str, list: &[Member], owner: usize, nodes: &mut Vec<Node>) -> Vec<Symbol> {
    let scope = nodes[owner].scope;
    let mut out = Vec::new();
    for m in list {
        let mut children = Vec::new();
        let selection = member_name_span(source, m).unwrap_or(m.span);
        let binding = match &m.kind {
            M::Property { value, .. } => value.as_ref(),
            M::Binding { value, .. } => Some(value),
            _ => None,
        };
        match binding {
            Some(Binding::Object(o)) => {
                children.push(object(source, o, Parent::Object(owner), Some(scope), nodes))
            }
            Some(Binding::Objects(objects)) => children.extend(
                objects
                    .iter()
                    .map(|o| object(source, o, Parent::Object(owner), Some(scope), nodes)),
            ),
            Some(Binding::Expr(Expr::JavaScript(script))) => {
                nodes[owner].reads.extend(script_references(script, &[]));
            }
            Some(Binding::Script(stmts)) => collect_body(&mut nodes[owner], stmts, &[]),
            _ => {}
        }
        match &m.kind {
            M::Function {
                parameters, body, ..
            } => collect_body(
                &mut nodes[owner],
                body,
                &parameters
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>(),
            ),
            M::Handler {
                body, parameters, ..
            } => collect_body(
                &mut nodes[owner],
                body,
                &parameters.iter().map(String::as_str).collect::<Vec<_>>(),
            ),
            _ => {}
        }
        let (name, detail, kind) = match &m.kind {
            M::Id(name) => {
                nodes[owner].id = Some((name.clone(), selection));
                (name, "id", 13)
            }
            M::Property { name, ty, .. } => (name, ty.as_str(), 7),
            M::Signal { name, .. } => (name, "signal", 24),
            M::Function { name, .. } => (name, "function", 6),
            M::Handler { name, .. } => (name, "handler", 24),
            M::Binding { name, .. } => {
                if !name.contains('.') {
                    nodes[owner].reads.push(ScriptReference {
                        span: selection,
                        name: name.clone(),
                        receiver: Some("this".into()),
                    });
                }
                (name, "binding", 7)
            }
            M::Enum { name, .. } => (name, "enum", 10),
            M::Component { name, object: obj } => {
                children.push(object(source, obj, Parent::Inline(owner), None, nodes));
                (name, "component", 5)
            }
            M::Group {
                name,
                members: items,
            } => {
                children = members(source, items, owner, nodes);
                (name, "group", 3)
            }
            M::Child(obj) | M::OnProperty { object: obj, .. } => {
                let child_scope =
                    (nodes[owner].ty.rsplit('.').next() != Some("Component")).then_some(scope);
                out.push(object(
                    source,
                    obj,
                    Parent::Object(owner),
                    child_scope,
                    nodes,
                ));
                continue;
            }
        };
        if matches!(
            m.kind,
            M::Property { .. }
                | M::Function { .. }
                | M::Signal { .. }
                | M::Enum { .. }
                | M::Component { .. }
        ) {
            nodes[owner]
                .declarations
                .insert(name.clone(), (detail.into(), selection));
        }
        out.push(Symbol {
            name: name.clone(),
            detail: detail.into(),
            kind,
            span: m.span,
            selection,
            children,
        });
    }
    out
}

fn collect_body(node: &mut Node, stmts: &[Stmt], params: &[&str]) {
    for stmt in stmts {
        if let Stmt::JavaScript(script) = stmt {
            node.reads.extend(script_references(script, params));
        }
    }
}
