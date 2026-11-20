use super::{is_native_binding, is_path, member_type, named_target, source_type};
use crate::references::markup_arguments_with_extensions;
use crate::{PropertyInfo, Schema};
use ferroui_designer_core::{Attribute, Document, MarkupValue, NodeId, SourceSpan};

/// Walk a statically typed binding path and keep the exact spelling of each member.
pub(crate) fn property_references(
    schema: &Schema,
    document: &Document,
    node: NodeId,
    attribute: &Attribute,
) -> Vec<(SourceSpan, PropertyInfo)> {
    let Some((mut path, span, mut owner)) = path_target(schema, document, node, attribute) else {
        return Vec::new();
    };
    let mut offset = span.end - path.len();
    let mut references = Vec::new();
    while !path.is_empty() {
        let (name, remaining) = path.split_once('.').unwrap_or((path, ""));
        if !crate::is_xaml_identifier(name) {
            break;
        }
        let Some(property) = schema.property(&owner.name, Some(&owner.namespace), name) else {
            break;
        };
        let next = member_type(schema, document, node, owner, &property);
        references.push((SourceSpan::new(offset, offset + name.len()), property));
        let Some(next) = next else { break };
        offset += name.len() + 1;
        path = remaining;
        owner = next;
    }
    references
}

fn path_target<'s, 'a>(
    schema: &'s Schema,
    document: &'a Document,
    node: NodeId,
    attribute: &Attribute,
) -> Option<(&'a str, SourceSpan, &'s crate::TypeInfo)> {
    let MarkupValue::Extension(extension) = attribute.markup().ok()? else {
        return None;
    };
    if !is_native_binding(schema, document, node, &extension.name) {
        return None;
    }
    if extension
        .arguments
        .iter()
        .any(|argument| match argument.name.as_deref() {
            Some("Source") => true,
            Some("RelativeSource") => !matches!(
                &argument.value, MarkupValue::Extension(value)
                    if value.name == "RelativeSource"
                        && value.arguments.len() == 1
                        && value.arguments[0].value == MarkupValue::Literal("Self".into())
            ),
            Some("ElementName") => !matches!(argument.value, MarkupValue::Literal(_)),
            _ => false,
        })
    {
        return None;
    }
    let raw = document.source().get(attribute.value_span.range())?;
    let arguments: Vec<_> = markup_arguments_with_extensions(raw)
        .into_iter()
        .filter(|argument| argument.depth == 0)
        .collect();
    let mut paths = arguments.iter().filter(|argument| is_path(argument));
    let argument = paths.next()?;
    if paths.next().is_some() {
        return None;
    }
    let span = SourceSpan::new(
        attribute.value_span.start + argument.span.start,
        attribute.value_span.start + argument.span.end,
    );
    let mut path = document.source().get(span.range())?;
    if path != argument.value {
        return None;
    }
    let owner = source_type(schema, document, node, raw, &arguments, &mut path, |name| {
        named_target(document, node, name)
    })?;
    Some((path, span, owner))
}
