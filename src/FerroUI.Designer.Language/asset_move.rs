//! Review physical asset moves using the typed XAML URI sites and authored Rust includes.
use super::file_moves::{move_drafts, new_reference, rust_files};
use super::source_review::{ReviewedDocument, RustSnapshot, check_incomplete, reviewed_document};
use super::{
    FileMoveDraft, FileMovePlan, LanguageService, State, file_move_literals, handlers::read_rust,
};
use crate::{LanguageError, Result, require};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use ferroui_designer_project::{
    ProjectFileMove, RustFilePath, RustPathAnchor, canonical_file_uri, file_path, file_uri,
    relative_asset_uri, relative_file_path, rust_file_includes,
};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{self, Read};
use std::path::Path;

impl LanguageService {
    pub fn plan_asset_move(
        &mut self,
        from: &str,
        to: &str,
        drafts: &[FileMoveDraft],
    ) -> Result<FileMovePlan> {
        let source = self.document_uri(from)?.into_owned();
        let root = self
            .workspace_root(&source)
            .cloned()
            .ok_or_else(|| LanguageError("Choose an asset in the workspace".into()))?;
        let movement = self.workspaces[&root].plan_asset_move(from, to)?;
        self.refresh_project_index()?;
        require(
            self.workspaces[&root].plan_asset_move(from, to)? == movement,
            "The asset or destination moved while planning",
        )?;

        let drafts = move_drafts(drafts)?;
        let original = file_path(&movement.from)?;
        require(
            !self.package_schemas.values().any(|schema| schema.catalog.packaged_assets().contains_source(&original)),
            "This asset is published by a Cargo export table. Update the package's asset registration before moving it.",
        )?;
        let target = file_path(&movement.to)?;
        let name = original
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| LanguageError("The asset name cannot be represented as text".into()))?;
        let encoded = relative_asset_uri(name).unwrap_or_else(|| name.to_owned());
        let mentions = |source: &str| {
            source.contains(name)
                || source
                    .to_ascii_lowercase()
                    .contains(&encoded.to_ascii_lowercase())
        };
        let mut documents: Vec<ReviewedDocument> = Vec::new();
        for (uri, state) in &self.documents {
            if self.workspace_root(uri).is_none() {
                continue;
            }
            if !state.document.is_valid() {
                check_incomplete(uri, state, mentions)?;
                continue;
            }
            let patches = self.asset_xaml_patches(uri, state, &movement)?;
            if !patches.is_empty() {
                let document = reviewed_document(uri, state.version, &state.index, patches)?;
                Document::new(&document.after)?;
                documents.push(document);
            }
        }
        for (uri, (entry, package, _)) in rust_files(self) {
            let snapshot = RustSnapshot::read(uri, &drafts)?;
            let may_include =
                |text: &str| text.contains("include_bytes") || text.contains("include_str");
            if !may_include(&snapshot.text) && snapshot.version < 0 {
                continue;
            }
            let parent = entry
                .path
                .parent()
                .ok_or_else(|| LanguageError("The Rust source has no parent directory".into()))?;
            let package_root = package.authored_package_root(&entry.path);
            let reference_base = |site: &RustFilePath| {
                let base = match site.anchor {
                    RustPathAnchor::SourceFile => parent,
                    RustPathAnchor::Package => package_root,
                    RustPathAnchor::XamlRoot | RustPathAnchor::ResourceUri => return None,
                };
                let resolved = file_uri(base.join(&site.path))
                    .and_then(|uri| canonical_file_uri(&uri))
                    .ok();
                (resolved.as_deref() == Some(movement.from.as_str())).then_some(base)
            };
            let could_reference =
                |text: &str| may_include(text) && (mentions(text) || text.contains("concat"));
            let sites = match rust_file_includes(&snapshot.text) {
                Ok(sites) => sites,
                Err(_) => {
                    let disk_uses_asset = snapshot.version >= 0
                        && read_rust(uri).is_ok_and(|disk| {
                            may_include(&disk)
                                && rust_file_includes(&disk).map_or_else(
                                    |_| could_reference(&disk),
                                    |sites| sites.iter().any(|site| reference_base(site).is_some()),
                                )
                        });
                    require(
                        !could_reference(&snapshot.text) && !disk_uses_asset,
                        format!(
                            "Repair the Rust draft that may reference this asset before moving it: {uri}"
                        ),
                    )?;
                    continue;
                }
            };
            let mut patches = Vec::new();
            for site in sites {
                let Some(base) = reference_base(&site) else {
                    continue;
                };
                let relative = relative_file_path(base, &target)?;
                let replacement = if site.path.starts_with("./") && !relative.starts_with("../") {
                    format!("./{relative}")
                } else {
                    relative
                };
                patches.push(TextPatch {
                    span: SourceSpan::new(site.range.start, site.range.end),
                    replacement: site.replacement(&snapshot.text, &replacement),
                });
            }
            if !patches.is_empty() {
                documents.push(snapshot.document(uri, patches)?);
            }
        }
        documents.sort_by(|left, right| left.edit.uri.cmp(&right.edit.uri));
        let review = asset_review_id(&movement, &original, &documents)
            .map_err(|error| LanguageError(error.to_string()))?;
        Ok(FileMovePlan {
            from: movement.from,
            to: movement.to,
            review,
            documents,
            generated_rust: Vec::new(),
        })
    }

    fn asset_xaml_patches(
        &self,
        uri: &str,
        state: &State,
        movement: &ProjectFileMove,
    ) -> Result<Vec<TextPatch>> {
        let resources = self.compiled_resources(uri);
        let mut patches = Vec::new();
        for site in file_move_literals::paths(&state.document, self.schema_for(uri)) {
            let delim = if site.value.starts_with("resm:") {
                site.value.find('#')
            } else {
                site.value.find(['?', '#'])
            };
            let (value, suffix) = site.value.split_at(delim.unwrap_or(site.value.len()));
            if value.is_empty() {
                continue;
            }
            if resources
                .as_ref()
                .is_some_and(|compiled| compiled.references(uri, value, &movement.from))
            {
                return Err(LanguageError(format!(
                    "Review the compiled asset mapping before moving this file: {uri}"
                )));
            }
            if self
                .resource_uri(uri, value)
                .is_ok_and(|target| target == movement.from)
            {
                let reference = new_reference(value, uri, &movement.to, movement)?;
                let replacement = format!("{reference}{suffix}");
                patches.extend(site.patches(&replacement));
            }
        }
        Ok(patches)
    }
}

fn asset_review_id(
    movement: &ProjectFileMove,
    original: &Path,
    documents: &[ReviewedDocument],
) -> io::Result<String> {
    let mut hash = DefaultHasher::new();
    (&movement.from, &movement.to).hash(&mut hash);
    let mut file = std::fs::File::open(original)?;
    let mut bytes = [0; 32 * 1024];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hash.write(&bytes[..count]);
    }
    for document in documents {
        (
            &document.edit.uri,
            document.edit.version,
            &document.before,
            &document.after,
        )
            .hash(&mut hash);
    }
    Ok(format!("{:016x}", hash.finish()))
}
