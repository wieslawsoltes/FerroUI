//! Review a source-backed resource rename using the language service's scope rules.
use super::super::{
    LanguageService, Location, Occurrence, SourceRenamePlan, SourceReviewRequest, SymbolKind,
    source_review::{check_incomplete, reviewed_document},
};
use crate::{LanguageError, Result, require};
use ferroui_designer_core::{Document, TextPatch};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};

#[path = "resource_theme_family.rs"]
mod theme_family;
use theme_family::ThemeFamily;
pub use theme_family::{ThemeResourceFamily, ThemeResourceMember};

#[cfg(test)]
#[path = "resource_rename_tests.rs"]
mod tests;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceRenameTarget {
    pub name: String,
    pub declaration: Location,
    #[serde(rename = "themeFamily", skip_serializing_if = "Option::is_none")]
    pub theme_family: Option<ThemeResourceFamily>,
}

struct ResourceSelection {
    anchor: (String, Occurrence),
    family: Option<ThemeFamily>,
}

pub use super::super::source_review::SourceRenamePlan as ResourceRenamePlan;

impl LanguageService {
    pub fn resource_rename_target(
        &self,
        uri: &str,
        request: &SourceReviewRequest,
    ) -> Result<ResourceRenameTarget> {
        let target = self.resource_rename_selection(uri, request)?;
        let (uri, symbol) = target.anchor;
        Ok(ResourceRenameTarget {
            name: symbol.name,
            declaration: self.location(&uri, symbol.span)?,
            theme_family: target
                .family
                .map(|family| family.describe(self))
                .transpose()?,
        })
    }

    pub fn plan_resource_rename(
        &mut self,
        uri: &str,
        request: &SourceReviewRequest,
        new_name: &str,
    ) -> Result<ResourceRenamePlan> {
        let uri = self.document_uri(uri)?.into_owned();
        self.refresh_project_index()?;
        let selection = self.resource_rename_selection(&uri, request)?;
        let target = &selection.anchor;
        let name = &target.1.name;
        require(name != new_name, "Enter a different resource key")?;
        for (file, state) in &self.documents {
            if self.workspace_root(file).is_some() && !state.document.is_valid() {
                check_incomplete(file, state, |source| {
                    source.contains(name) || !new_name.is_empty() && source.contains(new_name)
                })?;
            }
        }
        let (edits, theme_family) = if let Some(family) = selection.family {
            family.check_name(self, new_name)?;
            (
                self.rename_declarations(&family.declarations(), new_name)?,
                Some(family.describe(self)?),
            )
        } else {
            (self.rename_declaration(target, new_name)?, None)
        };
        let documents = edits
            .document_changes
            .into_iter()
            .map(|edit| {
                require(
                    self.workspace_root(&edit.uri).is_some(),
                    "Open the workspace containing every file in this resource rename",
                )?;
                let state = self.state(&edit.uri)?;
                let patches = edit
                    .edits
                    .into_iter()
                    .map(|edit| {
                        Ok(TextPatch {
                            span: state.index.span(edit.range)?,
                            replacement: edit.new_text,
                        })
                    })
                    .collect::<Result<_>>()?;
                let reviewed = reviewed_document(&edit.uri, edit.version, &state.index, patches)?;
                Document::new(&reviewed.after)?;
                Ok(reviewed)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut hash = DefaultHasher::new();
        (
            uri,
            request.version,
            request.position.line,
            request.position.character,
            &target.0,
            target.1.span,
            name,
            new_name,
        )
            .hash(&mut hash);
        for document in &documents {
            (
                &document.edit.uri,
                document.edit.version,
                &document.before,
                &document.after,
            )
                .hash(&mut hash);
        }
        if let Some(family) = &theme_family {
            let hash_location = |location: &Location, hash: &mut DefaultHasher| {
                let range = &location.range;
                (
                    &location.uri,
                    range.start.line,
                    range.start.character,
                    range.end.line,
                    range.end.character,
                )
                    .hash(hash);
            };
            hash_location(&family.owner, &mut hash);
            for member in &family.members {
                member.layers.hash(&mut hash);
                hash_location(&member.declaration, &mut hash);
            }
        }
        Ok(SourceRenamePlan {
            name: name.clone(),
            new_name: new_name.into(),
            review: format!("{:016x}", hash.finish()),
            documents,
            unmapped: Vec::new(),
            rust_rename: None,
            theme_family,
        })
    }

    fn resource_rename_selection(
        &self,
        uri: &str,
        request: &SourceReviewRequest,
    ) -> Result<ResourceSelection> {
        let uri = self.document_uri(uri)?;
        let state = self.state(&uri)?;
        require(
            request.version >= 0 && state.opened && state.version == request.version,
            "The XAML source changed before the resource review",
        )?;
        require(
            self.workspace_root(&uri).is_some(),
            "Open the owning workspace before renaming a resource",
        )?;
        require(
            state.document.is_valid(),
            "Repair the current XAML before renaming a resource",
        )?;
        let offset = state.index.offset(request.position)?;
        let ast = state.document.ast();
        let label = ast
            .element_at(offset)
            .and_then(|node| ast.node(node))
            .filter(|node| node.name_span.contains(offset));
        let occurrence = state.symbols.iter().find(|symbol| {
            symbol.kind == SymbolKind::Resource
                && (symbol.span.contains(offset)
                    || symbol.span.end == offset
                    || symbol.declaration && label.is_some_and(|node| node.id == symbol.node))
        });
        let targets = occurrence
            .map(|symbol| self.resolve(&uri, symbol))
            .unwrap_or_default();
        let family = self.resource_theme_family(&targets)?;
        let Some(target) = targets
            .first()
            .filter(|_| targets.len() == 1 || family.is_some())
        else {
            return Err(LanguageError(
                "Choose a resource declaration, one theme family, or a reference with one authored definition".into(),
            ));
        };
        require(
            self.workspace_root(&target.0).is_some(),
            "Open the workspace containing the resource declaration",
        )?;
        Ok(ResourceSelection {
            anchor: target.clone(),
            family,
        })
    }
}
