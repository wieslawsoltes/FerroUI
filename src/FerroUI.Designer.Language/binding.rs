//! Binding paths share schema metadata with ordinary property editing. Resolution uses the
//! explicit source, a named element, template target, or nearest x:DataType in that order.
use crate::references::{markup_arguments, Argument};
use crate::{PropertyInfo, Schema, TypeInfo};
use ferroui_designer_core::{Document, MarkupValue, NodeId, SourceSpan};
use std::collections::BTreeMap;

pub(crate) fn members(
    schema: &Schema,
    document: &Document,
    node: NodeId,
    expression: &str,
    path: &str,
) -> BTreeMap<String, PropertyInfo> {
    let arguments = markup_arguments(expression);
    let mut path = path;
    let Some(mut owner) = source_type(schema, document, node, expression, &arguments, &mut path)
    else {
        return BTreeMap::new();
    };
    if let Some((prefix, _)) = path.rsplit_once('.') {
        for segment in prefix.split('.').filter(|part| !part.is_empty()) {
            let Some(property) = schema.property(&owner.name, Some(&owner.namespace), segment)
            else {
                return BTreeMap::new();
            };
            let Some(next) = property_type(
                schema,
                &property.type_name,
                &owner.namespace,
                document,
                node,
            ) else {
                return BTreeMap::new();
            };
            owner = next;
        }
    }
    schema.properties(&owner.name, Some(&owner.namespace))
}

/// Resolve the individual path member under the caret, retaining its exact lexical source span.
pub(crate) fn at(
    schema: &Schema,
    document: &Document,
    offset: usize,
) -> Option<(PropertyInfo, SourceSpan)> {
    if !document.is_valid() {
        return None;
    }
    let node = document.ast().node(document.ast().element_at(offset)?)?;
    for attribute in &node.attributes {
        if !attribute.value_span.contains(offset) {
            continue;
        }
        let raw = &document.source()[attribute.value_span.range()];
        for argument in markup_arguments(raw) {
            if !is_binding(&argument.extension)
                || argument.name.as_deref().is_some_and(|name| name != "Path")
            {
                continue;
            }
            let span = SourceSpan::new(
                attribute.value_span.start + argument.span.start,
                attribute.value_span.start + argument.span.end,
            );
            if !span.contains(offset) {
                continue;
            }
            let lexical = &document.source()[span.range()];
            // Entity-encoded names are still indexed for rename; property navigation requires
            // normal path syntax so no decoded byte is mistaken for a source byte.
            if lexical.contains('&') || lexical.contains('\\') {
                continue;
            }
            let caret = offset - span.start;
            let start = lexical[..caret].rfind('.').map_or(0, |index| index + 1);
            let end = lexical[caret..]
                .find(['.', '[', ']', '!', '?'])
                .map_or(lexical.len(), |index| caret + index);
            let word = &lexical[start..end];
            if word.starts_with(['#', '$']) || word.is_empty() {
                return None;
            }
            let available = members(schema, document, node.id, raw, &lexical[..end]);
            return available.get(word).cloned().map(|property| {
                (
                    property,
                    SourceSpan::new(span.start + start, span.start + end),
                )
            });
        }
    }
    None
}

fn is_binding(extension: &str) -> bool {
    matches!(
        extension.rsplit(':').next().unwrap_or(extension),
        "Binding" | "CompiledBinding" | "ReflectionBinding"
    )
}
fn source_type<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
    expression: &str,
    arguments: &[Argument],
    path: &mut &str,
) -> Option<&'a TypeInfo> {
    let parameters: BTreeMap<_, _> = arguments
        .iter()
        .filter(|argument| is_binding(&argument.extension))
        .filter_map(|argument| {
            argument
                .name
                .as_deref()
                .map(|name| (name, argument.value.as_str()))
        })
        .collect();
    let mut element = parameters.get("ElementName").copied();
    if let Some(named) = path.strip_prefix('#') {
        let (name, rest) = named.split_once('.').unwrap_or((named, ""));
        element = Some(name);
        *path = rest;
    }
    if let Some(name) = element {
        let scope = crate::name_scope(document, node);
        let targets: Vec<_> = crate::name_occurrences(document)
            .into_iter()
            .filter(|occurrence| {
                occurrence.declaration && occurrence.scope == scope && occurrence.name == name
            })
            .collect();
        if targets.len() != 1 {
            return None;
        }
        let target = document.ast().node(targets[0].node)?;
        return schema.type_info(&target.name, target.namespace_uri.as_deref());
    }
    if path.starts_with("$self.") || expression.contains("{RelativeSource Self}") {
        *path = path.strip_prefix("$self.").unwrap_or(path);
        let target = document.ast().node(node)?;
        return schema.type_info(&target.name, target.namespace_uri.as_deref());
    }
    if let Some(data_type) = parameters.get("DataType") {
        return type_reference(schema, document, node, data_type);
    }
    let mut current = Some(node);
    while let Some(id) = current {
        let ancestor = document.ast().node(id)?;
        if expression.starts_with("{TemplateBinding") || expression.contains("TemplatedParent") {
            if ancestor.local_name() == "ControlTemplate" {
                return ancestor
                    .attribute("TargetType")
                    .and_then(|name| type_reference(schema, document, id, name));
            }
        } else if let Some(data_type) = ancestor.attributes.iter().find(|attribute| {
            attribute.name.rsplit(':').next() == Some("DataType")
                && attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
        }) {
            return type_reference(schema, document, id, &data_type.value);
        }
        current = ancestor.parent;
    }
    None
}
fn type_reference<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
    reference: &str,
) -> Option<&'a TypeInfo> {
    let reference = if let Ok(MarkupValue::Extension(extension)) = MarkupValue::parse(reference) {
        if extension.name.rsplit(':').next() != Some("Type") {
            return None;
        }
        match extension.arguments.first()?.value.clone() {
            MarkupValue::Literal(value) => value,
            _ => return None,
        }
    } else {
        reference.to_owned()
    };
    let namespaces = document.ast().namespaces(node);
    let namespace = reference
        .split_once(':')
        .map(|(prefix, _)| prefix)
        .unwrap_or("");
    schema.type_info(&reference, namespaces.get(namespace).map(String::as_str))
}
fn property_type<'a>(
    schema: &'a Schema,
    name: &str,
    namespace: &str,
    document: &Document,
    node: NodeId,
) -> Option<&'a TypeInfo> {
    let name = name.trim();
    let name = name
        .strip_prefix("Option<")
        .and_then(|name| name.strip_suffix('>'))
        .unwrap_or(name);
    type_reference(schema, document, node, name)
        .or_else(|| schema.type_info(name, Some(namespace)))
        .or_else(|| schema.type_info(name, None))
}
