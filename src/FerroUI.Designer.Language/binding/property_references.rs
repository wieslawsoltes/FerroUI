use super::{
    member_type,
    path_source::{element_type, named_type, PathSource},
};
use crate::{PropertyInfo, Schema};
use ferroui_designer_core::{Attribute, Document, NodeId, SourceSpan};
use ferroui_markup_syntax::binding_expression_grammar::{Node, SpannedNode};
use std::collections::BTreeMap;

/// Resolve the member operands whose owner follows from the authored path.
pub(crate) fn property_references(
    schema: &Schema,
    document: &Document,
    node: NodeId,
    attribute: &Attribute,
) -> Vec<(SourceSpan, PropertyInfo)> {
    let Some(path) = PathSource::from_attribute(schema, document, node, attribute) else {
        return Vec::new();
    };
    let namespaces = document.ast().namespaces(node);
    let type_info =
        |prefix: &str, name: &str| schema.type_info(name, Some(namespace(&namespaces, prefix)?));
    let mut owner = path.owner;
    let mut references = Vec::new();
    for SpannedNode {
        node: part,
        operand,
    } in path.nodes
    {
        let (receiver, name, attached) = match part {
            Node::PropertyName { property_name, .. } => (owner, property_name, false),
            Node::AttachedPropertyName {
                namespace,
                type_name,
                property_name,
                ..
            } => (type_info(&namespace, &type_name), property_name, true),
            Node::Name { name } => {
                owner = named_type(schema, document, node, &name);
                continue;
            }
            Node::SelfNode => {
                owner = element_type(schema, document, node);
                continue;
            }
            Node::TypeCast {
                namespace,
                type_name,
            } => {
                owner = type_info(&namespace, &type_name);
                continue;
            }
            Node::Ancestor {
                namespace,
                type_name,
                ..
            } => {
                owner = type_name
                    .and_then(|name| type_info(namespace.as_deref().unwrap_or_default(), &name));
                continue;
            }
            Node::Indexer { .. } | Node::Stream => {
                // A later explicit cast can restore a known type.
                owner = None;
                continue;
            }
            Node::EmptyExpression | Node::Not => continue,
        };
        owner = None;
        let Some(receiver) = receiver else { continue };
        let Some(property) = schema
            .property(&receiver.name, Some(&receiver.namespace), &name)
            .filter(|property| !property.event && (!attached || property.attached))
        else {
            continue;
        };
        owner = member_type(schema, document, node, receiver, &property);
        if let Some(span) = operand.and_then(|operand| path.text.source_span(operand.name)) {
            references.push((span, property));
        }
    }
    references
}

pub(crate) struct BindingTypeUse {
    pub span: SourceSpan,
    pub spelling: String,
    pub namespace: Option<String>,
}

/// Keep the lexical namespace even for a type that cannot currently resolve.
/// Rename review also checks whether the proposed name would capture that use.
pub(crate) fn path_type_references(
    schema: &Schema,
    document: &Document,
    node: NodeId,
    attribute: &Attribute,
) -> Vec<BindingTypeUse> {
    let Some(path) = PathSource::from_attribute(schema, document, node, attribute) else {
        return Vec::new();
    };
    let namespaces = document.ast().namespaces(node);
    path.nodes
        .into_iter()
        .filter_map(|part| {
            let operand = part.operand?;
            let (prefix, spelling, range) = match part.node {
                Node::TypeCast {
                    namespace,
                    type_name,
                } => (namespace, type_name, operand.name),
                Node::Ancestor {
                    namespace,
                    type_name: Some(name),
                    ..
                } => (namespace.unwrap_or_default(), name, operand.name),
                Node::AttachedPropertyName {
                    namespace,
                    type_name,
                    ..
                } => (namespace, type_name, operand.owner?),
                _ => return None,
            };
            Some(BindingTypeUse {
                span: path.text.source_span(range)?,
                namespace: namespace(&namespaces, &prefix).map(str::to_owned),
                spelling,
            })
        })
        .collect()
}

fn namespace<'a>(namespaces: &'a BTreeMap<String, String>, prefix: &str) -> Option<&'a str> {
    namespaces
        .get(prefix)
        .map(String::as_str)
        .or_else(|| prefix.is_empty().then_some(crate::FRAMEWORK_NAMESPACE))
}
