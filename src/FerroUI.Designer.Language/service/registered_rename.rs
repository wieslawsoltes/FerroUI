//! Review a public XAML property rename against the selected Cargo registrations.
mod schema;
mod sites;

use super::{
    handlers::read_rust, source_review::reviewed_document, HandlerDraft, LanguageService, Location,
    ReviewedDocument, State,
};
use crate::{require, LanguageError, LineIndex, Position, Range, Result};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use ferroui_designer_project::{canonical_file_uri, file_path, file_uri, MarkupSource};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
    sync::Arc,
};
pub use super::source_review::RegisteredRenamePlan as RegisteredPropertyPlan;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct RegisteredPropertyRequest {
    pub version: i64,
    pub position: Position,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredPropertyTarget {
    pub name: String,
    pub range: Range,
    pub rust_uri: String,
}

impl LanguageService {
    pub fn registered_property_target(
        &self,
        uri: &str,
        request: &RegisteredPropertyRequest,
    ) -> Result<RegisteredPropertyTarget> {
        Ok(self.property_rename_target(uri, request)?.0)
    }

    pub fn plan_registered_property_rename(
        &mut self,
        uri: &str,
        request: &RegisteredPropertyRequest,
        new_name: &str,
        draft: Option<&HandlerDraft>,
    ) -> Result<RegisteredPropertyPlan> {
        require(
            crate::is_xaml_identifier(new_name) && !new_name.contains('-'),
            "Enter a property identifier made of letters, digits, or underscores",
        )?;
        let uri = self.document_uri(uri)?.into_owned();
        self.refresh_project_index()?;
        let (target, registration) = self.property_rename_target(&uri, request)?;
        require(target.name != new_name, "Enter a different property name")?;
        let mut documents = vec![rust_document(&target, &registration, new_name, draft)?];
        let mut unmapped = Vec::new();
        let mut proposed = BTreeMap::new();
        for (root, project) in &self.package_schemas {
            if let Some(schema) =
                schema::proposed_schema(project, &registration, &target.name, new_name)?
            {
                proposed.insert(root.as_path(), schema);
            }
        }
        let origin_schema = self.schema_for(&uri);
        for (source_uri, state) in &self.documents {
            if self.workspace_root(source_uri).is_none() {
                continue;
            }
            let package = self.project_schema_for(source_uri);
            let Some(after_schema) = package.and_then(|(root, _)| proposed.get(root)) else {
                if package.is_none() {
                    require(
                        !sites::has_mapped_reference(origin_schema, &state.document, &target.name, &registration),
                        format!(
                            "Build the Cargo schema for a possible consuming file before review: {source_uri}"
                        ),
                    )?;
                }
                continue;
            };
            let before_schema = self.schema_for(source_uri);
            if !state.document.is_valid() {
                check_incomplete(source_uri, state, &[&target.name, new_name])?;
                continue;
            }
            let source = state.document.source();
            if !sites::mentions(source, &target.name)
                && !sites::mentions(source, new_name)
                && !source.contains('&')
            {
                continue;
            }
            let (mut patches, unverified) = sites::review_uses(
                &state.document,
                before_schema,
                after_schema,
                &registration,
                &target.name,
                new_name,
            )?;
            for span in unverified {
                unmapped.push(self.location(source_uri, span)?);
            }
            if !patches.is_empty() {
                patches.sort_by_key(|patch| patch.span.start);
                let document =
                    reviewed_document(source_uri, state.version, &state.index, patches.clone())?;
                sites::validate_references(
                    after_schema,
                    &Document::new(&document.after)?,
                    &patches,
                    &registration,
                    new_name,
                )?;
                documents.push(document);
            }
        }
        documents.sort_by(|left, right| left.edit.uri.cmp(&right.edit.uri));
        let review = review_id(&uri, request, &target, new_name, &documents, &unmapped);
        Ok(RegisteredPropertyPlan {
            name: target.name,
            new_name: new_name.into(),
            review,
            documents,
            unmapped,
            rust_rename: None,
        })
    }

    fn property_rename_target(
        &self,
        uri: &str,
        request: &RegisteredPropertyRequest,
    ) -> Result<(RegisteredPropertyTarget, Arc<MarkupSource>)> {
        let uri = self.document_uri(uri)?;
        let uri = uri.as_ref();
        let state = self.state(uri)?;
        require(
            request.version >= 0 && state.opened && state.version == request.version,
            "The XAML source changed before the property review",
        )?;
        require(
            state.document.is_valid(),
            "Repair the current XAML before renaming a property",
        )?;
        require(
            self.workspace_root(uri).is_some(),
            "Open the owning Cargo project before renaming a property",
        )?;
        let offset = state.index.offset(request.position)?;
        let definition = state
            .document
            .ast()
            .element_at(offset)
            .and_then(|id| state.document.ast().node(id))
            .and_then(|node| {
                sites::of_node(self.schema_for(uri), &state.document, node)
                    .into_iter()
                    .find(|site| site.span.contains(offset) && site.property.authored.is_some())
            });
        let site = definition.ok_or_else(|| {
            LanguageError(
                "Place the caret on a property or typed binding member in a built Cargo schema"
                    .into(),
            )
        })?;
        let registration = site.property.authored.expect("checked source registration");
        let file = registration.file().ok_or_else(|| {
            LanguageError("Rebuild the schema to locate the Rust registration".into())
        })?;
        let rust_uri = file_uri(&file)?;
        require(
            self.workspace_root(&rust_uri).is_some(),
            "Open the workspace containing the Rust registration to rename it",
        )?;
        Ok((
            RegisteredPropertyTarget {
                name: site.property.name,
                range: state.index.range(site.span)?,
                rust_uri,
            },
            registration,
        ))
    }
}

fn rust_document(
    target: &RegisteredPropertyTarget,
    registration: &MarkupSource,
    new_name: &str,
    draft: Option<&HandlerDraft>,
) -> Result<ReviewedDocument> {
    let (source, version) = if let Some(draft) = draft {
        require(
            canonical_file_uri(&draft.uri)? == target.rust_uri
                && draft.version >= 0
                && draft.source.len() <= super::handlers::MAX_RUST_BYTES,
            "The open Rust draft does not match the property registration",
        )?;
        (
            std::borrow::Cow::Borrowed(draft.source.as_str()),
            draft.version,
        )
    } else {
        (std::borrow::Cow::Owned(read_rust(&target.rust_uri)?), -1)
    };
    let span = registration.property_name_range(&source, new_name).ok_or_else(|| LanguageError(
        "The current Rust source needs a unique direct registration and a valid name. Rebuild the schema if the declaration changed".into()
    ))?;
    reviewed_document(
        &target.rust_uri,
        version,
        &LineIndex::new(source.as_ref()),
        vec![TextPatch {
            span: SourceSpan::new(span.start, span.end),
            replacement: new_name.into(),
        }],
    )
}

pub(super) fn check_incomplete(uri: &str, state: &State, names: &[&str]) -> Result<()> {
    let mentions = |source: &str| names.iter().any(|name| sites::mentions(source, name));
    let relevant = |source: &str| {
        mentions(source)
            || source.contains('&')
                && ferroui_designer_core::XmlText::text_node(source, SourceSpan::new(0, source.len()))
                    .is_some_and(|text| mentions(text.value()))
    };
    let path = file_path(uri)?;
    let draft_mentions = relevant(state.document.source()) || relevant(state.document.valid_source());
    let disk_mentions = !draft_mentions
        && state.opened
        && path.is_file()
        && read_xaml(&path).map(|source| relevant(&source))?;
    require(
        !draft_mentions && !disk_mentions,
        format!("Repair the incomplete XAML that may use the name before reviewing: {uri}"),
    )
}

fn read_xaml(path: &Path) -> Result<String> {
    use std::io::Read;
    const MAX: u64 = 16 * 1024 * 1024;
    let mut source = String::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(MAX + 1).read_to_string(&mut source))
        .map_err(|error| LanguageError(error.to_string()))?;
    require(source.len() as u64 <= MAX, "Indexed file exceeds 16 MiB")?;
    Ok(source)
}

fn review_id(
    uri: &str,
    request: &RegisteredPropertyRequest,
    target: &RegisteredPropertyTarget,
    new: &str,
    documents: &[ReviewedDocument],
    unmapped: &[Location],
) -> String {
    let mut hash = DefaultHasher::new();
    (
        uri,
        request.version,
        request.position.line,
        request.position.character,
        &target.name,
        new,
    )
        .hash(&mut hash);
    for document in documents {
        (
            &document.edit.uri,
            document.edit.version,
            &document.before,
            &document.after,
        )
            .hash(&mut hash);
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
    format!("{:016x}", hash.finish())
}
