//! Give each composite instance its own ids while preserving lexical locals.
use crate::{Binding, Expr, Member, MemberKind, Object, Stmt};
use std::collections::{HashMap, HashSet};

pub(super) fn isolate(root: &mut Object, serial: usize, root_id: &str, scope: u32) {
    let mut ids = HashMap::new();
    let previous = super::link::object_id(root).map(str::to_owned);
    visit_objects(root, &mut |object| {
        if let Some(id) = super::link::object_id(object) {
            ids.insert(id.to_owned(), format!("$qml:{serial}:{id}"));
        }
    });
    if let Some(previous) = previous {
        ids.insert(previous, root_id.to_owned());
    } else {
        root.members.push(Member {
            scope: Some(scope),
            kind: MemberKind::Id(root_id.to_owned()),
            span: root.span,
        });
    }
    visit_objects(root, &mut |object| {
        let binding = matches!(
            object.type_name.rsplit('.').next(),
            Some("Binding" | "PropertyChanges")
        );
        for member in &mut object.members {
            if binding {
                // The first segment names an object in the declaration's id scope.
                let name = match &mut member.kind {
                    MemberKind::Binding { name, .. } if name.contains('.') => Some(name),
                    MemberKind::Group { name, .. } => Some(name),
                    _ => None,
                };
                if let Some(name) = name {
                    let id = name.split('.').next().expect("nonempty binding target");
                    if let Some(private) = ids.get(id) {
                        name.replace_range(..id.len(), private);
                    }
                }
            }
            rewrite(member, &ids, root_id, scope);
        }
    });
}

fn visit_objects(object: &mut Object, visit: &mut impl FnMut(&mut Object)) {
    visit(object);
    visit_members(&mut object.members, visit);
}
fn visit_members(members: &mut [Member], visit: &mut impl FnMut(&mut Object)) {
    for member in members {
        match &mut member.kind {
            MemberKind::Child(child) | MemberKind::OnProperty { object: child, .. } => {
                visit_objects(child, visit)
            }
            MemberKind::Group { members, .. } => visit_members(members, visit),
            MemberKind::Binding { value, .. }
            | MemberKind::Property {
                value: Some(value), ..
            } => match value {
                Binding::Object(child) => visit_objects(child, visit),
                Binding::Objects(children) => {
                    for child in children {
                        visit_objects(child, visit);
                    }
                }
                _ => (),
            },
            // An inline declaration starts a new template scope.
            _ => (),
        }
    }
}

fn rewrite(member: &mut Member, ids: &HashMap<String, String>, root: &str, scope: u32) {
    member.scope = Some(scope);
    match &mut member.kind {
        MemberKind::Id(id) => {
            if let Some(new) = ids.get(id) {
                id.clone_from(new);
            }
        }
        MemberKind::Binding { value, .. }
        | MemberKind::Property {
            value: Some(value), ..
        } => match value {
            Binding::Expr(expr) => expression(expr, ids, root, &HashSet::new()),
            Binding::Script(body) => statements(body, ids, root, HashSet::new()),
            _ => (),
        },
        MemberKind::Group { members, .. } => {
            for m in members {
                rewrite(m, ids, root, scope);
            }
        }
        MemberKind::Handler {
            parameters, body, ..
        } => statements(body, ids, root, parameters.iter().cloned().collect()),
        MemberKind::Function {
            parameters, body, ..
        } => statements(
            body,
            ids,
            root,
            parameters.iter().map(|p| p.name.clone()).collect(),
        ),
        _ => (),
    }
}
fn expression(
    expr: &mut Expr,
    ids: &HashMap<String, String>,
    root: &str,
    locals: &HashSet<String>,
) {
    match expr {
        Expr::JavaScript(js) => {
            js.root = Some(root.into());
            js.ids
                .extend(ids.iter().map(|(a, b)| (a.clone(), b.clone())));
        }
        Expr::Name(name) if !locals.contains(name) => {
            if let Some(id) = ids.get(name) {
                name.clone_from(id);
            } else {
                *expr = Expr::ScopedName {
                    name: std::mem::take(name),
                    root: root.to_owned(),
                };
            }
        }
        Expr::Unary(_, item) | Expr::Member(item, _) => expression(item, ids, root, locals),
        Expr::Binary(_, a, b) | Expr::Index(a, b) => {
            expression(a, ids, root, locals);
            expression(b, ids, root, locals);
        }
        Expr::Conditional(c, a, b) => {
            for e in [c, a, b] {
                expression(e, ids, root, locals);
            }
        }
        Expr::Call(callee, args) => {
            expression(callee, ids, root, locals);
            for arg in args {
                expression(arg, ids, root, locals);
            }
        }
        Expr::Record(fields) => {
            for (_, value) in fields {
                expression(value, ids, root, locals);
            }
        }
        Expr::Array(items) => {
            for item in items {
                expression(item, ids, root, locals);
            }
        }
        _ => (),
    }
}
fn statements(
    body: &mut [Stmt],
    ids: &HashMap<String, String>,
    root: &str,
    mut locals: HashSet<String>,
) {
    for stmt in body.iter() {
        if let Stmt::Let { name, .. } = stmt {
            locals.insert(name.clone());
        }
    }
    for stmt in body {
        match stmt {
            Stmt::JavaScript(js) => {
                js.root = Some(root.into());
                js.ids
                    .extend(ids.iter().map(|(a, b)| (a.clone(), b.clone())));
            }
            Stmt::Expr(expr)
            | Stmt::Return(Some(expr))
            | Stmt::Let {
                value: Some(expr), ..
            } => expression(expr, ids, root, &locals),
            Stmt::Assign { target, value, .. } => {
                expression(target, ids, root, &locals);
                expression(value, ids, root, &locals);
            }
            Stmt::If {
                test,
                then_body,
                else_body,
            } => {
                expression(test, ids, root, &locals);
                statements(then_body, ids, root, locals.clone());
                statements(else_body, ids, root, locals.clone());
            }
            Stmt::While { test, body, .. } => {
                expression(test, ids, root, &locals);
                statements(body, ids, root, locals.clone());
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                let mut scope = locals.clone();
                if let Some(init) = init {
                    if let Stmt::Let { name, .. } = init.as_ref() {
                        scope.insert(name.clone());
                    }
                    statements(std::slice::from_mut(init), ids, root, scope.clone());
                }
                if let Some(test) = test {
                    expression(test, ids, root, &scope);
                }
                if let Some(update) = update {
                    statements(std::slice::from_mut(update), ids, root, scope.clone());
                }
                statements(body, ids, root, scope);
            }
            Stmt::Block(body) => statements(body, ids, root, locals.clone()),
            _ => (),
        }
    }
}
