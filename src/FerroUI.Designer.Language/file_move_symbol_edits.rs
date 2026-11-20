//! Merge semantic Rust rename patches into the original source transaction.
use super::{
    FileMoveDraft, FileMovePlan, GeneratedRustRename, move_document, move_drafts, rust::rust_files,
};
use crate::{DocumentEdits, LanguageError, LanguageService, LineIndex, Result, require};
use ferroui_designer_core::TextPatch;
use ferroui_designer_project::{
    RustXamlReferences, canonical_file_uri, check_rust_rename_tokens, file_path,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Debug, Deserialize)]
pub struct FileMoveRustResolution {
    pub rename: GeneratedRustRename,
    pub documents: Vec<DocumentEdits>,
}

impl LanguageService {
    /// Replan against current buffers after the host requests semantic Rust
    /// references. Validate their ranges and merge them into the same move.
    pub fn complete_file_move(
        &mut self,
        from: &str,
        to: &str,
        drafts: &[FileMoveDraft],
        review: &str,
        resolutions: &[FileMoveRustResolution],
    ) -> Result<FileMovePlan> {
        let mut plan = self.plan_file_move_with_drafts(from, to, drafts)?;
        require(
            plan.review == review,
            "The sources changed while resolving Rust references. Review the move again",
        )?;
        require(
            plan.generated_rust.len() == resolutions.len(),
            "Resolve every affected generated Rust symbol",
        )?;
        let drafts = move_drafts(drafts)?;
        let files = rust_files(self);
        let generated = plan
            .generated_rust
            .iter()
            .map(|rename| {
                file_path(&rename.uri).map(|path| {
                    path.parent()
                        .expect("a generated Rust module has a parent")
                        .to_owned()
                })
            })
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        let mut by_file = BTreeMap::<String, Vec<_>>::new();
        let mut resolved = BTreeSet::new();
        for resolution in resolutions {
            let index = plan
                .generated_rust
                .iter()
                .position(|name| name == &resolution.rename)
                .ok_or_else(|| {
                    LanguageError(
                        "The generated Rust declaration changed. Review the move again".into(),
                    )
                })?;
            require(
                resolved.insert(index),
                "A generated Rust rename was resolved twice",
            )?;
            for document in &resolution.documents {
                let uri = canonical_file_uri(&document.uri)?;
                let file = file_path(&uri)?;
                require(
                    file.parent()
                        .is_none_or(|directory| !generated.contains(directory)),
                    "Edit the authored Rust source for this move",
                )?;
                by_file
                    .entry(uri)
                    .or_default()
                    .push((&resolution.rename, document));
            }
        }
        for (uri, edits) in by_file {
            let (_, _, workspace) = files.get(uri.as_str()).ok_or_else(|| {
                LanguageError(
                    "The Rust rename reaches outside the indexed workspace sources".into(),
                )
            })?;
            let (version, source) = match drafts.get(&uri) {
                Some(draft) => (draft.version, draft.source.clone()),
                None => (-1, workspace.read_source(&uri)?),
            };
            let index = LineIndex::new(&source);
            let mut patches = Vec::new();
            for (rename, document) in edits {
                require(
                    document.version == version,
                    "The Rust reference buffer changed. Review the move again",
                )?;
                for edit in &document.edits {
                    let span = index.span(edit.range)?;
                    let previous = &source[span.start..span.end];
                    require(
                        edit.new_text == rename.new_name
                            && previous.strip_prefix("r#").unwrap_or(previous) == rename.old_name,
                        "The Rust rename added edits beyond the reviewed symbol change",
                    )?;
                    patches.push(TextPatch {
                        span,
                        replacement: edit.new_text.clone(),
                    });
                }
            }
            check_rust_rename_tokens(&source, &patches)?;
            if let Some(prior) = plan
                .documents
                .iter()
                .position(|entry| entry.edit.uri == uri)
            {
                let before = plan.documents.remove(prior);
                require(
                    before.before == source,
                    "The Rust reference buffer changed. Review the move again",
                )?;
                for edit in before.edit.edits {
                    patches.push(TextPatch {
                        span: index.span(edit.range)?,
                        replacement: edit.new_text,
                    });
                }
            }
            patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
            patches.dedup();
            let document = move_document(&uri, version, &index, patches)?;
            RustXamlReferences::parse(&document.after, false)?;
            plan.documents.push(document);
        }
        plan.documents
            .sort_by(|left, right| left.edit.uri.cmp(&right.edit.uri));
        let mut reviewed = DefaultHasher::new();
        plan.review.hash(&mut reviewed);
        for document in &plan.documents {
            (
                &document.edit.uri,
                document.edit.version,
                &document.before,
                &document.after,
            )
                .hash(&mut reviewed);
        }
        plan.review = format!("{:016x}", reviewed.finish());
        Ok(plan)
    }
}
