//! Lossless source plans for moving an authored resource scope to a sibling dictionary.
use super::{
    FRAMEWORK, component_fragment, directive, element, escape, external_dictionary, framework,
    namespace_attribute, property_elements, references::resource_uses, valid_document,
};
use crate::{Result, ToolError, error, require};
use ferroui_designer_core::{Document, Node, NodeId, NodeKind, SourceSpan, TextPatch};
use std::collections::BTreeMap;

use super::dictionary_context as context;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionarySite {
    pub node: NodeId,
    pub span: SourceSpan,
    pub entries: usize,
    kind: SiteKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SiteKind {
    Shorthand,
    Explicit,
    Merged,
}

pub struct DictionaryExtraction {
    pub patch: TextPatch,
    pub dictionary: Document,
    /// Literal static resource uses in the new document, including object forms.
    pub static_resources: Vec<(NodeId, String)>,
}

/// A source selection can be the owner, the dictionary, or one of its entries.
pub fn dictionary_extraction_site(
    document: &Document,
    anchor: NodeId,
) -> Result<Option<DictionarySite>> {
    valid_document(document)?;
    let selected = element(document, anchor)?;
    let ast = document.ast();
    let mut current = Some(selected);
    while let Some(node) = current {
        let parent = node.parent.and_then(|id| ast.node(id));
        if is_dictionary(node) && parent.is_some_and(|parent| is_merge_property(document, parent)) {
            return site(document, node, SiteKind::Merged).map(Some);
        }
        if let Some(owner) = parent.filter(|_| {
            node.kind == NodeKind::PropertyElement && node.local_name().ends_with(".Resources")
        }) {
            if property_elements(document, owner, "Resources")?.contains(&node) {
                return property_site(document, node).map(Some);
            }
        }
        current = parent;
    }
    property_elements(document, selected, "Resources")?
        .first()
        .map(|property| property_site(document, property))
        .transpose()
}

fn property_site(document: &Document, property: &Node) -> Result<DictionarySite> {
    let children = document.ast().element_children(property.id);
    let dictionaries = children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .filter(|node| is_dictionary(node))
        .collect::<Vec<_>>();
    if let [dictionary] = dictionaries.as_slice() {
        require(
            children.len() == 1,
            "Choose a Resources property with a single explicit dictionary",
        )?;
        require_element_content(document, property)?;
        return site(document, dictionary, SiteKind::Explicit);
    }
    require(
        dictionaries.is_empty(),
        "Choose a Resources property with a single explicit dictionary",
    )?;
    site(document, property, SiteKind::Shorthand)
}

fn site(document: &Document, node: &Node, kind: SiteKind) -> Result<DictionarySite> {
    require_element_content(document, node)?;
    let elements = document.ast().element_children(node.id);
    require(!elements.is_empty(), "The resource dictionary is empty")?;
    if kind != SiteKind::Shorthand {
        require(
            !external_dictionary(document, node),
            "The dictionary already has an external source",
        )?;
        require(
            directive(node, "Key").is_none(),
            "Choose a dictionary outside a keyed theme slot",
        )?;
        if let [only] = elements.as_slice() {
            let child = document.ast().node(*only).unwrap();
            if is_merge_property(document, child) {
                let merged = document.ast().element_children(child.id);
                if let [entry] = merged.as_slice() {
                    let include = document.ast().node(*entry).unwrap();
                    require(
                        !(framework(include)
                            && matches!(
                                include.local_name(),
                                "ResourceInclude" | "MergeResourceInclude"
                            )),
                        "This dictionary already refers to a separate file",
                    )?;
                }
            }
        }
    }
    Ok(DictionarySite {
        node: node.id,
        span: node.span,
        entries: elements.len(),
        kind,
    })
}

fn is_dictionary(node: &Node) -> bool {
    node.kind == NodeKind::Element && node.local_name() == "ResourceDictionary" && framework(node)
}

fn is_merge_property(document: &Document, node: &Node) -> bool {
    node.kind == NodeKind::PropertyElement
        && node.local_name() == "ResourceDictionary.MergedDictionaries"
        && node
            .parent
            .and_then(|id| document.ast().node(id))
            .is_some_and(|parent| {
                is_dictionary(parent) && parent.namespace_uri == node.namespace_uri
            })
}

fn require_element_content(document: &Document, node: &Node) -> Result<()> {
    require(
        node.children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .all(|child| {
                child.is_element()
                    || match child.kind {
                        NodeKind::Text => {
                            child.text.as_deref().unwrap_or_default().trim().is_empty()
                        }
                        NodeKind::Comment => true,
                        _ => false,
                    }
            }),
        "The resource dictionary has non-element content",
    )
}

pub fn plan_dictionary_extraction(
    document: &Document,
    target: &DictionarySite,
    reference: &str,
) -> Result<DictionaryExtraction> {
    valid_document(document)?;
    let node = element(document, target.node)?;
    require(
        &site(document, node, target.kind)? == target,
        "The authored dictionary changed",
    )?;
    let scope = document.ast().namespaces(node.id);
    let (prefix, declare) = framework_prefix(&scope);
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
    let dictionary_tag = format!("{qualifier}ResourceDictionary");
    let source = document.source();
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let (span, extracted) = if target.kind == SiteKind::Shorthand {
        let end = node
            .closing_span
            .ok_or_else(|| ToolError("The Resources property has no closing tag".into()))?
            .start;
        let body = SourceSpan::new(node.opening_span.end, end);
        let namespaces = scope
            .iter()
            .filter(|(prefix, _)| *prefix != "xml")
            .map(|(prefix, uri)| format!(" {}=\"{}\"", namespace_attribute(prefix), escape(uri)))
            .collect::<String>();
        (
            body,
            format!(
                "<{dictionary_tag}{declaration}{namespaces}>{}</{dictionary_tag}>",
                &source[body.range()]
            ),
        )
    } else {
        (node.span, component_fragment(document, node.id)?)
    };
    let parent_scope = if target.kind == SiteKind::Shorthand {
        scope
    } else {
        document.ast().namespaces(
            node.parent
                .ok_or_else(|| ToolError("Choose an inline dictionary".into()))?,
        )
    };
    let include = include_markup(
        &parent_scope,
        &escape(reference),
        target.kind,
        source,
        node,
        newline,
    );
    let patch = TextPatch {
        span,
        replacement: include,
    };
    Document::new(TextPatch::apply_all(source, vec![patch.clone()]).map_err(error)?)
        .map_err(error)?;
    let mut dictionary = Document::new(format!("{extracted}{newline}")).map_err(error)?;
    context::inherit(
        document,
        node,
        target.kind != SiteKind::Shorthand,
        &mut dictionary,
    )?;
    let mut static_resources = Vec::new();
    for child in dictionary.ast().elements() {
        static_resources.extend(
            resource_uses(&dictionary, child)?
                .into_iter()
                .filter(|(_, dynamic, _)| !dynamic)
                .map(|(_, _, key)| (child.id, key)),
        );
    }
    Ok(DictionaryExtraction {
        patch,
        dictionary,
        static_resources,
    })
}

pub(super) fn framework_prefix(namespaces: &BTreeMap<String, String>) -> (String, bool) {
    if let Some((prefix, _)) = namespaces.iter().find(|(_, uri)| *uri == FRAMEWORK) {
        return (prefix.clone(), false);
    }
    let mut prefix = "ferro".to_owned();
    while namespaces.contains_key(&prefix) {
        prefix.push('_');
    }
    (prefix, true)
}

fn include_markup(
    scope: &BTreeMap<String, String>,
    reference: &str,
    kind: SiteKind,
    source: &str,
    node: &Node,
    newline: &str,
) -> String {
    let (prefix, declare) = framework_prefix(scope);
    let declaration = if declare {
        format!(" xmlns:{prefix}=\"{FRAMEWORK}\"")
    } else {
        String::new()
    };
    let prefix = if prefix.is_empty() {
        String::new()
    } else {
        format!("{prefix}:")
    };
    let include = format!("{prefix}ResourceInclude");
    if kind == SiteKind::Merged {
        return format!("<{include}{declaration} Source=\"{reference}\" />");
    }
    let dictionary = format!("{prefix}ResourceDictionary");
    let merged = format!("{dictionary}.MergedDictionaries");
    if !source[node.span.range()].contains('\n') {
        return format!(
            "<{dictionary}{declaration}><{merged}><{include} Source=\"{reference}\" /></{merged}></{dictionary}>"
        );
    }
    let indent = source[..node.span.start]
        .rsplit('\n')
        .next()
        .unwrap_or_default();
    let indent = if indent.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
        indent
    } else {
        ""
    };
    let step = if indent.contains('\t') { "\t" } else { "  " };
    let wrapper = if kind == SiteKind::Shorthand {
        format!("{indent}{step}")
    } else {
        indent.to_owned()
    };
    let body = format!(
        "<{dictionary}{declaration}>{newline}{wrapper}{step}<{merged}>{newline}{wrapper}{step}{step}<{include} Source=\"{reference}\" />{newline}{wrapper}{step}</{merged}>{newline}{wrapper}</{dictionary}>"
    );
    if kind == SiteKind::Shorthand {
        format!("{newline}{wrapper}{body}{newline}{indent}")
    } else {
        body
    }
}
