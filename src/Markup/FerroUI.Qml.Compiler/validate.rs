//! Source-level invariants shared by interpreted and Cargo-compiled components.
use crate::{Binding, Diagnostic, Document, Expr, Member, MemberKind, Object, SourceSpan};
use std::collections::HashSet;

/// Validate declarations and assignments before creating any runtime objects.
pub fn validate(document: &Document, source: &str) -> Result<(), Diagnostic> {
    component(&document.root, source)
}

fn component(object: &Object, source: &str) -> Result<(), Diagnostic> {
    validate_object(object, &mut HashSet::new(), source)
}

fn validate_object<'a>(
    object: &'a Object,
    ids: &mut HashSet<&'a str>,
    source: &str,
) -> Result<(), Diagnostic> {
    let mut names = HashSet::new();
    let mut bindings = HashSet::new();
    let mut has_id = false;
    let mut has_default = false;
    for member in &object.members {
        let span = member.span;
        let declaration = match &member.kind {
            MemberKind::Property { name, .. }
            | MemberKind::Signal { name, .. }
            | MemberKind::Function { name, .. }
            | MemberKind::Component { name, .. }
            | MemberKind::Enum { name, .. } => Some(name),
            _ => None,
        };
        if let Some(name) = declaration {
            if name == "id" || !names.insert(name.as_str()) {
                return Err(Diagnostic::new(
                    source,
                    span,
                    format!("duplicate or reserved member '{name}'"),
                ));
            }
        }
        match &member.kind {
            MemberKind::Id(name) => {
                if has_id || !ids.insert(name) {
                    return Err(Diagnostic::new(
                        source,
                        span,
                        format!("duplicate id '{name}'"),
                    ));
                }
                has_id = true;
            }
            MemberKind::Property {
                name,
                ty,
                modifiers,
                value,
            } => {
                if modifiers.default && has_default {
                    return Err(Diagnostic::new(source, span, "multiple default properties"));
                }
                has_default |= modifiers.default;
                if modifiers.required && value.is_some() {
                    return Err(Diagnostic::new(
                        source,
                        span,
                        "a required property cannot declare an initializer",
                    ));
                }
                if modifiers.readonly && value.is_none() {
                    return Err(Diagnostic::new(
                        source,
                        span,
                        "a readonly property requires an initializer",
                    ));
                }
                if ty == "alias"
                    && !matches!(value, Some(Binding::Expr(e)) if alias_depth(e).is_some_and(|d| d <= 2))
                {
                    return Err(Diagnostic::new(
                        source,
                        span,
                        "an alias must refer to an id or its property",
                    ));
                }
                if let Some(value) = value {
                    assign(&mut bindings, name.clone(), source, span)?;
                    binding(value, ids, source)?;
                }
            }
            MemberKind::Binding { name, value } => {
                assign(&mut bindings, name.clone(), source, span)?;
                if object.type_name.rsplit('.').next() == Some("Repeater") && name == "delegate" {
                    binding(value, &mut HashSet::new(), source)?;
                } else {
                    binding(value, ids, source)?;
                }
            }
            MemberKind::Group { name, members } => {
                group(name, members, &mut bindings, ids, source)?
            }
            MemberKind::Handler { name, .. } => assign(&mut bindings, name.clone(), source, span)?,
            MemberKind::Child(child)
                if matches!(
                    object.type_name.rsplit('.').next(),
                    Some("Component" | "Repeater")
                ) =>
            {
                validate_object(child, &mut HashSet::new(), source)?;
            }
            MemberKind::Child(child) | MemberKind::OnProperty { object: child, .. } => {
                validate_object(child, ids, source)?
            }
            MemberKind::Component { object, .. } => component(object, source)?,
            _ => {}
        }
    }
    Ok(())
}

fn assign(
    bindings: &mut HashSet<String>,
    name: String,
    source: &str,
    span: SourceSpan,
) -> Result<(), Diagnostic> {
    if bindings.contains(&name) {
        return Err(Diagnostic::new(
            source,
            span,
            format!("duplicate binding for '{name}'"),
        ));
    }
    bindings.insert(name);
    Ok(())
}

fn group<'a>(
    prefix: &str,
    members: &'a [Member],
    bindings: &mut HashSet<String>,
    ids: &mut HashSet<&'a str>,
    source: &str,
) -> Result<(), Diagnostic> {
    for member in members {
        match &member.kind {
            MemberKind::Binding { name, value } => {
                assign(bindings, format!("{prefix}.{name}"), source, member.span)?;
                binding(value, ids, source)?;
            }
            MemberKind::Group { name, members } => {
                group(&format!("{prefix}.{name}"), members, bindings, ids, source)?
            }
            _ => {
                return Err(Diagnostic::new(
                    source,
                    member.span,
                    "expected a grouped property assignment",
                ))
            }
        }
    }
    Ok(())
}

fn binding<'a>(
    value: &'a Binding,
    ids: &mut HashSet<&'a str>,
    source: &str,
) -> Result<(), Diagnostic> {
    match value {
        Binding::Object(object) => validate_object(object, ids, source)?,
        Binding::Objects(objects) => {
            for object in objects {
                validate_object(object, ids, source)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn alias_depth(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::Name(_) => Some(0),
        Expr::Member(base, _) => Some(1 + alias_depth(base)?),
        _ => None,
    }
}
