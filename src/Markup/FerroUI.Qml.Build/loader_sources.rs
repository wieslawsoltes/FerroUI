//! Discover constant Loader URLs without evaluating the QML or walking a directory.
use ferroui_qml_compiler::{Binding, Document, Expr, Member, MemberKind, Object};

pub(super) fn sources(document: &Document) -> Vec<&str> {
    fn object<'a>(node: &'a Object, names: &[String], out: &mut Vec<&'a str>) {
        members(&node.members, names, names.contains(&node.type_name), out);
    }
    fn members<'a>(list: &'a [Member], names: &[String], loader: bool, out: &mut Vec<&'a str>) {
        for member in list {
            match &member.kind {
                MemberKind::Binding {
                    name,
                    value: Binding::Expr(Expr::String(url)),
                } if loader && name == "source" => out.push(url),
                MemberKind::Group { members: list, .. } => members(list, names, false, out),
                MemberKind::Child(child)
                | MemberKind::Component { object: child, .. }
                | MemberKind::OnProperty { object: child, .. } => object(child, names, out),
                MemberKind::Property {
                    value: Some(binding),
                    ..
                }
                | MemberKind::Binding { value: binding, .. } => match binding {
                    Binding::Object(child) => object(child, names, out),
                    Binding::Objects(children) => {
                        for child in children {
                            object(child, names, out);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
    let names = document
        .imports
        .iter()
        .filter(|import| !import.is_path && import.uri == "QtQuick")
        .map(|import| {
            import
                .alias
                .as_ref()
                .map_or_else(|| "Loader".into(), |alias| format!("{alias}.Loader"))
        })
        .collect::<Vec<_>>();
    let mut result = Vec::new();
    if !names.is_empty() {
        object(&document.root, &names, &mut result);
    }
    result
}
