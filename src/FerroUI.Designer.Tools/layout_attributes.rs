//! Namespace-aware source authoring shared by placement and layout helper transactions.
use crate::{Result, ToolError, error, require};
use ferroui_designer_core::{Attribute, Document, Edit, Node, NodeId, NodeKind};
use std::collections::BTreeSet;

pub(crate) fn framework_namespace(namespace: Option<&str>) -> bool {
    matches!(
        namespace,
        None | Some(
            "https://github.com/ferroui"
                | "using:FerroUI.Controls"
                | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
        )
    )
}
pub(crate) fn layout_attribute<'a>(
    document: &Document,
    node: &'a Node,
    name: &str,
) -> Result<Option<&'a Attribute>> {
    let lexical = name.contains(':');
    let mut matches = node.attributes.iter().filter(|attribute| {
        if lexical {
            return attribute.name == name;
        }
        attribute.name.rsplit(':').next() == Some(name)
            && if name.contains('.') && !attribute.name.contains(':') {
                framework_namespace(
                    document
                        .ast()
                        .namespaces(node.id)
                        .get("")
                        .map(String::as_str),
                )
            } else {
                framework_namespace(attribute.namespace_uri.as_deref())
            }
    });
    let first = matches.next();
    require(
        matches.next().is_none(),
        format!("{name} is declared more than once through namespace aliases"),
    )?;
    Ok(first)
}
pub(crate) fn ensure_layout_literal(document: &Document, node: &Node, name: &str) -> Result<()> {
    if let Some(attribute) = layout_attribute(document, node, name)? {
        let value = attribute.value.trim_start();
        require(
            !value.starts_with('{') || value.starts_with("{}"),
            format!("{name} is an expression; edit its source explicitly"),
        )?;
    }
    let local = name.rsplit(':').next().unwrap_or(name);
    require(
        !node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .any(|child| {
                child.kind == NodeKind::PropertyElement
                    && if name.contains(':') {
                        child.name == name
                            || child.name.ends_with(&format!(".{local}"))
                                && child.namespace_uri == node.namespace_uri
                    } else {
                        framework_namespace(child.namespace_uri.as_deref())
                            && (child.local_name() == local
                                || child.local_name().ends_with(&format!(".{local}")))
                    }
            }),
        format!("{name} is authored as a property element; edit its source explicitly"),
    )
}
/// Plan native layout attributes without changing attached-property namespace identity or
/// overwriting bindings/property elements. Values must already be validated by the adapter.
/// New native attached properties are qualified when the lexical default namespace is custom.
pub fn plan_layout_attributes(
    document: &Document,
    node: NodeId,
    attributes: &[(String, String)],
) -> Result<Vec<Edit>> {
    require(
        document.is_valid(),
        "Layout attributes require valid source",
    )?;
    let element = document
        .ast()
        .node(node)
        .ok_or_else(|| ToolError("Unknown layout target".into()))?;
    require(element.is_element(), "Layout attributes require an element")?;
    let namespaces = document.ast().namespaces(node);
    let mut prefix = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && framework_namespace(Some(uri.as_str())))
        .map(|(prefix, _)| prefix.to_owned());
    let mut declaration = None;
    let mut edits = Vec::new();
    let mut names = BTreeSet::new();
    for (name, value) in attributes {
        require(
            names.insert(name.as_str()),
            "A layout plan writes the same attribute twice",
        )?;
        ensure_layout_literal(document, element, name)?;
        let authored = layout_attribute(document, element, name)?;
        if authored.is_some_and(|attribute| attribute.value == *value) {
            continue;
        }
        let target = if let Some(attribute) = authored {
            attribute.name.clone()
        } else if !name.contains(':')
            && name.contains('.')
            && !framework_namespace(namespaces.get("").map(String::as_str))
        {
            if prefix.is_none() {
                let mut candidate = "layout".to_owned();
                let mut suffix = 1;
                while namespaces.contains_key(&candidate) {
                    candidate = format!("layout{suffix}");
                    suffix += 1;
                }
                prefix = Some(candidate.clone());
                declaration = Some(candidate);
            }
            format!("{}:{name}", prefix.as_ref().unwrap())
        } else {
            name.clone()
        };
        edits.push(Edit::SetAttribute {
            node,
            name: target,
            value: value.clone(),
        });
    }
    if let Some(prefix) = declaration {
        edits.insert(
            0,
            Edit::SetAttribute {
                node,
                name: format!("xmlns:{prefix}"),
                value: "https://github.com/ferroui".into(),
            },
        );
    }
    let mut candidate = document.clone();
    candidate
        .apply("Validate native layout attributes", edits.clone())
        .map_err(error)?;
    Ok(edits)
}
