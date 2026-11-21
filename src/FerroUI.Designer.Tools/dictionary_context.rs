//! Preserve the XML and XAML context that previously surrounded the dictionary.
use super::{Result, require};
use ferroui_designer_core::{Attribute, Document, Edit, Node};
use std::collections::{BTreeMap, BTreeSet};

const XML: &str = "http://www.w3.org/XML/1998/namespace";
const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const COMPATIBILITY: &str = "http://schemas.openxmlformats.org/markup-compatibility/2006";

pub(super) fn inherit(
    source: &Document,
    selected: &Node,
    already_copied: bool,
    dictionary: &mut Document,
) -> Result<()> {
    let root = dictionary.ast().root();
    let current = dictionary.ast().node(root).expect("dictionary root");
    let mut namespaces = dictionary.ast().namespaces(root);
    let ancestors = std::iter::successors(Some(selected), |node| {
        node.parent.and_then(|id| source.ast().node(id))
    })
    .skip(usize::from(already_copied))
    .collect::<Vec<_>>();
    let mut edits = Vec::new();
    for (namespace, local, preferred) in [
        (XML, "space", "xml"),
        (XML, "lang", "xml"),
        (XAML, "CompileBindings", "x"),
        (XAML, "DataType", "x"),
    ] {
        if attribute(current, namespace, local).is_some() {
            continue;
        }
        let Some((origin, inherited)) = ancestors
            .iter()
            .find_map(|node| attribute(node, namespace, local).map(|attr| (*node, attr)))
        else {
            continue;
        };
        if local == "DataType" {
            require_type_namespaces(source, origin, &inherited.value, &namespaces)?;
        }
        let prefix = ensure_prefix(&mut namespaces, &mut edits, root, namespace, preferred);
        edits.push(Edit::SetAttribute {
            node: root,
            name: format!("{prefix}:{local}"),
            value: inherited.value.clone(),
        });
    }

    let mut ignored = BTreeSet::new();
    for origin in ancestors {
        add_ignorable(source, origin, &mut ignored);
    }
    if !ignored.is_empty() {
        let authored = attribute(current, COMPATIBILITY, "Ignorable");
        add_ignorable(dictionary, current, &mut ignored);
        let name = authored
            .map(|attribute| attribute.name.clone())
            .unwrap_or_else(|| {
                let prefix = ensure_prefix(&mut namespaces, &mut edits, root, COMPATIBILITY, "mc");
                format!("{prefix}:Ignorable")
            });
        let value = ignored
            .iter()
            .map(|uri| ensure_prefix(&mut namespaces, &mut edits, root, uri, "ignored"))
            .collect::<Vec<_>>()
            .join(" ");
        edits.push(Edit::SetAttribute {
            node: root,
            name,
            value,
        });
    }
    if !edits.is_empty() {
        dictionary
            .apply("Preserve inherited XAML context", edits)
            .map_err(super::error)?;
    }
    Ok(())
}

pub(super) fn attribute<'a>(node: &'a Node, namespace: &str, local: &str) -> Option<&'a Attribute> {
    node.attributes.iter().find(|attribute| {
        attribute.namespace_uri.as_deref() == Some(namespace)
            && attribute.name.rsplit(':').next() == Some(local)
    })
}

pub(super) fn add_ignorable(source: &Document, node: &Node, ignored: &mut BTreeSet<String>) {
    if let Some(ignorable) = attribute(node, COMPATIBILITY, "Ignorable") {
        let namespaces = source.ast().namespaces(node.id);
        ignored.extend(
            ignorable
                .value
                .split_whitespace()
                .filter_map(|prefix| namespaces.get(prefix).cloned()),
        );
    }
}

fn ensure_prefix(
    namespaces: &mut BTreeMap<String, String>,
    edits: &mut Vec<Edit>,
    root: u64,
    namespace: &str,
    preferred: &str,
) -> String {
    if let Some((prefix, _)) = namespaces
        .iter()
        .find(|(prefix, uri)| !prefix.is_empty() && *uri == namespace)
    {
        return prefix.clone();
    }
    let mut prefix = preferred.to_owned();
    while namespaces.contains_key(&prefix) {
        prefix.push('_');
    }
    namespaces.insert(prefix.clone(), namespace.to_owned());
    edits.push(Edit::SetAttribute {
        node: root,
        name: format!("xmlns:{prefix}"),
        value: namespace.to_owned(),
    });
    prefix
}

fn require_type_namespaces(
    source: &Document,
    origin: &Node,
    value: &str,
    destination: &BTreeMap<String, String>,
) -> Result<()> {
    let original = source.ast().namespaces(origin.id);
    let name = value.trim();
    let default_matters = !name.contains(':') || name.starts_with('{');
    require(
        !default_matters || original.get("") == destination.get(""),
        "Declare the inherited data type on the dictionary or template before moving it across a default namespace change",
    )?;
    require(
        original.iter().all(|(prefix, uri)| {
            prefix.is_empty()
                || !name.contains(&format!("{prefix}:"))
                || destination.get(prefix) == Some(uri)
        }),
        "Declare the inherited data type on the dictionary or template before moving it across a shadowed namespace",
    )
}
