use crate::{PropertyInfo, Schema};
use ferroui_designer_core::{Document, Edit, Node, NodeKind, SourceSpan};
use std::collections::BTreeMap;

/// Routed events inherited by a source element, together with authored qualified events.
pub fn routed_events(
    schema: &Schema,
    document: &Document,
    node: &Node,
) -> BTreeMap<String, PropertyInfo> {
    if node.kind != NodeKind::Element {
        return BTreeMap::new();
    }
    let mut result = schema.properties(node.local_name(), node.namespace_uri.as_deref());
    result.retain(|_, info| info.event);
    for attribute in node
        .attributes
        .iter()
        .filter(|attribute| !attribute.is_namespace())
    {
        if let Some(info) = schema
            .attribute_property(document, node, attribute)
            .filter(|info| info.event)
        {
            result.insert(attribute.name.clone(), info);
        }
    }
    result
}

pub fn event_object_span(document: &Document, node: &Node, name: &str) -> Option<SourceSpan> {
    node.children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .find(|child| {
            child.kind == NodeKind::PropertyElement
                && child.local_name().rsplit('.').next() == Some(name)
        })
        .map(|child| child.name_span)
}

pub fn event_handler_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

pub fn plan_event_handler(
    schema: &Schema,
    document: &Document,
    node: &Node,
    name: &str,
    value: Option<&str>,
) -> Result<Edit, String> {
    if !routed_events(schema, document, node).contains_key(name) {
        return Err(format!("{name} is unavailable for this control"));
    }
    if node
        .attribute(name)
        .is_some_and(|current| !event_handler_name(current))
        || event_object_span(document, node, name).is_some()
    {
        return Err("Edit the existing event declaration in source".into());
    }
    match value {
        Some(value) if event_handler_name(value) => Ok(Edit::SetAttribute {
            node: node.id,
            name: name.into(),
            value: value.into(),
        }),
        None => Ok(Edit::RemoveAttribute {
            node: node.id,
            name: name.into(),
        }),
        _ => Err("Enter the name of the handler declared in code-behind".into()),
    }
}
