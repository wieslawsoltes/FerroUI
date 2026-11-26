//! Source operations for moving a resource. The project planner proves lookup equivalence.
use super::{
    FRAMEWORK, Result, ToolError, authoring::resource_container, component_fragment,
    dictionary_context, element, error, escape, extraction::framework_prefix, literal_key,
    property_elements, require, resource_owner, valid_document,
};
use ferroui_designer_core::{Document, Edit, Node, NodeId, NodeKind, SourceSpan, TextPatch};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AuthoredResourceScope {
    /// Add or remove entries in this syntax container.
    pub container: NodeId,
    /// Start resource lookup here. A shorthand Resources property uses its owning control.
    pub owner: NodeId,
}

pub fn resource_move_scope(
    document: &Document,
    anchor: NodeId,
) -> Result<Option<AuthoredResourceScope>> {
    valid_document(document)?;
    resource_container(document, anchor)?
        .map(|container| {
            let node = element(document, container)?;
            let owner = if node.kind == NodeKind::PropertyElement {
                node.parent
                    .ok_or_else(|| ToolError("The Resources property has no owner".into()))?
            } else {
                node.id
            };
            Ok(AuthoredResourceScope { container, owner })
        })
        .transpose()
}

pub fn resource_move_entry(document: &Document, node: NodeId) -> Result<AuthoredResourceScope> {
    let entry = element(document, node)?;
    require(
        entry.kind == NodeKind::Element && literal_key(entry).is_some(),
        "Choose an entry with a literal x:Key",
    )?;
    let anchor = resource_owner(document, node).ok_or_else(|| {
        ToolError("Choose an entry in an authored Resources property or dictionary".into())
    })?;
    let scope = resource_move_scope(document, anchor)?
        .ok_or_else(|| ToolError("The resource has no authored dictionary".into()))?;
    require(
        entry.parent == Some(scope.container),
        "Choose a direct entry in the dictionary",
    )?;
    Ok(scope)
}

pub fn resource_move_fragment(
    source: &Document,
    node: NodeId,
    destination: &Document,
    scope: AuthoredResourceScope,
) -> Result<Document> {
    let original = element(source, node)?;
    let container = element(destination, scope.container)?;
    let mut fragment = Document::new(component_fragment(source, node)?).map_err(error)?;
    dictionary_context::inherit(source, original, true, &mut fragment)?;
    let root = fragment.ast().root();
    if !source.ast().namespaces(node).contains_key("")
        && destination.ast().namespaces(container.id).contains_key("")
    {
        fragment
            .apply(
                "Preserve default namespace",
                vec![Edit::SetAttribute {
                    node: root,
                    name: "xmlns".into(),
                    value: String::new(),
                }],
            )
            .map_err(error)?;
    }
    let authored = element(&fragment, root)?;
    for (namespace, name) in [
        ("http://www.w3.org/XML/1998/namespace", "space"),
        ("http://www.w3.org/XML/1998/namespace", "lang"),
        (super::XAML, "DataType"),
        (super::XAML, "CompileBindings"),
    ] {
        let inherited = ancestors(destination, container)
            .any(|node| dictionary_context::attribute(node, namespace, name).is_some());
        require(
            !inherited || dictionary_context::attribute(authored, namespace, name).is_some(),
            format!("The destination would add inherited {name} to the moved resource"),
        )?;
    }
    let mut original_ignorable = BTreeSet::new();
    let mut destination_ignorable = BTreeSet::new();
    for node in ancestors(source, original) {
        dictionary_context::add_ignorable(source, node, &mut original_ignorable);
    }
    for node in ancestors(destination, container) {
        dictionary_context::add_ignorable(destination, node, &mut destination_ignorable);
    }
    require(
        destination_ignorable.is_subset(&original_ignorable),
        "The destination would ignore additional namespaces in the moved resource",
    )?;
    Ok(fragment)
}

fn ancestors<'a>(document: &'a Document, start: &'a Node) -> impl Iterator<Item = &'a Node> {
    std::iter::successors(Some(start), |node| {
        node.parent.and_then(|id| document.ast().node(id))
    })
}

/// Keep the previous resource owner connected to a standalone destination dictionary.
/// Shorthand Resources entries gain an explicit dictionary, retaining their original source.
pub fn link_resource_dictionary(
    document: &mut Document,
    scope: AuthoredResourceScope,
    reference: &str,
    highest_priority: bool,
) -> Result<()> {
    let container = element(document, scope.container)?;
    let (prefix, declare) = framework_prefix(&document.ast().namespaces(container.id));
    let declaration = if declare {
        format!(" xmlns:{prefix}=\"{FRAMEWORK}\"")
    } else {
        String::new()
    };
    let qualifier = if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}:")
    };
    let source = escape(reference);
    if container.kind != NodeKind::PropertyElement {
        let merged = property_elements(document, container, "MergedDictionaries")?;
        let include = format!("<{qualifier}ResourceInclude{declaration} Source=\"{source}\" />");
        let (parent, index, fragment) = if let [merged] = merged.as_slice() {
            let index = if highest_priority {
                document.ast().element_children(merged.id).len()
            } else {
                0
            };
            (merged.id, index, include)
        } else {
            require(
                merged.is_empty(),
                "The dictionary has several merged collections",
            )?;
            let name = format!("{}.MergedDictionaries", container.name);
            (container.id, 0, format!("<{name}>{include}</{name}>"))
        };
        document
            .apply(
                "Link the destination resource dictionary",
                vec![Edit::InsertChild {
                    parent,
                    index,
                    source: fragment,
                }],
            )
            .map_err(error)?;
        return Ok(());
    }
    let end = container
        .closing_span
        .ok_or_else(|| ToolError("Choose an authored Resources property".into()))?
        .start;
    let start = container.opening_span.end;
    let name = format!("{qualifier}ResourceDictionary");
    let header = format!(
        "<{name}{declaration}><{name}.MergedDictionaries><{qualifier}ResourceInclude Source=\"{source}\" /></{name}.MergedDictionaries>"
    );
    let footer = format!("</{name}>");
    let insert = |offset, replacement| TextPatch {
        span: SourceSpan::new(offset, offset),
        replacement,
    };
    let patches = if start == end {
        vec![insert(start, header + &footer)]
    } else {
        vec![insert(start, header), insert(end, footer)]
    };
    document
        .apply_text_patches(
            document.revision(),
            "Link the destination resource dictionary",
            patches,
        )
        .map_err(error)?;
    Ok(())
}
