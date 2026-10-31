//! Give each composite instance its own ids while preserving lexical locals.
use crate::{Binding, Expr, Member, MemberKind, Object, Stmt};
use std::collections::{HashMap, HashSet};

pub(super) fn isolate(root: &mut Object, serial: usize, root_id: &str) {
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
            kind: MemberKind::Id(root_id.to_owned()),
            span: root.span,
        });
    }
    visit_objects(root, &mut |object| {
        for member in &mut object.members {
            rewrite(member, &ids, root_id);
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
            MemberKind::Child(child) => visit_objects(child, visit),
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

fn rewrite(member: &mut Member, ids: &HashMap<String, String>, root: &str) {
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
                rewrite(m, ids, root);
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
            Stmt::Block(body) => statements(body, ids, root, locals.clone()),
            _ => (),
        }
    }
}
