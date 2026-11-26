//! Reviewed moves of one authored resource between XAML dictionaries.
use super::{
    LanguageService, Location, Occurrence, State, SymbolKind,
    source_review::{ReviewedDocument, SourceReviewRequest, reviewed_document},
};
use crate::{LanguageError, Range, Result, require};
use ferroui_designer_core::{Node, NodeKind, TextPatch};
use ferroui_designer_tools::{
    AuthoredResourceScope, component_theme_key, resource_move_entry, resource_move_scope,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

mod graph;
mod source;
#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceMoveTarget {
    pub name: String,
    pub declaration: Location,
    pub destinations: Vec<ResourceMoveDestination>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceMoveDestination {
    pub uri: String,
    pub request: SourceReviewRequest,
    pub range: Range,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceMovePlan {
    pub name: String,
    pub from: Location,
    pub to: Location,
    pub review: String,
    pub linked_destination: bool,
    pub documents: Vec<ReviewedDocument>,
}

struct Entry {
    uri: String,
    declaration: Occurrence,
    scope: AuthoredResourceScope,
}

impl LanguageService {
    pub fn resource_move_target(
        &mut self,
        uri: &str,
        request: &SourceReviewRequest,
    ) -> Result<ResourceMoveTarget> {
        self.refresh_project_index()?;
        let entry = self.resource_move_origin(uri, request)?;
        let root = self.workspace_root(&entry.uri);
        let mut destinations = Vec::new();
        for (file, state) in &self.documents {
            if self.workspace_root(file) != root || !state.document.is_valid() {
                continue;
            }
            let ast = state.document.ast();
            let mut seen = BTreeSet::new();
            for node in ast
                .nodes_in_source_order()
                .filter(|node| scope_syntax(node))
            {
                let Ok(Some(scope)) = resource_move_scope(&state.document, node.id) else {
                    continue;
                };
                if !seen.insert(scope.container) || !entry.is_destination(file, state, scope) {
                    continue;
                }
                let container = ast
                    .node(scope.container)
                    .expect("authored resource container");
                let name = ast.node(scope.owner).expect("resource owner");
                let theme = std::iter::successors(Some(container), |node| {
                    node.parent.and_then(|id| ast.node(id))
                })
                .find_map(|node| {
                    let parent = node.parent.and_then(|id| ast.node(id))?;
                    super::dictionary_property(&state.document, parent, "ThemeDictionaries")
                        .then(|| component_theme_key(&state.document, node))
                        .flatten()
                });
                let label = if let Some(theme) = theme {
                    format!("{theme} dictionary")
                } else if name.local_name() == "ResourceDictionary" {
                    name.xaml_name().unwrap_or("Resource dictionary").into()
                } else {
                    format!(
                        "{} resources",
                        name.xaml_name().unwrap_or_else(|| name.local_name())
                    )
                };
                let range = state.index.range(container.name_span)?;
                destinations.push(ResourceMoveDestination {
                    uri: file.clone(),
                    request: SourceReviewRequest {
                        version: state.version,
                        position: range.start,
                    },
                    range,
                    label,
                });
            }
        }
        Ok(ResourceMoveTarget {
            name: entry.declaration.name.clone(),
            declaration: self.location(&entry.uri, entry.declaration.span)?,
            destinations,
        })
    }

    pub fn plan_resource_move(
        &mut self,
        uri: &str,
        request: &SourceReviewRequest,
        destination_uri: &str,
        destination: &SourceReviewRequest,
    ) -> Result<ResourceMovePlan> {
        self.refresh_project_index()?;
        let entry = self.resource_move_origin(uri, request)?;
        let destination_uri = self.document_uri(destination_uri)?.into_owned();
        require(
            self.workspace_root(&entry.uri) == self.workspace_root(&destination_uri),
            "Choose a destination in the resource's indexed workspace",
        )?;
        let target = self.state(&destination_uri)?;
        require(
            target.version == destination.version && target.document.is_valid(),
            "The destination changed or is incomplete. Select the dictionary again",
        )?;
        let anchor = target
            .document
            .ast()
            .element_at(target.index.offset(destination.position)?)
            .ok_or_else(|| LanguageError("Choose an authored resource dictionary".into()))?;
        let scope = resource_move_scope(&target.document, anchor)
            .map_err(tool_error)?
            .ok_or_else(|| LanguageError("Choose a Resources property or dictionary".into()))?;
        require(
            entry.is_destination(&destination_uri, target, scope),
            "Choose a different dictionary outside the moved subtree",
        )?;
        let draft = source::Draft::prepare(self, &entry, &destination_uri, scope)?;
        graph::require_relocated_includes_acyclic(&draft)?;

        let (draft, linked_destination) = match graph::validate(self, &draft, &entry, false) {
            Ok(()) => (draft, false),
            Err(original) if draft.can_link(&entry, scope) => {
                graph::check_destination_link(&draft, &entry)?;
                let mut last_error = original;
                let mut verified = None;
                for highest_priority in [false, true] {
                    let linked = draft.link(self, &entry, highest_priority)?;
                    match graph::validate(self, &linked, &entry, true) {
                        Ok(()) => {
                            verified = Some(linked);
                            break;
                        }
                        Err(error) => last_error = error,
                    }
                }
                (verified.ok_or(last_error)?, true)
            }
            Err(error) => return Err(error),
        };
        let documents = draft
            .documents()
            .map(|(file, document)| {
                let state = self.state(file)?;
                let patch = TextPatch::between(state.document.source(), document.source())
                    .ok_or_else(|| {
                        LanguageError("The resource move produced no source change".into())
                    })?;
                reviewed_document(file, state.version, &state.index, vec![patch])
            })
            .collect::<Result<Vec<_>>>()?;
        let from = self.location(&entry.uri, entry.declaration.span)?;
        let target_state = draft.service.state(&destination_uri)?;
        let moved = target_state
            .symbols
            .iter()
            .find(|symbol| {
                symbol.declaration
                    && symbol.kind == SymbolKind::Resource
                    && symbol.node == draft.destination_node
                    && symbol.name == entry.declaration.name
            })
            .ok_or_else(|| LanguageError("The moved resource has no authored key".into()))?;
        let to = draft.service.location(&destination_uri, moved.span)?;
        let mut digest = DefaultHasher::new();
        (
            &entry.uri,
            entry.declaration.span,
            &destination_uri,
            scope.container,
            linked_destination,
        )
            .hash(&mut digest);
        for document in &documents {
            (
                &document.edit.uri,
                document.edit.version,
                &document.before,
                &document.after,
            )
                .hash(&mut digest);
        }
        Ok(ResourceMovePlan {
            name: entry.declaration.name,
            from,
            to,
            review: format!("{:016x}", digest.finish()),
            linked_destination,
            documents,
        })
    }

    fn resource_move_origin(&self, uri: &str, request: &SourceReviewRequest) -> Result<Entry> {
        let identity = self.document_uri(uri)?;
        let state = self.state(&identity)?;
        let root = self.workspace_root(&identity).ok_or_else(|| {
            LanguageError("Open the owning workspace before moving a resource".into())
        })?;
        require(
            state.opened && state.version == request.version && state.document.is_valid(),
            "Use the current XAML buffer before reviewing a resource move",
        )?;
        let offset = state.index.offset(request.position)?;
        let occurrence = state.symbols.iter().find(|symbol| {
            symbol.kind == SymbolKind::Resource
                && (symbol.span.contains(offset) || symbol.span.end == offset)
        });
        let (uri, declaration) = if let Some(occurrence) = occurrence {
            if occurrence.declaration {
                (identity.into_owned(), occurrence.clone())
            } else {
                graph::reference_target(self, &identity, occurrence)?
            }
        } else {
            let ast = state.document.ast();
            let anchor = ast.element_at(offset).and_then(|node| ast.node(node));
            let declaration =
                std::iter::successors(anchor, |node| node.parent.and_then(|id| ast.node(id)))
                    .find_map(|node| {
                        state.symbols.iter().find(|symbol| {
                            symbol.declaration
                                && symbol.kind == SymbolKind::Resource
                                && symbol.node == node.id
                        })
                    })
                    .ok_or_else(|| {
                        LanguageError("Put the caret on a resource or its literal reference".into())
                    })?;
            (identity.into_owned(), declaration.clone())
        };
        require(
            self.workspace_root(&uri) == Some(root),
            "The resource source is outside this workspace",
        )?;
        let scope = resource_move_entry(&self.state(&uri)?.document, declaration.node)
            .map_err(tool_error)?;
        Ok(Entry {
            uri,
            declaration,
            scope,
        })
    }
}

impl Entry {
    fn is_destination(&self, file: &str, state: &State, scope: AuthoredResourceScope) -> bool {
        file != self.uri
            || (scope.container != self.scope.container
                && scope.container != self.declaration.node
                && !state
                    .document
                    .ast()
                    .is_descendant_of(scope.container, self.declaration.node))
    }
}

fn scope_syntax(node: &Node) -> bool {
    match node.kind {
        NodeKind::Element => node.local_name() == "ResourceDictionary",
        NodeKind::PropertyElement => node.local_name().ends_with(".Resources"),
        _ => false,
    }
}

fn tool_error(error: ferroui_designer_tools::ToolError) -> LanguageError {
    LanguageError(error.to_string())
}
