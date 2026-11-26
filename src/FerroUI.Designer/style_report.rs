//! The same authored selector, setter and source-range model for both preview hosts.
use crate::{PropertyEditorKind, PropertyEditorMetadata};
use ferroui_designer_core::{Document, MarkupValue};
use ferroui_designer_language::LineIndex;
use ferroui_designer_tools::{
    component_property_attribute, component_property_elements, component_styles::authored_setters,
    ComponentEntry, ComponentKind,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub(crate) fn setter_field(metadata: &PropertyEditorMetadata, type_name: &str) -> Value {
    let (kind, options) = match &metadata.kind {
        PropertyEditorKind::Boolean => ("boolean", vec!["True".into(), "False".into()]),
        PropertyEditorKind::Enumeration {
            values,
            flags: false,
        } => ("choice", values.clone()),
        PropertyEditorKind::Color | PropertyEditorKind::Brush => ("color", vec![]),
        PropertyEditorKind::Number => ("number", vec![]),
        _ => ("text", vec![]),
    };
    json!({"kind":kind,"options":options,"type_name":type_name,"resources":true})
}

pub(crate) fn definition(
    document: &Document,
    entry: &ComponentEntry,
    available: impl IntoIterator<Item = String>,
    field: impl Fn(&str) -> (Value, bool),
) -> Result<Value, String> {
    let node = entry.node;
    let declaration = document.ast().node(node).ok_or("The style changed")?;
    let index = LineIndex::new(document.source());
    let (identity, member) = match entry.kind {
        ComponentKind::Style => ("Selector", "selector"),
        ComponentKind::ControlTheme => ("BasedOn", "base"),
        _ => return Err("Choose one authored Style or ControlTheme".into()),
    };
    let attribute = component_property_attribute(document, node, identity)
        .map_err(|error| error.to_string())?;
    let editable = component_property_elements(document, node, identity)
        .map_err(|error| error.to_string())?
        .is_empty()
        && (member != "selector"
            || attribute.is_none_or(|value| matches!(value.markup(), Ok(MarkupValue::Literal(_)))));
    let mut existing = BTreeSet::new();
    let setters = authored_setters(document, node).map_err(|error| error.to_string())?
        .into_iter().map(|setter| {
            existing.insert(setter.property.clone());
            let (field, editable) = field(&setter.property);
            let expression = setter.value.as_ref().is_some_and(|value| matches!(MarkupValue::parse(value), Ok(MarkupValue::Extension(_))));
            Ok(json!({"node":setter.node,"property":setter.property,"value":setter.value,"object":setter.object.is_some(),
                "range":index.range(setter.span).map_err(|error| error.to_string())?, "field":field,
                "editable":editable && !expression && setter.object.is_none(), "expression":expression}))
        }).collect::<Result<Vec<_>, String>>()?;
    let available: BTreeSet<_> = available
        .into_iter()
        .filter(|name| !existing.contains(name))
        .collect();
    let available = available
        .into_iter()
        .map(|name| json!({"property":name,"field":field(&name).0}))
        .collect::<Vec<_>>();
    Ok(json!({
        "node":node,"kind":entry.kind,"name":entry.display_name,"target":entry.target_type,
        "range":index.range(declaration.name_span).map_err(|error| error.to_string())?,
        "identity":{"kind":member,"value":attribute.map(|attribute| &attribute.value),"editable":editable},
        "can_remove":entry.kind == ComponentKind::Style && entry.key.is_none(),
        "setters":setters, "available":available,
    }))
}
