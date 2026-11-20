//! Resource operands with the exact source span and spelling used by the editor.
use super::{native_resource_type, theme_provider, Occurrence, OccurrenceSpelling, SymbolKind};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeKind, SourceSpan, XmlText};
use ferroui_designer_tools::{component_property_attribute, component_property_elements};

pub(super) fn collect(document: &Document, occurrences: &mut Vec<Occurrence>) {
    for node in document.ast().elements() {
        let namespaces = document.ast().namespaces(node.id);
        if resource_extension(node.local_name(), node.namespace_uri.as_deref()) {
            if let Some((name, span, spelling)) = object_key(document, node) {
                occurrences.push(occurrence(node, name, span, spelling, false));
            }
        }
        for attribute in &node.attributes {
            if attribute.name.rsplit(':').next() == Some("Key")
                && attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
                && !theme_provider(document, node)
            {
                if let Ok(MarkupValue::Literal(key)) = attribute.markup() {
                    occurrences.push(occurrence(
                        node,
                        key,
                        attribute.value_span,
                        OccurrenceSpelling::Attribute,
                        true,
                    ));
                }
            }
            if attribute.is_namespace()
                || ferroui_designer_core::design_fixtures::is_fixtures(attribute)
            {
                continue;
            }
            let raw = &document.source()[attribute.value_span.range()];
            for argument in crate::references::markup_arguments(raw) {
                let (prefix, extension) = argument
                    .extension
                    .split_once(':')
                    .unwrap_or(("", &argument.extension));
                let namespace = namespaces.get(prefix).map(String::as_str);
                if (!prefix.is_empty() && namespace.is_none())
                    || !resource_extension(extension, namespace)
                    || argument
                        .name
                        .as_deref()
                        .is_some_and(|name| name != "ResourceKey")
                {
                    continue;
                }
                let span = SourceSpan::new(
                    attribute.value_span.start + argument.span.start,
                    attribute.value_span.start + argument.span.end,
                );
                // Quoted braces still introduce markup unless the author escapes them.
                let Some(raw) = XmlText::attribute(document.source(), span) else {
                    continue;
                };
                let key = if raw.value().starts_with("{}") {
                    argument.value.strip_prefix("{}").map(str::to_owned)
                } else {
                    (!raw.value().starts_with('{')).then_some(argument.value)
                };
                if let Some(key) = key {
                    occurrences.push(occurrence(
                        node,
                        key,
                        span,
                        OccurrenceSpelling::MarkupArgument(argument.quote),
                        false,
                    ));
                }
            }
        }
    }
}

fn occurrence(
    node: &Node,
    name: String,
    span: SourceSpan,
    spelling: OccurrenceSpelling,
    declaration: bool,
) -> Occurrence {
    Occurrence {
        name,
        span,
        kind: SymbolKind::Resource,
        declaration,
        node: node.id,
        spelling,
    }
}

fn resource_extension(name: &str, namespace: Option<&str>) -> bool {
    let runtime = match name {
        "StaticResource" | "StaticResourceExtension" => "StaticResourceExtension",
        "DynamicResource" | "DynamicResourceExtension" => "DynamicResourceExtension",
        _ => return false,
    };
    namespace.is_none_or(|namespace| namespace == crate::schema::FRAMEWORK_NAMESPACE)
        || native_resource_type(runtime, namespace)
}

fn object_key(
    document: &Document,
    node: &Node,
) -> Option<(String, SourceSpan, OccurrenceSpelling)> {
    if let Some(attribute) = component_property_attribute(document, node.id, "ResourceKey").ok()? {
        let MarkupValue::Literal(key) = attribute.markup().ok()? else {
            return None;
        };
        return Some((key, attribute.value_span, OccurrenceSpelling::Attribute));
    }
    let properties = component_property_elements(document, node.id, "ResourceKey").ok()?;
    let [property] = properties.as_slice() else {
        return None;
    };
    let property = document.ast().node(*property)?;
    let mut parts = property
        .children
        .iter()
        .filter_map(|child| document.ast().node(*child))
        .filter(|child| {
            child.kind != NodeKind::Comment
                && !child
                    .text
                    .as_deref()
                    .is_some_and(|text| text.trim().is_empty())
        });
    let part = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    if part.kind == NodeKind::Text {
        let text = XmlText::text_node(document.source(), part.span)?;
        let first = text.value().trim_start();
        let key = first.trim_end();
        let start = text.value().len() - first.len();
        return Some((
            key.into(),
            text.source_span(start..start + key.len())?,
            OccurrenceSpelling::Text,
        ));
    }
    if part.kind == NodeKind::CData {
        let raw = &document.source()[part.span.start + 9..part.span.end - 3];
        let key = raw.trim();
        let start = part.span.start + 9 + raw.len() - raw.trim_start().len();
        return Some((
            key.into(),
            SourceSpan::new(start, start + key.len()),
            OccurrenceSpelling::CData,
        ));
    }
    None
}
