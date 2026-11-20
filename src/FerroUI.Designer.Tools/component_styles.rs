//! Authored style declarations and source-preserving style composition.
use crate::components::{
    ComponentAction, authoring, component_property_attribute, component_property_object,
    component_resource_expression, element, escape, framework, key_attribute, literal_key,
    plan_component_action, property_elements, references, resolve_component_resource,
    valid_document, validated,
};
use crate::{Result, ToolError, require};
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, SourceSpan};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AuthoredSetter {
    pub node: NodeId,
    pub property: String,
    pub value: Option<String>,
    pub object: Option<String>,
    pub span: SourceSpan,
}

pub fn authored_setters(document: &Document, style: NodeId) -> Result<Vec<AuthoredSetter>> {
    valid_document(document)?;
    authoring::setters(document, style)?
        .1
        .into_iter()
        .filter_map(
            |id| match component_property_attribute(document, id, "Property") {
                Ok(Some(property)) => Some(Ok((id, property))),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .map(|entry| {
            let (node, property) = entry?;
            let value = component_property_attribute(document, node, "Value")?;
            Ok(AuthoredSetter {
                node,
                property: property.value.to_owned(),
                value: value.map(|attribute| attribute.value.to_owned()),
                object: component_property_object(document, node, "Value")?,
                span: value.map_or(element(document, node)?.name_span, |attribute| {
                    attribute.value_span
                }),
            })
        })
        .collect()
}

pub enum StyleChange<'a> {
    Create { owner: NodeId, selector: &'a str },
    SetSelector { node: NodeId, selector: &'a str },
    SetBase { theme: NodeId, base: Option<NodeId> },
    Remove { node: NodeId },
}

/// Source placement and scope checks are portable. The host validates native selector
/// syntax and registered target types before applying this plan.
pub fn plan_style_change(document: &Document, change: StyleChange<'_>) -> Result<Vec<Edit>> {
    valid_document(document)?;
    match change {
        StyleChange::Create { owner, selector } => create_style(document, owner, selector),
        StyleChange::SetSelector { node, selector } => {
            require(
                is_style(element(document, node)?),
                "Select an authored Style",
            )?;
            require(
                component_property_attribute(document, node, "Selector")?
                    .is_none_or(|attr| matches!(attr.markup(), Ok(MarkupValue::Literal(_))))
                    && property_elements(document, element(document, node)?, "Selector")?
                        .is_empty(),
                "Open the existing Selector expression in source",
            )?;
            plan_component_action(
                document,
                &ComponentAction::SetProperty {
                    node,
                    property: "Selector".into(),
                    value: Some(selector.into()),
                },
            )
        }
        StyleChange::SetBase { theme, base } => set_base(document, theme, base),
        StyleChange::Remove { node } => {
            let style = element(document, node)?;
            require(
                is_style(style) && key_attribute(style).is_none(),
                "Use the resource library to remove a keyed style",
            )?;
            validated(document, vec![Edit::RemoveNode { node }])
        }
    }
}

fn create_style(document: &Document, owner: NodeId, selector: &str) -> Result<Vec<Edit>> {
    let node = element(document, owner)?;
    let (prefix, _, namespaces) = authoring::creation_namespaces(document, owner);
    let fragment = format!(
        "<{prefix}:Style{namespaces} Selector=\"{}\" />",
        escape(selector)
    );
    let attributes = component_property_attribute(document, owner, "Styles")?;
    let styles = property_elements(document, node, "Styles")?;
    require(
        attributes.is_none() && styles.len() <= 1,
        "Open the existing Styles assignment in source",
    )?;
    let (parent, source) = if let Some(styles) = styles.first() {
        // Property elements can hold an explicit Styles collection object.
        let children = document.ast().element_children(styles.id);
        let container = match children.as_slice() {
            [child]
                if document
                    .ast()
                    .node(*child)
                    .is_some_and(|child| framework(child) && child.local_name() == "Styles") =>
            {
                *child
            }
            _ => styles.id,
        };
        (container, fragment)
    } else if is_style(node) || is_theme(node) || (framework(node) && node.local_name() == "Styles")
    {
        (owner, fragment)
    } else {
        let name = format!("{}.Styles", node.name);
        (owner, format!("<{name}>\n{fragment}\n</{name}>"))
    };
    validated(
        document,
        vec![Edit::InsertChild {
            parent,
            index: document.ast().element_children(parent).len(),
            source,
        }],
    )
}

fn set_base(document: &Document, theme: NodeId, base: Option<NodeId>) -> Result<Vec<Edit>> {
    require(
        is_theme(element(document, theme)?),
        "Select an authored ControlTheme",
    )?;
    require(
        property_elements(document, element(document, theme)?, "BasedOn")?.is_empty(),
        "Edit the existing BasedOn object in source",
    )?;
    let value = base
        .map(|base| {
            require(
                is_theme(element(document, base)?),
                "Choose an authored ControlTheme as the base",
            )?;
            let key = literal_key(element(document, base)?).ok_or_else(|| {
                ToolError("A base theme needs an explicit string resource key".into())
            })?;
            require(
                resolve_component_resource(document, theme, &key) == Some(base),
                "The base theme does not resolve unambiguously in this definition's scope",
            )?;
            let mut visited = BTreeSet::from([theme]);
            let mut current = Some(base);
            while let Some(id) = current {
                require(
                    visited.insert(id),
                    "BasedOn would create a control theme cycle",
                )?;
                let next = base_theme(document, id)?;
                require(
                    next.is_some()
                        || (component_property_attribute(document, id, "BasedOn")?.is_none()
                            && property_elements(document, element(document, id)?, "BasedOn")?
                                .is_empty()),
                    "The base theme has unresolved inheritance; edit that chain in source first",
                )?;
                current = next.filter(|next| element(document, *next).is_ok_and(is_theme));
                require(
                    current == next,
                    "The base chain references an object outside the control themes",
                )?;
            }
            component_resource_expression(document, theme, &key, false)
        })
        .transpose()?;
    plan_component_action(
        document,
        &ComponentAction::SetProperty {
            node: theme,
            property: "BasedOn".into(),
            value,
        },
    )
}

/// Resolve a simple authored base reference with the same lexical search as resource edits.
pub fn base_theme(document: &Document, theme: NodeId) -> Result<Option<NodeId>> {
    let Some(attribute) = component_property_attribute(document, theme, "BasedOn")? else {
        return Ok(None);
    };
    let Ok(MarkupValue::Extension(extension)) = attribute.markup() else {
        return Ok(None);
    };
    if references::extension_kind(document, theme, &extension.name) != Some(false) {
        return Ok(None);
    }
    let Some(index) = references::resource_argument(&extension) else {
        return Ok(None);
    };
    let MarkupValue::Literal(key) = &extension.arguments[index].value else {
        return Ok(None);
    };
    Ok(resolve_component_resource(document, theme, key))
}

fn is_style(node: &Node) -> bool {
    framework(node) && node.local_name() == "Style"
}
fn is_theme(node: &Node) -> bool {
    framework(node) && node.local_name() == "ControlTheme"
}

#[cfg(test)]
#[path = "component_styles_tests.rs"]
mod tests;
