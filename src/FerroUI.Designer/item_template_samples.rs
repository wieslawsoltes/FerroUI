//! Select a representative item from the active fixture without evaluating application code.
use crate::{
    DefinitionScope, NodeId, component_session::property_schema, design_time_samples::simple_path,
};
use ferroui_designer_core::{
    Document, MarkupValue, Node,
    design_fixtures::{DESIGN_NAMESPACE, DesignFixture, DesignFixtures},
};
use ferroui_designer_language::FRAMEWORK_NAMESPACE;
use ferroui_designer_tools::{ItemTemplateMember, nearest_table};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemTemplatePreview {
    pub(crate) owner: NodeId,
    pub(crate) property: ItemTemplateMember,
}

pub(crate) fn definition_fixture(
    document: &Document,
    scope: &DefinitionScope,
    enabled: bool,
    selected: Option<&str>,
) -> Result<Option<DesignFixture>, String> {
    let Some(template) = scope.item_template.filter(|_| enabled) else {
        return Ok(None);
    };
    let Some((owner, property)) = source_owner(document, template) else {
        return Ok(None);
    };
    let Some(path) = items_binding(document, owner, property) else {
        return Ok(None);
    };
    let Some(manifest) = DesignFixtures::from_document(document)? else {
        return Ok(None);
    };
    let Some(fixture) = manifest
        .get(selected.unwrap_or(&manifest.default))
        .or_else(|| manifest.get(&manifest.default))
    else {
        return Ok(None);
    };
    let mut members = path.split('.');
    let mut value = members.next().and_then(|root| fixture.data.get(root));
    for member in members {
        value = value.and_then(|value| value.get(member));
    }
    Ok(value
        .and_then(serde_json::Value::as_array)
        .and_then(|items| items.first())
        .map(|item| DesignFixture {
            name: fixture.name.clone(),
            data: [("Item".into(), item.clone())].into_iter().collect(),
        }))
}

fn source_owner(
    document: &Document,
    template: ItemTemplatePreview,
) -> Option<(&Node, &'static str)> {
    use ItemTemplateMember::*;
    let owner = match template.property {
        CellTemplate | CellEditingTemplate => nearest_table(document, template.owner)?,
        _ => template.owner,
    };
    let property = if template.property == MenuItemTemplate {
        "MenuItemsSource"
    } else {
        "ItemsSource"
    };
    Some((document.ast().node(owner)?, property))
}

fn items_binding(document: &Document, owner: &Node, property: &str) -> Option<String> {
    let schema = property_schema();
    let attribute = owner
        .attributes
        .iter()
        .find(|attribute| {
            attribute.namespace_uri.as_deref() == Some(DESIGN_NAMESPACE)
                && attribute.name.rsplit(':').next() == Some(property)
        })
        .or_else(|| {
            owner.attributes.iter().find(|attribute| {
                !attribute.is_namespace()
                    && schema
                        .attribute_property(document, owner, attribute)
                        .is_some_and(|info| info.name == property)
            })
        })?;
    let MarkupValue::Extension(extension) = attribute.markup().ok()? else {
        return None;
    };
    let (prefix, name) = extension
        .name
        .rsplit_once(':')
        .unwrap_or(("", &extension.name));
    let namespaces = document.ast().namespaces(owner.id);
    let namespace = namespaces.get(prefix).map(String::as_str);
    if (!prefix.is_empty() && namespace.is_none())
        || !matches!(
            name,
            "Binding"
                | "BindingExtension"
                | "CompiledBinding"
                | "CompiledBindingExtension"
                | "ReflectionBinding"
                | "ReflectionBindingExtension"
        )
        || !namespace.is_none_or(|namespace| {
            namespace == FRAMEWORK_NAMESPACE
                || schema
                    .type_info(name, Some(namespace))
                    .is_some_and(|info| info.namespace == FRAMEWORK_NAMESPACE)
        })
        || extension.arguments.iter().any(|argument| {
            matches!(
                argument.name.as_deref(),
                Some("Source" | "ElementName" | "RelativeSource")
            )
        })
    {
        return None;
    }
    let argument = extension
        .arguments
        .into_iter()
        .find(|argument| matches!(argument.name.as_deref(), None | Some("Path")))?;
    let MarkupValue::Literal(path) = argument.value else {
        return None;
    };
    simple_path(&path).then_some(path)
}
