//! Lexical resource lookup and structured markup references shared by catalog and edits.
use super::*;
use ferroui_designer_core::{MarkupArgument, MarkupExtension};

enum Lookup {
    Found(NodeId),
    Missing,
    External,
}

/// Resolve exactly the resource visible at an authored element. Local resources override
/// merged dictionaries; the last merged dictionary wins. Unindexed external dictionaries
/// stop lookup rather than guessing that an earlier dictionary or ancestor wins.
pub fn resolve_component_resource(
    document: &Document,
    target: NodeId,
    key: &str,
) -> Option<NodeId> {
    if !document.is_valid() {
        return None;
    }
    let mut ancestor = Some(target);
    while let Some(id) = ancestor {
        let node = document.ast().node(id)?;
        match lookup_scope(document, node, key, &mut BTreeSet::new()) {
            Lookup::Found(id) => return Some(id),
            Lookup::External => return None,
            Lookup::Missing => {}
        }
        ancestor = node.parent;
    }
    None
}

fn lookup_scope(
    document: &Document,
    owner: &Node,
    key: &str,
    seen: &mut BTreeSet<NodeId>,
) -> Lookup {
    if !seen.insert(owner.id) {
        return Lookup::Missing;
    }
    if framework(owner) && owner.local_name() == "ResourceDictionary" {
        if external_dictionary(document, owner) {
            return Lookup::External;
        }
        return lookup_children(document, &owner.children, key, seen);
    }
    let Ok(resources) = property_elements(document, owner, "Resources") else {
        return Lookup::External;
    };
    let children = resources
        .into_iter()
        .flat_map(|node| node.children.iter().copied())
        .collect::<Vec<_>>();
    lookup_children(document, &children, key, seen)
}

fn lookup_children(
    document: &Document,
    children: &[NodeId],
    key: &str,
    seen: &mut BTreeSet<NodeId>,
) -> Lookup {
    let nodes = children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .collect::<Vec<_>>();
    let local = nodes
        .iter()
        .filter(|node| literal_key(node).as_deref() == Some(key))
        .map(|node| node.id)
        .collect::<Vec<_>>();
    if local.len() > 1 {
        return Lookup::External;
    } // Ambiguous local declarations are not safe edit targets.
    if let Some(id) = local.first() {
        return Lookup::Found(*id);
    }
    // Explicit dictionary wrappers are the property's actual resource collection.
    for node in nodes.iter().rev().filter(|node| {
        framework(node)
            && node.local_name() == "ResourceDictionary"
            && key_attribute(node).is_none()
    }) {
        match lookup_scope(document, node, key, seen) {
            Lookup::Missing => {}
            result => return result,
        }
    }
    for merged in nodes.iter().rev().filter(|node| {
        node.kind == NodeKind::PropertyElement && node.local_name().ends_with(".MergedDictionaries")
    }) {
        let Some(owner) = merged.parent.and_then(|id| document.ast().node(id)) else {
            continue;
        };
        let Ok(properties) = property_elements(document, owner, "MergedDictionaries") else {
            return Lookup::External;
        };
        if !properties.iter().any(|property| property.id == merged.id) {
            continue;
        }
        for child in merged
            .children
            .iter()
            .rev()
            .filter_map(|id| document.ast().node(*id))
            .filter(|node| node.is_element())
        {
            if framework(child)
                && child.local_name() == "ResourceDictionary"
                && !external_dictionary(document, child)
            {
                match lookup_scope(document, child, key, seen) {
                    Lookup::Missing => {}
                    result => return result,
                }
            } else {
                return Lookup::External;
            }
        }
    }
    Lookup::Missing
}

pub fn component_resource_references(
    document: &Document,
    resource: NodeId,
) -> Result<Vec<ComponentReference>> {
    valid_document(document)?;
    element(document, resource)?;
    Ok(all_references(document)?
        .into_iter()
        .filter(|(_, _, _, target)| *target == resource)
        .map(|(node, property, dynamic, _)| ComponentReference {
            node,
            property,
            dynamic,
        })
        .collect())
}

pub(crate) fn all_references(document: &Document) -> Result<Vec<(NodeId, String, bool, NodeId)>> {
    let mut result = Vec::new();
    for node in document.ast().elements() {
        for (property, dynamic, key) in resource_uses(document, node)? {
            if let Some(target) = resolve_component_resource(document, node.id, &key) {
                result.push((node.id, property, dynamic, target));
            }
        }
    }
    Ok(result)
}

/// Keep unresolved uses as well as resolved ones: moving/copying a definition can
/// capture a previously unknown key in its new lexical resource scope.
fn resource_uses(document: &Document, node: &Node) -> Result<Vec<(String, bool, String)>> {
    let mut result = Vec::new();
    if framework(node)
        && matches!(
            node.local_name(),
            "StaticResource"
                | "DynamicResource"
                | "StaticResourceExtension"
                | "DynamicResourceExtension"
        )
    {
        if let Some((property, key)) = object_resource_key(document, node)? {
            result.push((
                property,
                matches!(
                    node.local_name(),
                    "DynamicResource" | "DynamicResourceExtension"
                ),
                key,
            ));
        }
    }
    for attribute in &node.attributes {
        if attribute.is_namespace() || !attribute.value.starts_with('{') {
            continue;
        }
        let value = attribute.markup().map_err(ToolError)?;
        visit_resources(document, node.id, &value, &mut |key, dynamic| {
            result.push((attribute.name.clone(), dynamic, key.into()));
        });
    }
    Ok(result)
}

fn object_resource_key(document: &Document, node: &Node) -> Result<Option<(String, String)>> {
    let Some(attribute) = component_property_attribute(document, node.id, "ResourceKey")? else {
        return Ok(None);
    };
    Ok(match attribute.markup().map_err(ToolError)? {
        MarkupValue::Literal(key) => Some((attribute.name.clone(), key)),
        _ => None,
    })
}

/// A copied definition retains the meaning of every locally resolvable reference.
/// References to resources inside the copied subtree follow their corresponding copy;
/// references outside it must still resolve to the original declaration. External
/// dictionaries are intentionally left unresolved, as in the catalog's lookup contract.
pub(crate) fn validate_copied_references(
    document: &Document,
    original: NodeId,
    candidate: &Document,
    copied: NodeId,
) -> Result<()> {
    let before = document
        .ast()
        .subtree(original)
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    let after = candidate
        .ast()
        .subtree(copied)
        .filter(|node| node.is_element())
        .collect::<Vec<_>>();
    require(
        before.len() == after.len(),
        "The copied definition changed its object structure",
    )?;
    let copies = before
        .iter()
        .zip(&after)
        .map(|(a, b)| (a.id, b.id))
        .collect::<BTreeMap<_, _>>();
    for (before, after) in before.into_iter().zip(after) {
        let original_uses = resource_uses(document, before)?;
        let copied_uses = resource_uses(candidate, after)?;
        require(
            original_uses == copied_uses,
            "The copied definition changed its resource expressions",
        )?;
        for (_, _, key) in original_uses {
            let expected = resolve_component_resource(document, before.id, &key)
                .map(|resource| copies.get(&resource).copied().unwrap_or(resource));
            require(
                resolve_component_resource(candidate, after.id, &key) == expected,
                format!(
                    "Copying this definition would change the resource '{key}'; choose a scope that preserves its dependencies"
                ),
            )?;
        }
    }
    Ok(())
}

fn extension_kind(document: &Document, node: NodeId, name: &str) -> Option<bool> {
    let (prefix, local) = name.rsplit_once(':').unwrap_or(("", name));
    if !document
        .ast()
        .namespaces(node)
        .get(prefix)
        .is_some_and(|namespace| framework_type_namespace(local, namespace))
    {
        return None;
    }
    match local {
        "StaticResource" | "StaticResourceExtension" => Some(false),
        "DynamicResource" | "DynamicResourceExtension" => Some(true),
        _ => None,
    }
}

fn resource_argument(extension: &MarkupExtension) -> Option<usize> {
    extension
        .arguments
        .iter()
        .position(|argument| argument.name.as_deref() == Some("ResourceKey"))
        .or_else(|| {
            extension
                .arguments
                .iter()
                .position(|argument| argument.name.is_none())
        })
}

fn visit_resources(
    document: &Document,
    node: NodeId,
    value: &MarkupValue,
    visit: &mut impl FnMut(&str, bool),
) {
    let MarkupValue::Extension(extension) = value else {
        return;
    };
    if let Some(dynamic) = extension_kind(document, node, &extension.name) {
        if let Some(index) = resource_argument(extension) {
            if let MarkupValue::Literal(key) = &extension.arguments[index].value {
                visit(key, dynamic);
            }
        }
    }
    for argument in &extension.arguments {
        visit_resources(document, node, &argument.value, visit);
    }
}

pub(crate) fn rename_resource(
    document: &Document,
    resource: NodeId,
    key: &str,
) -> Result<Vec<Edit>> {
    let node = element(document, resource)?;
    let old = literal_key(node)
        .ok_or_else(|| ToolError("Only explicit string-key resources can be renamed".into()))?;
    super::authoring::validate_key(key)?;
    if old == key {
        return Ok(Vec::new());
    }
    let owner = resource_owner(document, resource)
        .ok_or_else(|| ToolError("The resource is not in an authored dictionary".into()))?;
    super::authoring::ensure_key_available(document, owner, key, Some(resource))?;
    let mut edits = Vec::new();
    for element in document.ast().elements() {
        if framework(element)
            && matches!(
                element.local_name(),
                "StaticResource"
                    | "DynamicResource"
                    | "StaticResourceExtension"
                    | "DynamicResourceExtension"
            )
        {
            if let Some((property, object_key)) = object_resource_key(document, element)? {
                if object_key == old
                    && resolve_component_resource(document, element.id, &old) == Some(resource)
                {
                    edits.push(Edit::SetAttribute {
                        node: element.id,
                        name: property,
                        value: escaped_literal(key),
                    });
                }
            }
        }
        for attribute in &element.attributes {
            if attribute.is_namespace() || !attribute.value.starts_with('{') {
                continue;
            }
            let mut value = attribute.markup().map_err(ToolError)?;
            if rewrite_resource(document, element.id, &mut value, resource, key) {
                edits.push(Edit::SetAttribute {
                    node: element.id,
                    name: attribute.name.clone(),
                    value: serialize(&value),
                });
            }
        }
    }
    edits.push(Edit::SetAttribute {
        node: resource,
        name: key_attribute(node).expect("literal key").name.clone(),
        value: escaped_literal(key),
    });
    let before = all_references(document)?;
    let mut candidate = document.clone();
    candidate
        .apply("Validate resource rename", edits.clone())
        .map_err(error)?;
    // Existing resolved references must retain the same declaration identity. A rename
    // cannot capture another key through an ancestor, merged dictionary or local shadow.
    let after = all_references(&candidate)?;
    require(
        before == after,
        "Renaming this resource would change another reference's scope; choose a different key",
    )?;
    Ok(edits)
}

fn rewrite_resource(
    document: &Document,
    node: NodeId,
    value: &mut MarkupValue,
    target: NodeId,
    key: &str,
) -> bool {
    let MarkupValue::Extension(extension) = value else {
        return false;
    };
    let mut changed = false;
    if extension_kind(document, node, &extension.name).is_some() {
        if let Some(index) = resource_argument(extension) {
            if let MarkupValue::Literal(old) = &extension.arguments[index].value {
                if resolve_component_resource(document, node, old) == Some(target) {
                    extension.arguments[index].value = MarkupValue::Literal(key.into());
                    changed = true;
                }
            }
        }
    }
    for argument in &mut extension.arguments {
        changed |= rewrite_resource(document, node, &mut argument.value, target, key);
    }
    changed
}

pub(crate) fn escaped_literal(value: &str) -> String {
    if value.starts_with('{') {
        format!("{{}}{value}")
    } else {
        value.into()
    }
}
pub(crate) fn argument(value: &str) -> String {
    format!(
        "'{}'",
        escaped_literal(value)
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
    )
}
fn serialize(value: &MarkupValue) -> String {
    match value {
        MarkupValue::Literal(value) => argument(value),
        MarkupValue::Extension(extension) => {
            let arguments = extension
                .arguments
                .iter()
                .map(|MarkupArgument { name, value }| {
                    let value = serialize(value);
                    name.as_ref()
                        .map_or(value.clone(), |name| format!("{name}={value}"))
                })
                .collect::<Vec<_>>()
                .join(", ");
            if arguments.is_empty() {
                format!("{{{}}}", extension.name)
            } else {
                format!("{{{} {arguments}}}", extension.name)
            }
        }
    }
}

pub(crate) fn reference_expression(
    document: &Document,
    node: NodeId,
    key: &str,
    dynamic: bool,
) -> Result<String> {
    let namespaces = document.ast().namespaces(node);
    let prefix = namespaces
        .iter()
        .find(|(_, uri)| framework_type_namespace("StaticResource", uri))
        .map(|(prefix, _)| prefix)
        .ok_or_else(|| {
            ToolError("A FerroUI namespace is required to apply a resource reference".into())
        })?;
    let name = if dynamic {
        "DynamicResource"
    } else {
        "StaticResource"
    };
    let qualified = if prefix.is_empty() {
        name.into()
    } else {
        format!("{prefix}:{name}")
    };
    Ok(format!("{{{qualified} {}}}", argument(key)))
}
