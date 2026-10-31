//! Copy authored definitions with their resource dependencies. The complete candidate
//! is planned privately and committed once; imported names cannot capture local keys.
use super::{DesignerDefinitionAsset, DesignerWorkspace};
use ferroui_base::controls::ResourceKey;
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, SourceSpan};
use ferroui_designer_tools::{
    ComponentAction, ComponentCatalog, component_fragment, plan_component_action,
    resolve_component_resource,
};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone)]
pub(super) struct DefinitionSource {
    pub document: Rc<Document>,
    pub node: NodeId,
}

impl DesignerWorkspace {
    pub(super) fn copy_provided_definition(
        &self,
        asset: &DesignerDefinitionAsset,
        owner: NodeId,
        key: &str,
    ) -> Result<(), String> {
        let registered = self.components.borrow().sources.get(&asset.id).cloned();
        let provider = match registered {
            Some(provider) => provider,
            None => {
                let document =
                    Rc::new(Document::new(&asset.source).map_err(|error| error.to_string())?);
                DefinitionSource {
                    node: document.ast().root(),
                    document,
                }
            }
        };
        let session = self.session.borrow();
        let revision = session.document().revision();
        let host = session.preview_host();
        let edits = plan_import(session.document(), owner, key, &provider, &|key| {
            host.try_find_resource(key, host.actual_theme_variant().as_ref())
                .is_some()
                || ferroui_controls::Application::current().is_some_and(|app| {
                    app.try_get_resource(key, host.actual_theme_variant().as_ref())
                        .is_some()
                })
        })?;
        drop(session);
        self.session
            .borrow_mut()
            .apply_at(revision, "Copy definition and resource dependencies", edits)
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn plan_import(
    target: &Document,
    owner: NodeId,
    key: &str,
    provider: &DefinitionSource,
    external: &impl Fn(&ResourceKey) -> bool,
) -> Result<Vec<Edit>, String> {
    let mut imported = provider.document.as_ref().clone();
    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    collect_dependencies(
        &imported,
        provider.node,
        target,
        owner,
        external,
        &mut visiting,
        &mut visited,
        &mut order,
    )?;
    let mut names = ComponentCatalog::from_document(target)
        .map_err(|error| error.to_string())?
        .entries
        .into_iter()
        .chain(
            ComponentCatalog::from_document(&imported)
                .map_err(|error| error.to_string())?
                .entries,
        )
        .filter_map(|entry| entry.key)
        .collect::<BTreeSet<_>>();
    names.insert(key.into());
    let mut keys = BTreeMap::new();
    for node in order.iter().copied().filter(|node| *node != provider.node) {
        let old = literal_key(
            imported
                .ast()
                .node(node)
                .ok_or("The dependency was removed")?,
        )
        .ok_or("A dependency needs an explicit string key to make an editable copy")?;
        let base = format!("{}_{}", key, old);
        let mut fresh = base.clone();
        let mut index = 2;
        while names.contains(&fresh) {
            fresh = format!("{base}{index}");
            index += 1;
        }
        names.insert(fresh.clone());
        let action = ComponentAction::RenameResource {
            resource: node,
            key: fresh.clone(),
        };
        let edits = plan_component_action(&imported, &action).map_err(|error| error.to_string())?;
        imported
            .apply("Stage imported resource identity", edits)
            .map_err(|error| error.to_string())?;
        keys.insert(node, fresh);
    }
    let mut candidate = target.clone();
    for node in order {
        let name = keys.get(&node).map_or(key, String::as_str);
        let source = component_fragment(&imported, node).map_err(|error| error.to_string())?;
        let action = ComponentAction::CreateResource {
            owner,
            key: name.into(),
            source,
        };
        let edits =
            plan_component_action(&candidate, &action).map_err(|error| error.to_string())?;
        candidate
            .apply("Stage imported definition", edits)
            .map_err(|error| error.to_string())?;
    }
    // A single bounded source patch keeps the whole import one history/collaboration
    // transaction while avoiding candidate-only node identities in the public edit list.
    Ok(vec![difference(target.source(), candidate.source())])
}

#[allow(clippy::too_many_arguments)]
fn collect_dependencies(
    source: &Document,
    root: NodeId,
    target: &Document,
    owner: NodeId,
    external: &impl Fn(&ResourceKey) -> bool,
    visiting: &mut BTreeSet<NodeId>,
    visited: &mut BTreeSet<NodeId>,
    order: &mut Vec<NodeId>,
) -> Result<(), String> {
    if visited.contains(&root) {
        return Ok(());
    }
    if visiting.len() >= 128 || visited.len() >= 4096 {
        return Err("This resource dependency graph is too large to copy as one definition; register a smaller self-contained source document".into());
    }
    if !visiting.insert(root) {
        return Err("A cyclic resource dependency cannot be copied independently".into());
    }
    let mut dependencies = BTreeSet::new();
    for node in source.ast().subtree(root) {
        for key in resource_keys(source, node)? {
            match &key {
                ResourceKey::String(value) => {
                    if let Some(dependency) = resolve_component_resource(source, node.id, value) {
                        if dependency == root {
                            return Err(format!(
                                "Resource {value} refers to itself and cannot be copied independently"
                            ));
                        }
                        if !source.ast().is_descendant_of(dependency, root) {
                            dependencies.insert(dependency);
                        }
                    } else if resolve_component_resource(target, owner, value).is_none()
                        && !external(&key)
                    {
                        return Err(format!(
                            "Resource {value} is required by this definition. Register its source document or load its resource dictionary before creating a copy."
                        ));
                    }
                }
                _ if !external(&key) => {
                    return Err(format!(
                        "The resource keyed by {key} must be loaded before copying this definition"
                    ));
                }
                _ => {}
            }
        }
    }
    for dependency in dependencies {
        collect_dependencies(
            source, dependency, target, owner, external, visiting, visited, order,
        )?;
    }
    visiting.remove(&root);
    visited.insert(root);
    order.push(root);
    Ok(())
}

fn resource_keys(document: &Document, node: &Node) -> Result<Vec<ResourceKey>, String> {
    let mut keys = Vec::new();
    if is_framework_namespace(node.namespace_uri.as_deref())
        && resource_extension_name(node.local_name())
    {
        if let Some(value) = node.attribute("ResourceKey") {
            keys.push(resource_key(
                document,
                node.id,
                &MarkupValue::parse(value)?,
            )?);
        }
    }
    for attribute in &node.attributes {
        if !attribute.is_namespace() && attribute.value.starts_with('{') {
            collect_markup_resources(
                document,
                node.id,
                &attribute.markup().map_err(|error| error.to_string())?,
                &mut keys,
            )?;
        }
    }
    Ok(keys)
}

fn collect_markup_resources(
    document: &Document,
    node: NodeId,
    value: &MarkupValue,
    keys: &mut Vec<ResourceKey>,
) -> Result<(), String> {
    let MarkupValue::Extension(extension) = value else {
        return Ok(());
    };
    let (prefix, local) = extension
        .name
        .split_once(':')
        .unwrap_or(("", &extension.name));
    let namespaces = document.ast().namespaces(node);
    if resource_extension_name(local)
        && is_framework_namespace(namespaces.get(prefix).map(String::as_str))
    {
        let argument = extension
            .arguments
            .iter()
            .find(|argument| argument.name.as_deref() == Some("ResourceKey"))
            .or_else(|| {
                extension
                    .arguments
                    .iter()
                    .find(|argument| argument.name.is_none())
            })
            .ok_or("A resource reference requires a key")?;
        keys.push(resource_key(document, node, &argument.value)?);
    }
    for argument in &extension.arguments {
        collect_markup_resources(document, node, &argument.value, keys)?;
    }
    Ok(())
}

fn resource_key(
    document: &Document,
    node: NodeId,
    value: &MarkupValue,
) -> Result<ResourceKey, String> {
    match value {
        MarkupValue::Literal(value) => Ok(value.as_str().into()),
        MarkupValue::Extension(extension) => {
            let namespaces = document.ast().namespaces(node);
            let (prefix, local) = extension
                .name
                .split_once(':')
                .unwrap_or(("", &extension.name));
            if local != "Type"
                || namespaces.get(prefix).map(String::as_str)
                    != Some("http://schemas.microsoft.com/winfx/2006/xaml")
            {
                return Err("This resource uses a custom key expression; provide a self-contained definition source to copy it".into());
            }
            let token = extension
                .arguments
                .iter()
                .find(|argument| {
                    argument.name.is_none() || argument.name.as_deref() == Some("TypeName")
                })
                .and_then(|argument| {
                    if let MarkupValue::Literal(value) = &argument.value {
                        Some(value.as_str())
                    } else {
                        None
                    }
                })
                .ok_or("A type resource key requires a concrete registered type")?;
            let (prefix, local) = token.split_once(':').unwrap_or(("", token));
            let namespace = namespaces
                .get(prefix)
                .ok_or("Unknown type resource namespace")?;
            let type_ =
                super::control_types::registered_xml_type(namespace, local).ok_or_else(|| {
                    format!(
                        "Register the native type {token} before copying its resource definition"
                    )
                })?;
            Ok(ResourceKey::Type(type_))
        }
    }
}

fn resource_extension_name(name: &str) -> bool {
    matches!(
        name,
        "StaticResource"
            | "DynamicResource"
            | "StaticResourceExtension"
            | "DynamicResourceExtension"
    )
}

fn is_framework_namespace(namespace: Option<&str>) -> bool {
    matches!(
        namespace,
        Some(
            "https://github.com/ferroui"
                | "using:FerroUI.Markup.Xaml.MarkupExtensions"
                | "clr-namespace:FerroUI.Markup.Xaml.MarkupExtensions;assembly=FerroUI.Markup.Xaml"
        )
    )
}
fn literal_key(node: &Node) -> Option<String> {
    let attribute = node.attributes.iter().find(|attribute| {
        attribute.namespace_uri.as_deref() == Some("http://schemas.microsoft.com/winfx/2006/xaml")
            && attribute.name.rsplit(':').next() == Some("Key")
    })?;
    match attribute.markup().ok()? {
        MarkupValue::Literal(key) => Some(key),
        _ => None,
    }
}
fn difference(before: &str, after: &str) -> Edit {
    let mut start = before
        .bytes()
        .zip(after.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(start) || !after.is_char_boundary(start) {
        start -= 1;
    }
    let mut suffix = before[start..]
        .bytes()
        .rev()
        .zip(after[start..].bytes().rev())
        .take_while(|(a, b)| a == b)
        .count();
    while !before.is_char_boundary(before.len() - suffix)
        || !after.is_char_boundary(after.len() - suffix)
    {
        suffix -= 1;
    }
    Edit::Text {
        span: SourceSpan::new(start, before.len() - suffix),
        replacement: after[start..after.len() - suffix].into(),
    }
}

#[cfg(test)]
#[path = "workspace_component_import_tests.rs"]
mod tests;
