//! Browse authored definitions by their owning object in the current view.
use crate::DesignerSession;
use ferroui_designer_animation::BehaviorModel;
use ferroui_designer_core::{Document, Node, NodeId, NodeKind, SourceSpan};
use ferroui_designer_rsx::RsxProjection;
use ferroui_designer_tools::{
    component_property_attribute, declaration_container, is_component_include,
    is_component_template, ComponentEntry, ComponentKind,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "library_scopes_rsx.rs"]
mod rsx;
#[cfg(test)]
#[path = "library_scopes_tests.rs"]
mod tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryObject {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) kind: &'static str,
    pub(crate) detail: Option<String>,
    pub(crate) node: Option<NodeId>,
    pub(crate) definition: Option<NodeId>,
    pub(crate) source: SourceSpan,
    pub(crate) source_uri: Option<String>,
}

impl LibraryObject {
    pub(crate) fn local(id: String, title: String, kind: &'static str, source: SourceSpan) -> Self {
        Self {
            id,
            title,
            kind,
            detail: None,
            node: None,
            definition: None,
            source,
            source_uri: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryScope {
    pub(crate) owner: NodeId,
    pub(crate) title: String,
    pub(crate) path: String,
    pub(crate) owner_range: Option<SourceSpan>,
    pub(crate) objects: Vec<LibraryObject>,
}

impl DesignerSession {
    pub(crate) fn library_scopes(
        &self,
        definitions: &[ComponentEntry],
    ) -> Result<Vec<LibraryScope>, String> {
        if let Some(rust) = &self.rust {
            return self.rust_library_scopes(&rust.projection()?);
        }
        if !self.authored_is_valid() || !self.document().is_valid() {
            return Err("Repair the current source before browsing its scopes".into());
        }
        let root = self.definition_scope().map(|scope| scope.node);
        xaml_scopes(self.document(), definitions, root)
    }

    pub(crate) fn rust_library_scopes(
        &self,
        projection: &RsxProjection<'_>,
    ) -> Result<Vec<LibraryScope>, String> {
        if !self.authored_is_valid() || !self.document().is_valid() {
            return Err("Repair the current Rust view before browsing its scopes".into());
        }
        Ok(rsx::scopes(
            self.document(),
            projection,
            self.definition_scope().map(|scope| scope.node),
        ))
    }
}

fn xaml_scopes(
    document: &Document,
    definitions: &[ComponentEntry],
    root: Option<NodeId>,
) -> Result<Vec<LibraryScope>, String> {
    let ast = document.ast();
    let nodes = ast
        .subtree(root.unwrap_or_else(|| ast.root()))
        .filter(|node| Some(node.id) != root);
    let definitions: BTreeMap<_, _> = definitions
        .iter()
        .map(|entry| (entry.node, entry))
        .collect();
    let mut groups = BTreeMap::new();
    let mut behavior_owners = BTreeSet::new();
    for node in nodes {
        if let Some((owner, object)) =
            xaml_object(document, node, definitions.get(&node.id).copied())
        {
            push(&mut groups, owner, object);
        }
        if let Some(owner) = node.parent.filter(|&owner| {
            BehaviorModel::is_collection_property(node) && behavior_owners.insert(owner)
        }) {
            for id in
                BehaviorModel::authored_nodes(document, owner).map_err(|error| error.to_string())?
            {
                let Some(behavior) = ast.node(id).filter(|node| node.kind == NodeKind::Element)
                else {
                    continue;
                };
                let title = behavior
                    .xaml_name()
                    .or_else(|| behavior.attribute("EventName"))
                    .unwrap_or(behavior.local_name())
                    .to_owned();
                let mut object = LibraryObject::local(
                    format!("behavior:{id}"),
                    title,
                    "Behavior",
                    behavior.name_span,
                );
                object.node = Some(id);
                push(&mut groups, owner, object);
            }
        }
        if node.kind == NodeKind::Element {
            for attribute in node
                .attributes
                .iter()
                .filter(|attribute| BehaviorModel::is_collection_attribute(attribute))
            {
                push(
                    &mut groups,
                    node.id,
                    LibraryObject::local(
                        format!("behaviors:{}:{}", node.id, attribute.name_span.start),
                        "Behavior collection".into(),
                        "Behavior",
                        attribute.value_span,
                    ),
                );
            }
        }
    }
    Ok(finish(
        document,
        groups,
        |node| Some(node.name_span),
        |node| {
            definitions
                .get(&node.id)
                .map(|entry| entry.display_name.as_str())
        },
    ))
}

fn xaml_object(
    document: &Document,
    node: &Node,
    entry: Option<&ComponentEntry>,
) -> Option<(NodeId, LibraryObject)> {
    let owner = || {
        entry
            .map(|entry| entry.declaration_owner(document))
            .or_else(|| declaration_container(document, node.id))
            .unwrap_or(node.id)
    };
    let (prefix, title, kind, definition) = if is_component_include(node) {
        let title = component_property_attribute(document, node.id, "Source")
            .ok()
            .flatten()
            .map_or_else(
                || node.local_name().to_owned(),
                |source| source.value.clone(),
            );
        ("include", title, "Include", None)
    } else if is_component_template(node)
        && entry.is_none_or(|entry| {
            !matches!(
                entry.kind,
                ComponentKind::ControlTemplate | ComponentKind::DataTemplate
            )
        })
    {
        let title = entry.map_or_else(
            || node.local_name().to_owned(),
            |entry| entry.display_name.clone(),
        );
        ("template", title, "Template", None)
    } else {
        let entry = entry?;
        (
            "document",
            entry.display_name.clone(),
            definition_kind(entry)?,
            Some(entry.node),
        )
    };
    let mut object =
        LibraryObject::local(format!("{prefix}:{}", node.id), title, kind, node.name_span);
    object.node = Some(node.id);
    object.definition = definition;
    Some((owner(), object))
}

fn definition_kind(entry: &ComponentEntry) -> Option<&'static str> {
    Some(match entry.kind {
        ComponentKind::ControlTemplate => "Control template",
        ComponentKind::DataTemplate => "Data template",
        ComponentKind::ControlTheme => "Control theme",
        ComponentKind::Style => "Style",
        ComponentKind::ResourceDictionary => "Resource dictionary",
        ComponentKind::Resource => "Resource",
        _ if entry.owner.is_some() && entry.key.is_some() => "Resource",
        _ => return None,
    })
}

pub(super) fn push(
    groups: &mut BTreeMap<NodeId, Vec<LibraryObject>>,
    owner: NodeId,
    object: LibraryObject,
) {
    groups.entry(owner).or_default().push(object);
}

pub(super) fn finish<'a>(
    document: &Document,
    groups: BTreeMap<NodeId, Vec<LibraryObject>>,
    owner_range: impl Fn(&Node) -> Option<SourceSpan>,
    authored_name: impl Fn(&Node) -> Option<&'a str>,
) -> Vec<LibraryScope> {
    let ast = document.ast();
    let mut scopes = groups
        .into_iter()
        .filter_map(|(owner, mut objects)| {
            let node = ast.node(owner)?;
            let mut path =
                std::iter::successors(Some(node), |node| node.parent.and_then(|id| ast.node(id)))
                    .filter(|node| node.kind == NodeKind::Element)
                    .map(|node| {
                        node.xaml_name()
                            .or_else(|| node.attribute("Header"))
                            .or_else(|| authored_name(node))
                            .unwrap_or(node.local_name())
                    })
                    .collect::<Vec<_>>();
            path.reverse();
            let title = path.last()?.to_string();
            objects.sort_by(|left, right| {
                (&left.source_uri, left.source.start, &left.id).cmp(&(
                    &right.source_uri,
                    right.source.start,
                    &right.id,
                ))
            });
            Some((
                node.span.start,
                LibraryScope {
                    owner,
                    title,
                    path: path.join(" › "),
                    owner_range: owner_range(node),
                    objects,
                },
            ))
        })
        .collect::<Vec<_>>();
    scopes.sort_by_key(|(start, _)| *start);
    scopes.into_iter().map(|(_, scope)| scope).collect()
}
