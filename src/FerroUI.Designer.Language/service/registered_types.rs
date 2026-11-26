//! Review type names with the same identity used by the active Cargo schema.
mod rust;
mod schema;
mod sites;

pub use super::source_review::RegisteredRenamePlan as RegisteredTypePlan;
use super::{
    file_moves::move_drafts, registered_rename::check_incomplete,
    registered_rsx::{MappingRename, RenameKind}, source_review::reviewed_document,
    FileMoveDraft, LanguageService, Location, RegisteredPropertyRequest, ReviewedDocument,
    RustSymbolRename,
};
use crate::{require, LanguageError, Range, Result};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use ferroui_designer_project::{file_uri, MarkupSource};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredTypeTarget {
    pub name: String,
    pub range: Range,
    pub rust_uri: String,
    pub rust_declaration: Option<String>,
}

impl LanguageService {
    pub fn registered_type_target(
        &self,
        uri: &str,
        request: &RegisteredPropertyRequest,
    ) -> Result<RegisteredTypeTarget> {
        Ok(self.type_rename_target(uri, request)?.0)
    }

    pub fn plan_registered_type_rename(
        &mut self,
        uri: &str,
        request: &RegisteredPropertyRequest,
        new_name: &str,
        drafts: &[FileMoveDraft],
    ) -> Result<RegisteredTypePlan> {
        require(
            crate::is_xaml_identifier(new_name) && !new_name.contains('-'),
            "Enter a type identifier made of letters, digits, or underscores",
        )?;
        let uri = self.document_uri(uri)?.into_owned();
        self.refresh_project_index()?;
        let (target, registration) = self.type_rename_target(&uri, request)?;
        require(target.name != new_name, "Enter a different type name")?;
        let drafts = move_drafts(drafts)?;
        let (mut documents, rust_rename) =
            self.plan_type_registration(&target, &registration, new_name, &drafts)?;
        let mut unmapped = Vec::new();
        let mut proposed = BTreeMap::new();
        for (root, project) in &self.package_schemas {
            if let Some(schema) = schema::renamed(&project.effective, &registration, new_name)? {
                proposed.insert(root.as_path(), schema);
            }
        }
        self.review_rsx_mappings(
            MappingRename {
                registration: &registration,
                old: &target.name,
                new: new_name,
                kind: RenameKind::Type { rename_rust: rust_rename.is_some() },
            },
            &drafts,
            &mut documents,
        )?;
        for (source_uri, state) in &self.documents {
            if self.workspace_root(source_uri).is_none() {
                continue;
            }
            let old_short = schema::shortened(&target.name).unwrap_or(&target.name);
            let new_short = schema::shortened(new_name).unwrap_or(new_name);
            let Some((root, project)) = self.project_schema_for(source_uri) else {
                let (root, origin) = self.project_schema_for(&uri).expect("checked project");
                require(
                    !sites::has_reference(&origin.effective, &state.document, root, &registration),
                    format!("Build the Cargo schema for a possible consuming file before review: {source_uri}"),
                )?;
                continue;
            };
            let Some(after) = proposed.get(root) else {
                continue;
            };
            if !state.document.is_valid() {
                check_incomplete(
                    source_uri,
                    state,
                    &[&target.name, new_name, old_short, new_short],
                )?;
                continue;
            }
            let source = state.document.source();
            if !source.contains(old_short) && !source.contains(new_short) && !source.contains('&') {
                continue;
            }
            let (patches, pending) = sites::review(
                &project.effective,
                after,
                &state.document,
                root,
                &registration,
                &target.name,
                new_name,
            )?;
            for span in pending {
                unmapped.push(self.location(source_uri, span)?);
            }
            if !patches.is_empty() {
                let document =
                    reviewed_document(source_uri, state.version, &state.index, patches.clone())?;
                let parsed = Document::new(&document.after)?;
                sites::validate(after, &parsed, root, &registration, &patches)?;
                documents.push(document);
            }
        }
        let rust_version = rust_rename
            .as_ref()
            .map(|_| self.rust_type_review_version(&drafts))
            .transpose()?;
        let review = review_type_id(
            (
                &uri,
                request.version,
                request.position.line,
                request.position.character,
                &target.name,
                new_name,
                rust_version,
            ),
            &mut documents,
            &unmapped,
            rust_rename.as_ref(),
        );
        Ok(RegisteredTypePlan {
            name: target.name,
            new_name: new_name.into(),
            review,
            documents,
            unmapped,
            rust_rename,
            theme_family: None,
        })
    }

    fn type_rename_target(
        &self,
        uri: &str,
        request: &RegisteredPropertyRequest,
    ) -> Result<(RegisteredTypeTarget, Arc<MarkupSource>)> {
        let uri = self.document_uri(uri)?;
        let state = self.state(&uri)?;
        require(
            state.opened && state.version == request.version && request.version >= 0,
            "The XAML source changed before the type review",
        )?;
        require(
            state.document.is_valid(),
            "Repair the current XAML before renaming a type",
        )?;
        require(
            self.workspace_root(&uri).is_some(),
            "Open the owning Cargo project before renaming a type",
        )?;
        let (package, project) = self.project_schema_for(&uri).ok_or_else(|| {
            LanguageError("Build the project XAML schema before renaming a type".into())
        })?;
        let offset = state.index.offset(request.position)?;
        let site = state
            .document
            .ast()
            .element_at(offset)
            .and_then(|id| state.document.ast().node(id))
            .and_then(|node| {
                sites::of_node(&project.effective, &state.document, node)
                    .into_iter()
                    .filter(|site| site.span.contains(offset) || site.span.end == offset)
                    .find_map(|site| {
                        let info = site.resolve(&project.effective, package)?;
                        Some((
                            site.span,
                            info.name.clone(),
                            info.authored.as_ref()?.clone(),
                        ))
                    })
            });
        let (span, name, registration) = site.ok_or_else(|| {
            LanguageError("Place the caret on a Cargo-registered XAML type name".into())
        })?;
        let file = registration.file().ok_or_else(|| {
            LanguageError("Rebuild the schema to locate the Rust type registration".into())
        })?;
        let rust_uri = file_uri(file)?;
        require(
            self.workspace_root(&rust_uri).is_some(),
            "Open the workspace containing the Rust type to rename it",
        )?;
        let rust_declaration = registration.type_file().map(file_uri).transpose()?;
        Ok((
            RegisteredTypeTarget {
                name,
                range: state.index.range(span)?,
                rust_uri,
                rust_declaration,
            },
            registration,
        ))
    }
}

fn review_type_id(
    seed: impl Hash,
    documents: &mut [ReviewedDocument],
    unmapped: &[Location],
    rust: Option<&RustSymbolRename>,
) -> String {
    documents.sort_by(|left, right| left.edit.uri.cmp(&right.edit.uri));
    let mut hash = DefaultHasher::new();
    seed.hash(&mut hash);
    for doc in documents {
        (&doc.edit.uri, doc.edit.version, &doc.before, &doc.after).hash(&mut hash);
    }
    for location in unmapped {
        (
            &location.uri,
            location.range.start.line,
            location.range.start.character,
            location.range.end.line,
            location.range.end.character,
        )
            .hash(&mut hash);
    }
    if let Some(rust) = rust {
        (
            &rust.uri,
            rust.position.line,
            rust.position.character,
            &rust.old_name,
            &rust.new_name,
        )
            .hash(&mut hash);
    }
    format!("{:016x}", hash.finish())
}

fn replacement(span: SourceSpan, value: &str) -> TextPatch {
    TextPatch {
        span,
        replacement: value.into(),
    }
}
