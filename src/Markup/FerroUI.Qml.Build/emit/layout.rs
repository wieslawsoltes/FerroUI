//! Separate immutable layout records from the code that allocates their owned
//! fields. A repeated composite shares its object factory, including lexical
//! imports and source layers. Instance-specific spans and members stay in data.
use super::*;

pub(super) fn object(object: &Object, out: &mut Writer) {
    let Object {
        bound_component,
        list_assignment,
        source_url,
        source_layers,
        shadows,
        type_name,
        members,
        span,
        type_imports,
        composite_types,
    } = object;
    let mut shape = Writer::default();
    // All shape fields use the same exhaustive, literal-safe emitter as the
    // direct constructor path. The key includes every field except members and span.
    shape.text.push_str("q::Object { members, span");
    macro_rules! fields {
        ($($name:ident),+) => {$(
            shape.text.push_str(concat!(", ", stringify!($name), ": "));
            $name.emit(&mut shape);
        )+};
    }
    fields!(
        bound_component,
        list_assignment,
        source_url,
        source_layers,
        shadows,
        type_name,
        type_imports,
        composite_types
    );
    shape.text.push_str(" }");
    let next = out.shapes.len();
    let index = out.shapes.entry(shape.text).or_insert(next);
    write!(
        out.text,
        "layout::Object(object_shape_{index}, {}, {}, &[",
        span.start, span.end
    )
    .unwrap();
    out.indent += 1;
    for member in members {
        out.new_line();
        emit_member(member, out);
        out.text.push(',');
    }
    out.indent -= 1;
    if !members.is_empty() {
        out.new_line();
    }
    out.text.push_str("])");
}

fn value(binding: &Binding) -> Option<String> {
    let Binding::Expr(expr) = binding else {
        return None;
    };
    Some(match expr {
        Expr::Null => "layout::Value::Null".into(),
        Expr::Undefined => "layout::Value::Undefined".into(),
        Expr::Bool(value) => format!("layout::Value::Bool({value})"),
        Expr::Number(value) => format!("layout::Value::Number({})", value.to_bits()),
        Expr::String(value) => format!("layout::Value::String({value:?})"),
        Expr::Native(i) => format!("layout::Value::Native({i})"),
        _ => return None,
    })
}

fn site(variant: &str, m: &Member, out: &mut Writer) {
    write!(out.text, "layout::Member::{variant}(layout::Site(").unwrap();
    m.scope.emit(out);
    write!(out.text, ", {}, {}), ", m.span.start, m.span.end).unwrap();
}

fn emit_member(m: &Member, out: &mut Writer) {
    if m.annotation_imports.is_none() {
        match &m.kind {
            MemberKind::Id(id) => {
                site("Id", m, out);
                write!(out.text, "{id:?})").unwrap();
                return;
            }
            MemberKind::Binding {
                name,
                value: binding,
            } => {
                if let Some(value) = value(binding) {
                    site("Binding", m, out);
                    write!(out.text, "{name:?}, {value})").unwrap();
                    return;
                }
            }
            MemberKind::Property {
                name,
                ty,
                modifiers,
                value: binding,
            } => {
                let init = match binding {
                    None => Some("None".to_owned()),
                    Some(binding) => value(binding).map(|v| format!("Some({v})")),
                };
                if let Some(init) = init {
                    site("Property", m, out);
                    write!(out.text, "{name:?}, {ty:?}, ").unwrap();
                    if modifiers == &Modifiers::default() {
                        out.text.push_str("layout::PLAIN");
                    } else {
                        modifiers.emit(out);
                    }
                    write!(out.text, ", {init})").unwrap();
                    return;
                }
            }
            MemberKind::Child(child) => {
                site("Child", m, out);
                object(child, out);
                out.text.push(')');
                return;
            }
            _ => {}
        }
    }
    out.text
        .push_str("layout::Member::Direct(|script_imports| { let _ = &script_imports; ");
    m.emit(out);
    out.text.push_str(" })");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_factories_share_metadata_but_keep_individual_spans_and_imports() {
        let mut doc = parse("import QtQuick\nItem { Item {} Item {} Item {} }").unwrap();
        let MemberKind::Child(last) = &mut doc.root.members[2].kind else {
            panic!("child expected")
        };
        last.type_imports = Some(doc.imports.clone());
        last.bound_component = true;
        let output = super::super::module(doc).unwrap();
        assert_eq!(output.matches("fn object_shape_").count(), 2);
        assert_eq!(output.matches("layout::Object(object_shape_0,").count(), 3);
        assert_eq!(output.matches("layout::Object(object_shape_1,").count(), 1);
    }
}
