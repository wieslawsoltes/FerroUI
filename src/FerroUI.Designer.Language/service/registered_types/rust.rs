//! Combine the indexed registry spelling with checked semantic Rust edits.
use super::{replacement, review_type_id, RegisteredTypePlan, RegisteredTypeTarget};
use crate::service::{
    file_moves::{move_drafts, rust_files},
    handlers::read_rust,
    source_review::reviewed_document,
};
use crate::{
    require, FileMoveDraft, LanguageError, LanguageService, LineIndex, RegisteredPropertyRequest,
    Result, ReviewedDocument, RustRenameResolution, RustSymbolRename,
};
use ferroui_build_scan::authored_symbols::TypeNameEdit;
use ferroui_designer_core::{SourceSpan, TextPatch};
use ferroui_designer_project::{
    canonical_file_uri, check_rust_rename_tokens, MarkupSource, RustXamlReferences,
};
use std::{
    borrow::Cow,
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

impl LanguageService {
    pub(super) fn rust_type_review_version(
        &self,
        drafts: &BTreeMap<String, &FileMoveDraft>,
    ) -> Result<u64> {
        let mut stamp = DefaultHasher::new();
        for (uri, (entry, _, _)) in rust_files(self) {
            uri.hash(&mut stamp);
            if let Some(draft) = drafts.get(uri) {
                (draft.version, &draft.source).hash(&mut stamp);
            }
            let metadata =
                std::fs::metadata(&entry.path).map_err(|error| LanguageError(error.to_string()))?;
            (metadata.len(), metadata.modified().ok()).hash(&mut stamp);
        }
        Ok(stamp.finish())
    }

    pub(super) fn plan_type_registration(
        &self,
        target: &RegisteredTypeTarget,
        registration: &MarkupSource,
        new: &str,
        drafts: &BTreeMap<String, &FileMoveDraft>,
    ) -> Result<(Vec<ReviewedDocument>, Option<RustSymbolRename>)> {
        let source = Snapshot::read(&target.rust_uri, drafts)?;
        let edit = registration.type_name_edit(&source.text, new).ok_or_else(|| LanguageError(
            "The Rust registration changed or the name is invalid. Rebuild the project schema before review".into()
        ))?;
        let span = match edit {
            TypeNameEdit::Symbol(span) => span,
            TypeNameEdit::Alias(span) => {
                let edits = vec![replacement(SourceSpan::new(span.start, span.end), new)];
                return Ok((vec![source.document(&target.rust_uri, edits)?], None));
            }
        };
        let declaration = target.rust_declaration.as_deref().ok_or_else(|| {
            LanguageError(
                "Rebuild the project schema to index the authored Rust type declaration".into(),
            )
        })?;
        require(
            self.workspace_root(declaration).is_some(),
            "Open the workspace containing the Rust declaration",
        )?;
        let same_file = declaration == target.rust_uri;
        let declaration_source = if same_file {
            source.shared()
        } else {
            Snapshot::read(declaration, drafts)?
        };
        let range = registration.type_range_in(&declaration_source.text).ok_or_else(|| LanguageError(
            "The indexed Rust type declaration changed. Rebuild the project schema before review".into()
        ))?;
        require(
            declaration_source.text.get(range.clone()) == Some(&target.name),
            "The Rust type has a separate public spelling. Review its explicit markup alias",
        )?;
        let declaration_patch = replacement(SourceSpan::new(range.start, range.end), new);
        let mut patches = vec![replacement(SourceSpan::new(span.start, span.end), new)];
        let request = RustSymbolRename {
            uri: declaration.into(),
            position: LineIndex::new(&declaration_source.text).position(range.start)?,
            old_name: target.name.clone(),
            new_name: new.into(),
        };
        let mut documents = Vec::new();
        if same_file {
            patches.push(declaration_patch);
        } else {
            documents.push(declaration_source.document(declaration, vec![declaration_patch])?);
        }
        documents.push(source.document(&target.rust_uri, patches)?);
        Ok((documents, Some(request)))
    }

    pub fn complete_registered_type_rename(
        &mut self,
        uri: &str,
        request: &RegisteredPropertyRequest,
        new_name: &str,
        drafts: &[FileMoveDraft],
        review: &str,
        resolution: &RustRenameResolution,
    ) -> Result<RegisteredTypePlan> {
        let mut plan = self.plan_registered_type_rename(uri, request, new_name, drafts)?;
        require(
            plan.review == review,
            "The type source changed while resolving Rust references. Review the rename again",
        )?;
        require(
            plan.rust_rename.as_ref() == Some(&resolution.rename),
            "Resolve the Rust declaration from this type review",
        )?;
        let drafts = move_drafts(drafts)?;
        let indexed = rust_files(self);
        let mut verified = false;
        let mut merged = BTreeMap::<String, Vec<TextPatch>>::new();
        for document in &resolution.documents {
            let uri = canonical_file_uri(&document.uri)?;
            require(
                indexed.contains_key(uri.as_str()),
                "The Rust rename reaches outside indexed workspace source",
            )?;
            let source = Snapshot::read(&uri, &drafts)?;
            require(
                source.version == document.version,
                "A Rust buffer changed while resolving the type rename",
            )?;
            let index = LineIndex::new(&source.text);
            for edit in &document.edits {
                let span = index.span(edit.range)?;
                let previous = &source.text[span.range()];
                require(
                    previous.strip_prefix("r#").unwrap_or(previous) == resolution.rename.old_name
                        && edit.new_text == new_name,
                    "rust-analyzer returned edits beyond the selected symbol name",
                )?;
                verified |=
                    uri == resolution.rename.uri && edit.range.start == resolution.rename.position;
                merged
                    .entry(uri.clone())
                    .or_default()
                    .push(replacement(span, new_name));
            }
        }
        require(
            verified,
            "rust-analyzer did not rename the authored type declaration",
        )?;
        for (uri, mut patches) in merged {
            let source = Snapshot::read(&uri, &drafts)?;
            if let Some(index) = plan
                .documents
                .iter()
                .position(|entry| entry.edit.uri == uri)
            {
                let base = plan.documents.remove(index);
                require(
                    base.before == source.text && base.edit.version == source.version,
                    "The Rust registration changed during the type review",
                )?;
                let index = LineIndex::new(&source.text);
                for edit in base.edit.edits {
                    patches.push(replacement(index.span(edit.range)?, &edit.new_text));
                }
            }
            patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
            patches.dedup();
            check_rust_rename_tokens(&source.text, &patches)?;
            let document = source.document(&uri, patches)?;
            RustXamlReferences::parse(&document.after, false)?;
            plan.documents.push(document);
        }
        plan.rust_rename = None;
        plan.review = review_type_id(plan.review, &mut plan.documents, &plan.unmapped, None);
        Ok(plan)
    }
}

struct Snapshot<'a> {
    text: Cow<'a, str>,
    version: i64,
}

impl<'a> Snapshot<'a> {
    fn read(uri: &str, drafts: &BTreeMap<String, &'a FileMoveDraft>) -> Result<Self> {
        if let Some(draft) = drafts.get(uri) {
            return Ok(Self {
                text: Cow::Borrowed(&draft.source),
                version: draft.version,
            });
        }
        Ok(Self {
            text: Cow::Owned(read_rust(uri)?),
            version: -1,
        })
    }

    fn shared(&self) -> Snapshot<'_> {
        Snapshot {
            text: Cow::Borrowed(self.text.as_ref()),
            version: self.version,
        }
    }

    fn document(&self, uri: &str, patches: Vec<TextPatch>) -> Result<ReviewedDocument> {
        reviewed_document(uri, self.version, &LineIndex::new(&self.text), patches)
    }
}
