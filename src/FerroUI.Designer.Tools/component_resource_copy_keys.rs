//! Rewrite resource key operands in a copied fragment, retaining other authored properties.
use super::{escaped_literal, extension_kind, resource_argument, serialize};
use crate::components::{
    component_property_attribute, component_property_elements, escape, framework,
};
use crate::{require, Result, ToolError};
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeKind};

pub(in crate::components) fn rewrite_keys(
    document: &Document,
    node: &Node,
    mut replacement: impl FnMut(&str) -> Result<Option<String>>,
) -> Result<Vec<Edit>> {
    let mut edits = Vec::new();
    if framework(node)
        && matches!(
            node.local_name(),
            "StaticResource"
                | "DynamicResource"
                | "StaticResourceExtension"
                | "DynamicResourceExtension"
        )
    {
        if let Some(attribute) = component_property_attribute(document, node.id, "ResourceKey")? {
            let key = attribute.markup().map_err(ToolError)?;
            if let Some(value) = replacement(literal(&key)?)? {
                edits.push(Edit::SetAttribute {
                    node: node.id,
                    name: attribute.name.clone(),
                    value: escaped_literal(&value),
                });
            }
        } else {
            let properties = component_property_elements(document, node.id, "ResourceKey")?;
            let mut text = properties
                .iter()
                .filter_map(|id| document.ast().node(*id))
                .flat_map(|property| &property.children)
                .filter_map(|id| document.ast().node(*id))
                .filter(|child| child.kind != NodeKind::Comment);
            let content = text
                .next()
                .filter(|child| child.kind == NodeKind::Text)
                .ok_or_else(unverified_key)?;
            require(
                properties.len() == 1 && text.next().is_none(),
                unverified_key().0,
            )?;
            if let Some(value) = replacement(content.text.as_deref().unwrap_or_default().trim())? {
                edits.push(Edit::Text {
                    span: content.span,
                    replacement: escape(&value),
                });
            }
        }
    }
    for attribute in &node.attributes {
        if attribute.is_namespace()
            || !attribute.value.starts_with('{')
            || ferroui_designer_core::design_fixtures::is_fixtures(attribute)
        {
            continue;
        }
        let mut value = attribute.markup().map_err(ToolError)?;
        if rewrite_markup(document, node, &mut value, &mut replacement)? {
            edits.push(Edit::SetAttribute {
                node: node.id,
                name: attribute.name.clone(),
                value: serialize(&value),
            });
        }
    }
    Ok(edits)
}

fn rewrite_markup(
    document: &Document,
    node: &Node,
    value: &mut MarkupValue,
    replacement: &mut impl FnMut(&str) -> Result<Option<String>>,
) -> Result<bool> {
    let MarkupValue::Extension(extension) = value else {
        return Ok(false);
    };
    let mut changed = false;
    if extension_kind(document, node.id, &extension.name).is_some() {
        let index = resource_argument(extension).ok_or_else(unverified_key)?;
        let argument = &mut extension.arguments[index].value;
        if let Some(key) = replacement(literal(argument)?)? {
            *argument = MarkupValue::Literal(key);
            changed = true;
        }
    }
    for argument in &mut extension.arguments {
        changed |= rewrite_markup(document, node, &mut argument.value, replacement)?;
    }
    Ok(changed)
}

fn literal(value: &MarkupValue) -> Result<&str> {
    match value {
        MarkupValue::Literal(key) => Ok(key),
        MarkupValue::Extension(_) => Err(unverified_key()),
    }
}

fn unverified_key() -> ToolError {
    ToolError("A copied resource reference needs an authored string key.".into())
}
