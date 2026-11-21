//! Binding paths share schema metadata with ordinary property editing. Resolution uses the
//! explicit source, a named element, template target, or nearest x:DataType in that order.
use crate::references::{markup_arguments, markup_arguments_with_extensions, Argument};
use crate::{LanguageService, Position, PropertyInfo, Range, Result, Schema, TypeInfo};
use ferroui_designer_core::{Document, MarkupValue, Node, NodeId, SourceSpan};
use std::collections::BTreeMap;

mod fixes;
mod path_source;
mod property_references;
pub(crate) use fixes::case_mismatch;
pub(crate) use path_source::is_relative_source;
pub(crate) use property_references::{path_type_references, property_references};

impl LanguageService {
    /// Locate the ends of typed binding paths visible in the requested source range.
    pub fn binding_type_hints(&self, uri: &str, range: Range) -> Result<Vec<(Position, String)>> {
        let index = self.index(uri)?;
        let visible = index.span(range)?;
        let Some(document) = self.document(uri).filter(|document| document.is_valid()) else {
            return Ok(Vec::new());
        };
        let schema = self.schema_for(uri);
        let mut hints = Vec::new();
        for node in document.ast().elements() {
            for attribute in &node.attributes {
                if attribute.value_span.end < visible.start
                    || attribute.value_span.start > visible.end
                {
                    continue;
                }
                let raw = &document.source()[attribute.value_span.range()];
                for argument in markup_arguments(raw) {
                    if !is_path(&argument) {
                        continue;
                    }
                    let end = attribute.value_span.start + argument.span.end;
                    let name = argument.value.rsplit('.').next().unwrap_or_default();
                    if end < visible.start || end > visible.end || !crate::is_xaml_identifier(name)
                    {
                        continue;
                    }
                    let mut available = members(schema, document, node.id, raw, &argument.value);
                    if let Some(property) = available
                        .remove(name)
                        .filter(|property| !property.type_name.is_empty())
                    {
                        hints.push((index.position(end)?, property.type_name));
                    }
                }
            }
        }
        Ok(hints)
    }
}

pub(crate) fn members(
    schema: &Schema,
    document: &Document,
    node: NodeId,
    expression: &str,
    path: &str,
) -> BTreeMap<String, PropertyInfo> {
    let arguments = markup_arguments_with_extensions(expression);
    let mut path = path;
    let Some(mut owner) = source_type(
        schema,
        document,
        node,
        expression,
        &arguments,
        &mut path,
        |name| named_target(document, node, name),
    ) else {
        return BTreeMap::new();
    };
    if let Some((prefix, _)) = path.rsplit_once('.') {
        for segment in prefix.split('.').filter(|part| !part.is_empty()) {
            let Some(property) = schema.property(&owner.name, Some(&owner.namespace), segment)
            else {
                return BTreeMap::new();
            };
            let Some(next) = member_type(schema, document, node, owner, &property) else {
                return BTreeMap::new();
            };
            owner = next;
        }
    }
    schema.properties(&owner.name, Some(&owner.namespace))
}

fn named_target(document: &Document, anchor: NodeId, name: &str) -> Option<NodeId> {
    let scope = crate::name_scope(document, anchor);
    let mut targets = crate::name_occurrences(document)
        .into_iter()
        .filter(|occurrence| {
            occurrence.declaration && occurrence.scope == scope && occurrence.name == name
        });
    let node = targets.next()?.node;
    targets.next().is_none().then_some(node)
}

fn member_type<'a>(
    schema: &'a Schema, document: &Document, node: NodeId, owner: &TypeInfo, property: &PropertyInfo,
) -> Option<&'a TypeInfo> {
    if let Some(type_) = &property.model_type {
        schema.type_info(&type_.name, Some(&type_.namespace))
    } else {
        property_type(schema, &property.type_name, &owner.namespace, document, node)
    }
}

/// A type named by an authored XAML type value, including a local binding override.
pub(crate) fn type_at<'a>(
    schema: &'a Schema,
    document: &Document,
    node: &Node,
    offset: usize,
) -> Option<&'a TypeInfo> {
    let attribute = node
        .attributes
        .iter()
        .find(|attribute| attribute.value_span.contains(offset))?;
    if let Some(operand) = path_type_references(schema, document, node.id, attribute)
        .into_iter()
        .find(|operand| operand.span.contains(offset))
    {
        return schema.type_info(&operand.spelling, Some(operand.namespace.as_deref()?));
    }
    let directive = attribute.namespace_uri.as_deref()
        == Some("http://schemas.microsoft.com/winfx/2006/xaml")
        && attribute.name.rsplit(':').next() == Some("DataType");
    let native_datatype = attribute.name == "DataType"
        && schema
            .type_info(&node.name, node.namespace_uri.as_deref())
            .is_some_and(|info| {
                info.namespace == crate::schema::FRAMEWORK_NAMESPACE
                    && (info.name == "DataTemplate" || is_binding(&info.name))
            });
    if directive || native_datatype {
        return type_reference(schema, document, node.id, &attribute.value);
    }
    if attribute.value.trim_start().starts_with('{') {
        if let Some(type_) = type_reference(schema, document, node.id, &attribute.value) {
            return Some(type_);
        }
    }
    let raw = &document.source()[attribute.value_span.range()];
    let local_offset = offset.checked_sub(attribute.value_span.start)?;
    path_source::type_parameter_at(schema, document, node.id, raw, local_offset)
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
        if let Some((span, property)) = property_references(schema, document, node.id, attribute)
            .into_iter()
            .find(|(span, _)| span.contains(offset))
        {
            return Some((property, span));
        }
        let raw = &document.source()[attribute.value_span.range()];
        for argument in markup_arguments(raw) {
            if argument.depth == 0
                || !is_path(&argument)
                || !is_native_binding(schema, document, node.id, &argument.extension)
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

fn is_path(argument: &Argument) -> bool {
    is_binding(&argument.extension) && argument.name.as_deref().is_none_or(|name| name == "Path")
}

fn is_binding(extension: &str) -> bool {
    let extension = extension.rsplit(':').next().unwrap_or(extension);
    matches!(
        extension.strip_suffix("Extension").unwrap_or(extension),
        "Binding" | "CompiledBinding" | "ReflectionBinding"
    )
}

pub(crate) fn is_native_binding(schema: &Schema, document: &Document, node: NodeId, extension: &str) -> bool {
    let (prefix, _) = extension.split_once(':').unwrap_or(("", extension));
    let namespaces = document.ast().namespaces(node);
    schema
        .extension(extension, namespaces.get(prefix).map(String::as_str))
        .is_some_and(|info| {
            matches!(
                info.name.as_str(),
                "Binding" | "CompiledBinding" | "ReflectionBinding"
            )
        })
}

fn source_type<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
    expression: &str,
    arguments: &[Argument],
    path: &mut &str,
    named_element: impl FnOnce(&str) -> Option<NodeId>,
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
        let target = document.ast().node(named_element(name)?)?;
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
    if expression.starts_with("{TemplateBinding") || expression.contains("TemplatedParent") {
        return template_target_type(schema, document, node);
    }
    let mut current = Some(node);
    while let Some(id) = current {
        let ancestor = document.ast().node(id)?;
        let native_type = schema
            .type_info(&ancestor.name, ancestor.namespace_uri.as_deref())
            .filter(|info| info.namespace == crate::schema::FRAMEWORK_NAMESPACE)
            .map(|info| info.name.as_str());
        if let Some(data_type) = ancestor.attributes.iter().find(|attribute| {
            attribute.name.rsplit(':').next() == Some("DataType")
                && attribute.namespace_uri.as_deref()
                    == Some("http://schemas.microsoft.com/winfx/2006/xaml")
        }) {
            return type_reference(schema, document, id, &data_type.value);
        } else if let Some(context) = ancestor
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .find(|child| {
                child.namespace_uri.as_deref() == Some(crate::design_support::DESIGN_NAMESPACE)
                    && crate::design_support::property_element(schema, document, child)
                        .is_some_and(|property| property.name == "DataContext")
                    || child.name == "Design.DataContext"
                        && child
                            .namespace_uri
                            .as_deref()
                            .is_none_or(|namespace| namespace == crate::schema::FRAMEWORK_NAMESPACE)
            })
        {
            // A concrete design-time data object gives binding IntelliSense a real source
            // type. Do not infer arbitrary object/markup values or leak an outer context
            // through an unresolved explicit design context.
            let mut objects = context
                .children
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .filter(|node| node.kind == ferroui_designer_core::NodeKind::Element);
            let object = objects.next()?;
            if objects.next().is_some() {
                return None;
            }
            return schema.type_info(&object.name, object.namespace_uri.as_deref());
        } else if matches!(native_type, Some("DataTemplate" | "TreeDataTemplate")) {
            if let Some(data_type) = ancestor.attributes.iter().find(|attribute| {
                schema
                    .attribute_property(document, ancestor, attribute)
                    .is_some_and(|property| property.name == "DataType")
            }) {
                return type_reference(schema, document, id, &data_type.value);
            }
        }
        current = ancestor.parent;
    }
    None
}

fn template_target_type<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
) -> Option<&'a TypeInfo> {
    let mut current = Some(node);
    while let Some(id) = current {
        let ancestor = document.ast().node(id)?;
        if schema
            .type_info(&ancestor.name, ancestor.namespace_uri.as_deref())
            .is_some_and(|info| {
                info.namespace == crate::schema::FRAMEWORK_NAMESPACE && info.name == "ControlTemplate"
            })
        {
            let target = ancestor.attributes.iter().find(|attribute| {
                schema.attribute_property(document, ancestor, attribute)
                    .is_some_and(|property| property.name == "TargetType")
            })?;
            return type_reference(schema, document, id, &target.value);
        }
        current = ancestor.parent;
    }
    None
}

pub(crate) fn type_reference<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
    reference: &str,
) -> Option<&'a TypeInfo> {
    if let Ok(extension @ MarkupValue::Extension(_)) = MarkupValue::parse(reference) {
        return type_value(schema, document, node, &extension);
    }
    xml_type_name(schema, document, node, reference)
}

fn type_value<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
    value: &MarkupValue,
) -> Option<&'a TypeInfo> {
    let reference = match value {
        MarkupValue::Literal(reference) => reference,
        MarkupValue::Extension(extension) => {
            let (prefix, name) = extension
                .name
                .split_once(':')
                .unwrap_or(("", &extension.name));
            if name != "Type"
                || document.ast().namespaces(node).get(prefix).map(String::as_str)
                    != Some("http://schemas.microsoft.com/winfx/2006/xaml")
            {
                return None;
            }
            let [argument] = extension.arguments.as_slice() else {
                return None;
            };
            if argument.name.as_deref().is_some_and(|name| name != "TypeName") {
                return None;
            }
            let MarkupValue::Literal(reference) = &argument.value else {
                return None;
            };
            reference
        }
    };
    xml_type_name(schema, document, node, reference)
}

fn xml_type_name<'a>(
    schema: &'a Schema,
    document: &Document,
    node: NodeId,
    reference: &str,
) -> Option<&'a TypeInfo> {
    let namespaces = document.ast().namespaces(node);
    let reference = reference.trim();
    let namespace = match reference.split_once(':') {
        Some((prefix, _)) => Some(namespaces.get(prefix)?.as_str()),
        None => namespaces.get("").map(String::as_str),
    };
    schema.type_info(reference, namespace)
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
