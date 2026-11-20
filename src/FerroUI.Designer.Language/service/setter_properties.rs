use super::style_selectors;
use crate::{FRAMEWORK_NAMESPACE, PropertyInfo, Schema, TypeInfo, binding};
use ferroui_designer_core::{Attribute, Document, Node, NodeId};

pub(super) fn setter_property(
    schema: &Schema,
    document: &Document,
    node: &Node,
    attribute: &Attribute,
) -> Option<PropertyInfo> {
    if attribute.name != "Property"
        || !native(schema, node, "Setter")
        || document.source().get(attribute.value_span.range())? != attribute.value
    {
        return None;
    }
    if let Some((owner, member)) = attribute.value.rsplit_once('.') {
        let owner = binding::type_reference(schema, document, node.id, owner)?;
        return schema.property(&owner.name, Some(&owner.namespace), member);
    }
    let owner = setter_target(schema, document, node.parent?)?;
    schema.property(&owner.name, Some(&owner.namespace), &attribute.value)
}

pub(super) fn setter_value_property(
    schema: &Schema,
    document: &Document,
    owner: &Node,
    member: &str,
) -> Option<PropertyInfo> {
    if member.rsplit('.').next() != Some("Value") {
        return None;
    }
    setter_property(schema, document, owner, owner.attribute_info("Property")?)
}

fn setter_target<'a>(
    schema: &'a Schema,
    document: &Document,
    parent: NodeId,
) -> Option<&'a TypeInfo> {
    let mut parent = document.ast().node(parent)?;
    if parent.name.contains('.') {
        parent = document.ast().node(parent.parent?)?;
    }
    if style_selectors::is_style(schema, parent) {
        return style_selectors::target_type(schema, document, parent);
    }
    if !native(schema, parent, "ControlTheme") {
        return None;
    }
    let target = parent
        .attributes
        .iter()
        .find(|attribute| attribute.name == "TargetType")?;
    binding::type_reference(schema, document, parent.id, &target.value)
}

fn native(schema: &Schema, node: &Node, name: &str) -> bool {
    schema
        .type_info(&node.name, node.namespace_uri.as_deref())
        .is_some_and(|info| info.namespace == FRAMEWORK_NAMESPACE && info.name == name)
}
