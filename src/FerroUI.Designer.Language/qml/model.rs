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

pub(crate) struct Node {
    pub ty: String,
    pub span: SourceSpan,
    pub type_span: SourceSpan,
    pub scope: usize,
    pub parent: Option<usize>,
    pub declarations: BTreeMap<String, (String, SourceSpan)>,
    pub id: Option<(String, SourceSpan)>,
    reads: Vec<ScriptReference>,
}

pub(crate) struct Model {
    pub imports: Vec<ferroui_qml_compiler::Import>,
    pub nodes: Vec<Node>,
    pub outline: Symbol,
}

impl Model {
    pub fn new(source: &str, document: Document) -> Self {
        let mut nodes = Vec::new();
        let outline = object(source, &document.root, None, None, &mut nodes);
        Self {
            imports: document.imports,
            nodes,
            outline,
        }
    }

    pub fn owner(&self, at: usize) -> Option<usize> {
        self.nodes
            .iter()
            .rposition(|n| n.span.start <= at && at < n.span.end)
    }

    pub fn id(&self, owner: usize, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| {
            n.scope == self.nodes[owner].scope && n.id.as_ref().is_some_and(|(id, _)| id == name)
        })
    }

    pub fn receiver(&self, owner: usize, name: &str) -> Option<usize> {
        match name {
            "this" => Some(owner),
            "parent" => self.nodes[owner].parent,
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
        let read = node
            .reads
            .iter()
            .find(|r| contains(r.span) && r.name == name)?;
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

pub(super) enum Target {
    Id(usize),
    Member { owner: usize, lexical: bool },
}

fn object(
    source: &str,
    obj: &Object,
    parent: Option<usize>,
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
        id: None,
    });
    let children = members(source, &obj.members, index, nodes);
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
                children.push(object(source, o, Some(owner), Some(scope), nodes))
            }
            Some(Binding::Objects(objects)) => children.extend(
                objects
                    .iter()
                    .map(|o| object(source, o, Some(owner), Some(scope), nodes)),
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
                children.push(object(source, obj, None, None, nodes));
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
                out.push(object(source, obj, Some(owner), child_scope, nodes));
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
