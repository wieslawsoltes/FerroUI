//! Element names are symbols. Editing them must retain their references and name scopes.
use crate::DesignerSession;
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use ferroui_designer_language::{name_occurrences, LanguageService, LineIndex};
use std::collections::{BTreeMap, BTreeSet};

const XAML: &str = "http://schemas.microsoft.com/winfx/2006/xaml";
const RENAME_URI: &str = "untitled:/DesignerNameRefactoring.xaml";

/// Plan an atomic name change using the language service's namescope-aware reference grammar.
/// Existing `Name`/XAML directive aliases, quoting, unrelated strings and comments are retained.
/// `None` removes names only when doing so cannot leave a known reference unresolved.
pub fn plan_element_names(
    document: &Document,
    nodes: &[NodeId],
    value: Option<&str>,
) -> Result<Vec<Edit>, String> {
    if !document.is_valid() {
        return Err("Element names require valid current XAML".into());
    }
    if nodes.is_empty() {
        return Err("Select an authored element to edit its name".into());
    }
    if value.is_some_and(|value| !ferroui_designer_language::is_xaml_identifier(value)) {
        return Err("Element names must be nonempty XAML identifiers".into());
    }
    let occurrences = name_occurrences(document);
    let mut language = LanguageService::default();
    language
        .open(RENAME_URI, document.source(), 1)
        .map_err(|error| error.to_string())?;
    let index = LineIndex::new(document.source());
    let mut text_edits = Vec::new();
    let mut attributes = Vec::new();
    let mut targets = BTreeSet::new();
    let mut proposed_names = BTreeSet::new();
    let mut scoped_renames = BTreeMap::<NodeId, BTreeMap<String, String>>::new();
    let mut removals = BTreeMap::<NodeId, BTreeMap<String, String>>::new();
    for id in nodes {
        if !targets.insert(*id) {
            continue;
        }
        let node = document.ast().node(*id).ok_or("Unknown name target")?;
        if node.kind != NodeKind::Element {
            return Err("Only authored objects have element names".into());
        }
        if node
            .children
            .iter()
            .filter_map(|id| document.ast().node(*id))
            .any(|child| {
                child.kind == NodeKind::PropertyElement && child.local_name().ends_with(".Name")
            })
        {
            return Err("Edit object-valued element names in XAML before renaming".into());
        }
        let names = node
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name == "Name"
                    || attribute.name.rsplit(':').next() == Some("Name")
                        && attribute.namespace_uri.as_deref() == Some(XAML)
            })
            .collect::<Vec<_>>();
        if names.len() > 1 {
            return Err("This element has multiple name declarations; resolve them in XAML".into());
        }
        let current = names.first().copied();
        let old = current
            .map(|attribute| attribute.value.as_str())
            .filter(|value| !value.is_empty());
        let scope = document.ast().name_scope(*id);
        if let Some(name) = value {
            if !proposed_names.insert((scope, name.to_owned())) {
                return Err(
                    "Multiple selected elements cannot share a name in the same name scope".into(),
                );
            }
            if occurrences.iter().any(|item| {
                item.declaration && item.scope == scope && item.name == name && item.node != *id
            }) {
                return Err(format!(
                    "An element named {name} already exists in this name scope"
                ));
            }
            if old != Some(name)
                && occurrences
                    .iter()
                    .any(|item| !item.declaration && item.scope == scope && item.name == name)
            {
                return Err(format!(
                    "Naming this element {name} would capture an existing unresolved reference"
                ));
            }
        }
        if let Some(old) = old {
            if occurrences
                .iter()
                .filter(|item| item.declaration && item.scope == scope && item.name == old)
                .count()
                != 1
            {
                return Err(
                    "The existing name is ambiguous; resolve duplicate declarations in XAML".into(),
                );
            }
            if value == Some(old) {
                continue;
            }
            if let Some(name) = value {
                let declaration = current.expect("an existing name has an attribute");
                let changes = language
                    .rename(RENAME_URI, declaration.value_span.start, name)
                    .map_err(|error| error.to_string())?;
                for change in changes.document_changes {
                    if change.uri != RENAME_URI {
                        return Err("Element name references unexpectedly crossed documents".into());
                    }
                    for edit in change.edits {
                        text_edits.push(Edit::Text {
                            span: index.span(edit.range).map_err(|error| error.to_string())?,
                            replacement: edit.new_text,
                        });
                    }
                }
                scoped_renames
                    .entry(scope)
                    .or_default()
                    .insert(old.into(), name.into());
            } else {
                if occurrences
                    .iter()
                    .any(|item| !item.declaration && item.scope == scope && item.name == old)
                {
                    return Err(format!("{old} is referenced; rename it or remove its references before resetting the name"));
                }
                // A temporary distinct name lets the metadata codec identify references
                // without exposing or duplicating its encoded state representation here.
                removals
                    .entry(scope)
                    .or_default()
                    .insert(old.into(), format!("{old}_RemovedNameProbe"));
                attributes.push(Edit::RemoveAttribute {
                    node: *id,
                    name: current.unwrap().name.clone(),
                });
            }
        } else if let Some(name) = value {
            attributes.push(Edit::SetAttribute {
                node: *id,
                name: current
                    .map_or("Name", |attribute| attribute.name.as_str())
                    .into(),
                value: name.into(),
            });
        } else if let Some(attribute) = current {
            attributes.push(Edit::RemoveAttribute {
                node: *id,
                name: attribute.name.clone(),
            });
        }
    }
    for comment in document
        .ast()
        .nodes()
        .filter(|node| node.kind == NodeKind::Comment)
    {
        let scope = document.ast().name_scope(comment.id);
        let original = &document.source()[comment.span.range()];
        let wrapper = format!("<Metadata>{original}</Metadata>");
        if let Some(renames) = removals.get(&scope) {
            let rewritten = ferroui_designer_animation::rewrite_design_metadata(&wrapper, renames)
                .map_err(|error| error.to_string())?;
            if rewritten != wrapper {
                return Err(
                    "The name is used by a visual state; remove its references before resetting it"
                        .into(),
                );
            }
        }
        if let Some(renames) = scoped_renames.get(&scope) {
            let rewritten = ferroui_designer_animation::rewrite_design_metadata(&wrapper, renames)
                .map_err(|error| error.to_string())?;
            let rewritten = rewritten
                .strip_prefix("<Metadata>")
                .and_then(|value| value.strip_suffix("</Metadata>"))
                .ok_or("The state metadata codec changed its wrapper")?;
            if rewritten != original {
                text_edits.push(Edit::Text {
                    span: comment.span,
                    replacement: rewritten.into(),
                });
            }
        }
    }
    text_edits.sort_by_key(|edit| match edit {
        Edit::Text { span, .. } => std::cmp::Reverse(span.start),
        _ => unreachable!(),
    });
    // Text offsets address the accepted source. Node-identity edits follow them so added
    // or removed attributes cannot shift a reference before it is rewritten.
    text_edits.extend(attributes);
    let mut candidate = document.clone();
    candidate
        .apply("Validate element names", text_edits.clone())
        .map_err(|error| error.to_string())?;
    Ok(text_edits)
}

impl DesignerSession {
    /// Rename one authored element and its resolved references in one undo transaction.
    /// This command never records an animation key, even while recording is enabled.
    pub fn rename_element(&mut self, node: NodeId, name: &str) -> Result<(), String> {
        self.change_element_names(&[node], Some(name))
    }

    pub(crate) fn change_element_names(
        &mut self,
        nodes: &[NodeId],
        value: Option<&str>,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let edits = plan_element_names(self.document(), nodes, value)?;
        if !edits.is_empty() {
            self.apply(
                if value.is_some() {
                    "Rename element"
                } else {
                    "Reset element name"
                },
                edits,
            )
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "element_names_tests.rs"]
mod tests;
