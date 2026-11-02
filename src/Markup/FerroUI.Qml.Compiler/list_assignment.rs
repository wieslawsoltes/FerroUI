//! Merge inherited declarative object lists. Ordinary sequence bindings retain
//! their regular replacement semantics. Replaced objects continue to own ids.
use super::*;
use std::collections::HashMap;

fn take_objects(binding: &mut Binding) -> Option<Vec<Object>> {
    if !matches!(binding, Binding::Objects(_) | Binding::Object(_)) {
        return None;
    }
    match std::mem::replace(binding, Binding::Objects(vec![])) {
        Binding::Objects(items) => Some(items),
        Binding::Object(item) => Some(vec![item]),
        _ => unreachable!(),
    }
}

pub(super) fn merge(base: &mut Object, site: &mut Object) {
    let mut lists = HashMap::new();
    let mut default_list = None;
    for member in &base.members {
        if let MemberKind::Property {
            name,
            ty,
            modifiers,
            ..
        } = &member.kind
        {
            if ty.starts_with("list<") {
                lists.insert(name.clone(), (ty.clone(), modifiers.default));
                if modifiers.default {
                    default_list = Some(name.clone());
                }
            }
        }
    }
    if lists.is_empty() {
        return;
    }
    if let Some(name) = default_list {
        let mut children = Vec::new();
        let mut start = None;
        site.members.retain_mut(|member| {
            if matches!(member.kind, MemberKind::Child(_)) {
                start.get_or_insert((member.scope, member.span));
                let MemberKind::Child(child) =
                    std::mem::replace(&mut member.kind, MemberKind::Id(String::new()))
                else {
                    unreachable!()
                };
                children.push(child);
                false
            } else {
                true
            }
        });
        if let Some((scope, span)) = start {
            site.members.push(Member {
                annotation_imports: None,
                scope,
                span,
                kind: MemberKind::Binding {
                    name,
                    value: Binding::Objects(children),
                },
            });
        }
    }
    let assignments: HashMap<_, _> = site
        .members
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            let MemberKind::Binding { name, .. } = &m.kind else {
                return None;
            };
            lists.contains_key(name).then(|| (name.clone(), i))
        })
        .collect();
    let mut retained = vec![];
    let depth = base.composite_types.len();
    for (index, member) in base.members.iter_mut().enumerate() {
        let (name, old) = match &mut member.kind {
            MemberKind::Property {
                name,
                value: Some(old),
                ..
            }
            | MemberKind::Binding { name, value: old } => (name, old),
            _ => continue,
        };
        let Some(&i) = assignments.get(name) else {
            continue;
        };
        let Some(mut objects) = take_objects(old) else {
            continue;
        };
        let (ty, default) = &lists[name];
        let MemberKind::Binding { value: next, .. } = &mut site.members[i].kind else {
            unreachable!()
        };
        let append = match base.list_assignment {
            ListAssignment::Append => true,
            ListAssignment::Replace => false,
            ListAssignment::ReplaceIfNotDefault => *default,
        };
        if append {
            if let Some(new) = take_objects(next) {
                objects.extend(new);
                *next = Binding::Objects(objects);
                continue;
            }
        }
        retained.push(Member {
            annotation_imports: member.annotation_imports.clone(),
            scope: member.scope,
            span: member.span,
            kind: MemberKind::Property {
                name: format!("$qml:replacedList{depth}_{index}"),
                ty: ty.clone(),
                modifiers: Modifiers::default(),
                value: Some(Binding::Objects(objects)),
            },
        });
    }
    base.members.extend(retained);
}
