//! Select a representative item from the active fixture without evaluating application code.
use crate::{
    component_session::property_schema, design_time_samples::simple_path, DefinitionScope, NodeId,
};
use ferroui_designer_core::{
    design_fixtures::{DesignFixture, DesignFixtures, DESIGN_NAMESPACE},
    Document, MarkupValue, Node,
};
use ferroui_designer_language::FRAMEWORK_NAMESPACE;
use ferroui_designer_tools::{nearest_table, ItemTemplateMember};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ItemTemplatePreview {
    Assigned {
        owner: NodeId,
        property: ItemTemplateMember,
    },
    Included {
        key: String,
        sample: Option<DesignFixture>,
        consumer: String,
    },
}

pub(crate) fn definition_fixture(
    document: &Document,
    scope: &DefinitionScope,
    enabled: bool,
    selected: Option<&str>,
) -> Result<Option<DesignFixture>, String> {
    let Some(template) = scope.item_template.as_ref().filter(|_| enabled) else {
        return Ok(None);
    };
    match template {
        ItemTemplatePreview::Assigned { owner, property } => {
            assigned_fixture(document, *owner, *property, selected)
        }
        ItemTemplatePreview::Included { sample, .. } => Ok(sample.clone()),
    }
}

pub(crate) fn assigned_fixture(
    document: &Document,
    owner: NodeId,
    property: ItemTemplateMember,
    selected: Option<&str>,
) -> Result<Option<DesignFixture>, String> {
    let Some((owner, property)) = source_owner(document, owner, property) else {
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
    owner: NodeId,
    template: ItemTemplateMember,
) -> Option<(&Node, &'static str)> {
    use ItemTemplateMember::*;
    let owner = match template {
        CellTemplate | CellEditingTemplate => nearest_table(document, owner)?,
        _ => owner,
    };
    let property = if template == MenuItemTemplate {
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
