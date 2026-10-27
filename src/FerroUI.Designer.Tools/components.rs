//! Portable component definitions and scope-aware resource authoring. The catalog and
//! planners read the original document; native preview objects never become edit authority.
use crate::{Result, ToolError, error, require};
use ferroui_designer_core::{Document, Edit, MarkupValue, Node, NodeId, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[path = "component_authoring.rs"]
mod authoring;
#[path = "component_resource_refs.rs"]
mod references;
pub use authoring::{plan_component_action, plan_property_object};
pub use references::{component_resource_references, resolve_component_resource};

/// Build a scope-qualified StaticResource/DynamicResource expression without treating
/// punctuation, quotes or a leading brace in the key as markup syntax.
pub fn component_resource_expression(
    document: &Document,
    at: NodeId,
    key: &str,
    dynamic: bool,
) -> Result<String> {
    valid_document(document)?;
    element(document, at)?;
    authoring::validate_key(key)?;
    references::reference_expression(document, at, key, dynamic)
}

pub(crate) const FRAMEWORK: &str = "https://github.com/ferroui";
pub(crate) const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentKind {
    UserControl,
    CustomControl,
    ControlTemplate,
    DataTemplate,
    ControlTheme,
    Style,
    Resource,
    ResourceDictionary,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentEntry {
    pub node: NodeId,
    pub kind: ComponentKind,
    pub display_name: String,
    /// Canonical expanded type identity; it is independent of the source prefix.
    pub type_name: String,
    /// The authored key, including a type-key expression for implicit themes.
    pub key: Option<String>,
    pub target_type: Option<String>,
    /// The nearest resource owner when this is a resource entry.
    pub owner: Option<NodeId>,
    pub scope: NodeId,
    /// Resolved references in this document. External documents are indexed by the host.
    pub usages: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentCatalog {
    pub entries: Vec<ComponentEntry>,
}
impl ComponentCatalog {
    pub fn from_document(document: &Document) -> Result<Self> {
        valid_document(document)?;
        let mut entries = Vec::new();
        for node in document.ast().nodes_in_source_order() {
            if node.kind != NodeKind::Element {
                continue;
            }
            let key = literal_key(node)
                .or_else(|| key_attribute(node).map(|attribute| attribute.value.clone()));
            let kind = if framework(node) {
                match node.local_name() {
                    "UserControl" => Some(ComponentKind::UserControl),
                    "ControlTemplate" => Some(ComponentKind::ControlTemplate),
                    "DataTemplate" | "TreeDataTemplate" => Some(ComponentKind::DataTemplate),
                    "ControlTheme" => Some(ComponentKind::ControlTheme),
                    "Style" => Some(ComponentKind::Style),
                    "ResourceDictionary" => Some(ComponentKind::ResourceDictionary),
                    _ => key.as_ref().map(|_| ComponentKind::Resource),
                }
            } else if key.is_some() {
                Some(ComponentKind::Resource)
            } else if !matches!(node.namespace_uri.as_deref(), Some(XAML) | None)
                && !matches!(
                    node.local_name(),
                    "Binding" | "StaticResource" | "DynamicResource"
                )
            {
                Some(ComponentKind::CustomControl)
            } else {
                None
            };
            let Some(kind) = kind else { continue };
            let owner = resource_owner(document, node.id);
            let target_type = node
                .attribute("TargetType")
                .or_else(|| node.attribute("DataType"))
                .or_else(|| node.attribute("Selector"))
                .map(str::to_owned);
            let display_name = key
                .as_deref()
                .or_else(|| node.xaml_name())
                .or_else(|| directive(node, "Class"))
                .or(target_type.as_deref())
                .unwrap_or(node.local_name())
                .to_owned();
            entries.push(ComponentEntry {
                node: node.id,
                kind,
                display_name,
                type_name: format!(
                    "{{{}}}{}",
                    node.namespace_uri.as_deref().unwrap_or(""),
                    node.local_name()
                ),
                key,
                target_type,
                owner,
                scope: owner.unwrap_or_else(|| document.ast().name_scope(node.id)),
                usages: 0,
            });
        }
        let mut usages = BTreeMap::<NodeId, usize>::new();
        for (_, _, _, resource) in references::all_references(document)? {
            *usages.entry(resource).or_default() += 1;
        }
        for entry in &mut entries {
            entry.usages = usages.get(&entry.node).copied().unwrap_or(0);
        }
        Ok(Self { entries })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentReference {
    pub node: NodeId,
    pub property: String,
    pub dynamic: bool,
}

/// Semantic commands suitable for UI controls, automation and alternative designer hosts.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentAction {
    CreateResource {
        owner: NodeId,
        key: String,
        source: String,
    },
    DuplicateResource {
        resource: NodeId,
        key: String,
    },
    RenameResource {
        resource: NodeId,
        key: String,
    },
    RemoveResource {
        resource: NodeId,
    },
    ApplyResource {
        resource: NodeId,
        targets: Vec<NodeId>,
        property: String,
        dynamic: bool,
    },
    SetProperty {
        node: NodeId,
        property: String,
        value: Option<String>,
    },
    SetObjectProperty {
        node: NodeId,
        property: String,
        source: Option<String>,
    },
    SetSetter {
        owner: NodeId,
        property: String,
        value: Option<String>,
    },
    SetSetterObject {
        owner: NodeId,
        property: String,
        source: Option<String>,
    },
    SetResourceValue {
        resource: NodeId,
        value: String,
    },
    ReplaceDefinition {
        node: NodeId,
        source: String,
    },
    CreateTheme {
        owner: NodeId,
        key: String,
        target_type: String,
        based_on: Option<NodeId>,
    },
    CreateTemplate {
        owner: NodeId,
        key: String,
        target_type: Option<String>,
        content: String,
        data_template: bool,
    },
    CopyTemplate {
        template: NodeId,
        owner: NodeId,
        key: String,
    },
    ExtractProperty {
        node: NodeId,
        property: String,
        owner: NodeId,
        key: String,
    },
}

/// A self-contained source fragment retaining original trivia and all inherited namespaces.
/// Explicit namespace declarations on the element are kept exactly as authored.
pub fn component_fragment(document: &Document, node: NodeId) -> Result<String> {
    valid_document(document)?;
    let element = element(document, node)?;
    let mut source = document.source()[element.span.range()].to_owned();
    let namespaces = document.ast().namespaces(node);
    let mut declarations = String::new();
    for (prefix, uri) in namespaces {
        if prefix == "xml" {
            continue;
        }
        let name = if prefix.is_empty() {
            "xmlns".to_owned()
        } else {
            format!("xmlns:{prefix}")
        };
        if element.attribute(&name).is_none() {
            declarations.push_str(&format!(" {name}=\"{}\"", escape(&uri)));
        }
    }
    source.insert_str(element.name_span.end - element.span.start, &declarations);
    Ok(source)
}

/// Export a property's object/collection content with independent namespace closure.
pub fn component_property_object(
    document: &Document,
    node: NodeId,
    property: &str,
) -> Result<Option<String>> {
    valid_document(document)?;
    let element = element(document, node)?;
    let properties = property_elements(document, element, property)?;
    let implicit = implicit_property_children(document, element, property);
    if !implicit.is_empty() {
        require(
            properties.is_empty(),
            "The property has both explicit and implicit object values",
        )?;
        return implicit
            .into_iter()
            .map(|child| component_fragment(document, child.id))
            .collect::<Result<Vec<_>>>()
            .map(|parts| Some(parts.join("\n")));
    }
    let Some(property) = properties.first() else {
        return Ok(None);
    };
    let mut result = String::new();
    for child in &property.children {
        let child = document
            .ast()
            .node(*child)
            .ok_or_else(|| ToolError("Missing property child".into()))?;
        if child.is_element() {
            result.push_str(&component_fragment(document, child.id)?);
        } else {
            result.push_str(&document.source()[child.span.range()]);
        }
    }
    Ok(Some(result))
}

pub(crate) fn implicit_property_children<'a>(
    document: &'a Document,
    node: &Node,
    property: &str,
) -> Vec<&'a Node> {
    if property != "Value" || !framework(node) || node.local_name() != "Setter" {
        return Vec::new();
    }
    document
        .ast()
        .element_children(node.id)
        .into_iter()
        .filter_map(|id| document.ast().node(id))
        .filter(|child| child.kind == NodeKind::Element)
        .collect()
}

/// Visible string-key resources, with nearest scope and merge order already resolved.
pub fn visible_component_resources(
    document: &Document,
    target: NodeId,
) -> Result<Vec<ComponentEntry>> {
    element(document, target)?;
    Ok(ComponentCatalog::from_document(document)?
        .entries
        .into_iter()
        .filter(|entry| {
            entry.key.as_deref().is_some_and(|key| {
                resolve_component_resource(document, target, key) == Some(entry.node)
            })
        })
        .collect())
}

pub(crate) fn element(document: &Document, node: NodeId) -> Result<&Node> {
    document
        .ast()
        .node(node)
        .filter(|node| node.is_element())
        .ok_or_else(|| ToolError("The component element no longer exists".into()))
}
pub(crate) fn valid_document(document: &Document) -> Result<()> {
    require(
        document.is_valid(),
        "Repair the source before editing component definitions",
    )
}
pub(crate) fn framework(node: &Node) -> bool {
    node.namespace_uri.as_deref() == Some(FRAMEWORK)
}
pub(crate) fn directive<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    node.attributes
        .iter()
        .find(|attribute| {
            attribute.namespace_uri.as_deref() == Some(XAML)
                && attribute.name.rsplit(':').next() == Some(name)
        })
        .map(|attribute| attribute.value.as_str())
}
pub(crate) fn key_attribute(node: &Node) -> Option<&ferroui_designer_core::Attribute> {
    node.attributes.iter().find(|attribute| {
        attribute.namespace_uri.as_deref() == Some(XAML)
            && attribute.name.rsplit(':').next() == Some("Key")
    })
}
pub(crate) fn literal_key(node: &Node) -> Option<String> {
    match key_attribute(node)?.markup().ok()? {
        MarkupValue::Literal(key) => Some(key),
        _ => None,
    }
}
pub(crate) fn resource_owner(document: &Document, node: NodeId) -> Option<NodeId> {
    let parent = document
        .ast()
        .node(node)?
        .parent
        .and_then(|id| document.ast().node(id))?;
    if framework(parent) && parent.local_name() == "ResourceDictionary" {
        return Some(parent.id);
    }
    if parent.kind == NodeKind::PropertyElement && parent.local_name().ends_with(".Resources") {
        return parent.parent;
    }
    None
}
pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub(crate) fn property_name(property: &str) -> Result<()> {
    require(
        !property.is_empty()
            && property.len() <= 1024
            && property
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | ':' | '-')),
        "Invalid XAML property name",
    )
}
pub(crate) fn property_elements<'a>(
    document: &'a Document,
    node: &Node,
    property: &str,
) -> Result<Vec<&'a Node>> {
    property_name(property)?;
    let member = property.rsplit('.').next().unwrap_or(property);
    let result = document
        .ast()
        .element_children(node.id)
        .into_iter()
        .filter_map(|id| document.ast().node(id))
        .filter(|child| {
            child.kind == NodeKind::PropertyElement
                && child.local_name().rsplit('.').next() == Some(member)
        })
        .collect::<Vec<_>>();
    require(
        result.len() <= 1,
        "The property has multiple authored object values",
    )?;
    Ok(result)
}
pub(crate) fn attributes_for<'a>(
    node: &'a Node,
    property: &str,
) -> Result<Vec<&'a ferroui_designer_core::Attribute>> {
    property_name(property)?;
    let local = property.rsplit(':').next().unwrap_or(property);
    let values = node
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name == property || attribute.name.rsplit(':').next() == Some(local)
        })
        .collect::<Vec<_>>();
    require(
        values.len() <= 1,
        "The property has duplicate namespace aliases",
    )?;
    Ok(values)
}
pub(crate) fn validated(document: &Document, edits: Vec<Edit>) -> Result<Vec<Edit>> {
    valid_document(document)?;
    let mut candidate = document.clone();
    candidate
        .apply("Validate component edit", edits.clone())
        .map_err(error)?;
    Ok(edits)
}
pub(crate) fn unique_targets(document: &Document, nodes: &[NodeId]) -> Result<Vec<NodeId>> {
    let mut unique = BTreeSet::new();
    for id in nodes {
        element(document, *id)?;
        unique.insert(*id);
    }
    require(!unique.is_empty(), "Select at least one target")?;
    Ok(unique.into_iter().collect())
}

#[cfg(test)]
#[path = "component_tests.rs"]
mod tests;
