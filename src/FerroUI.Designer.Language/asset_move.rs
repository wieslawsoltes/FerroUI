//! Review asset moves using typed XAML URI sites, Rust operands, and the selected build.
use super::file_moves::{move_drafts, new_reference, rust_files, PublishedAssetAddressMove};
use super::source_review::{check_incomplete, reviewed_document, ReviewedDocument, RustSnapshot};
use super::{
    file_move_literals, handlers::read_rust, FileMoveDraft, FileMovePlan, LanguageService, State,
};
use crate::{require, LanguageError, Result};
use ferroui_designer_core::{Document, SourceSpan, TextPatch};
use ferroui_designer_project::{
    asset_digest, canonical_file_uri, file_path, file_uri, is_packaged_asset_uri,
    relative_asset_uri, relative_file_path, ProjectFileMove, RustAssetReferences, RustFilePath,
    RustPathAnchor,
};
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

#[path = "asset_move_build.rs"]
mod build;

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
        let mut publications = BTreeSet::new();
        for schema in self.package_schemas.values() {
            publications.extend(schema.resources.packaged_assets().plan_moves(&movement)?);
        }
        let published = !publications.is_empty();
        let build_inputs = build::build_inputs(self, &movement, &publications)?;
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
            let patches = self.asset_xaml_patches(uri, state, &movement, published)?;
            if !patches.is_empty() {
                let document = reviewed_document(uri, state.version, &state.index, patches)?;
                Document::new(&document.after)?;
                documents.push(document);
            }
        }
        let mut build_sources = Vec::new();
        for (uri, (entry, package, _)) in rust_files(self) {
            let snapshot = RustSnapshot::read(uri, &drafts)?;
            let build = build_inputs.get(uri);
            let may_include =
                |text: &str| text.contains("include_bytes") || text.contains("include_str");
            let may_address = |text: &str| {
                published
                    && (text.contains("ferres") || (text.contains("concat") && mentions(text)))
            };
            let may_reference = |text: &str| may_include(text) || may_address(text);
            if build.is_none() && !may_reference(&snapshot.text) && snapshot.version < 0 {
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
            let resources = self.compiled_resources(uri);
            let moved_address = |site: &RustFilePath| -> Result<Option<String>> {
                if let Some(resources) = resources.as_ref().filter(|_| published) {
                    return Ok(resources
                        .packaged_assets()
                        .moved_address(&site.path, &movement)?);
                }
                Ok(None)
            };
            let could_reference =
                |text: &str| may_reference(text) && (mentions(text) || text.contains("concat"));
            let syntax = match RustAssetReferences::parse(&snapshot.text, build.is_some()) {
                Ok(sites) => sites,
                Err(error) => {
                    if build.is_some() {
                        return Err(error.into());
                    }
                    let disk_uses_asset = snapshot.version >= 0
                        && read_rust(uri).is_ok_and(|disk| {
                            may_reference(&disk)
                                && RustAssetReferences::parse(&disk, false).map_or_else(
                                    |_| could_reference(&disk),
                                    |sites| {
                                        sites
                                            .includes
                                            .iter()
                                            .any(|site| reference_base(site).is_some())
                                            || sites.addresses.iter().any(|site| {
                                                !matches!(moved_address(site), Ok(None))
                                            })
                                    },
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
            if build.is_some() {
                build_sources.push((uri, snapshot.version, snapshot.text.clone()));
            }
            let mut patches =
                build::build_patches(build, &syntax, &snapshot.text, &movement.package_root)?;
            for site in syntax.includes {
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
            for site in syntax.addresses {
                if let Some(replacement) = moved_address(&site)? {
                    patches.push(TextPatch {
                        span: SourceSpan::new(site.range.start, site.range.end),
                        replacement: site.replacement(&snapshot.text, &replacement),
                    });
                }
            }
            if !patches.is_empty() {
                patches.sort_by_key(|patch| (patch.span.start, patch.span.end));
                patches.dedup();
                let document = snapshot.document(uri, patches)?;
                RustAssetReferences::parse(&document.after, false)?;
                documents.push(document);
            }
        }
        documents.sort_by(|left, right| left.edit.uri.cmp(&right.edit.uri));
        let mut publication_snapshot = DefaultHasher::new();
        publications.hash(&mut publication_snapshot);
        build_sources.hash(&mut publication_snapshot);
        let digest = asset_digest(&original).map_err(|error| LanguageError(error.to_string()))?;
        let review = asset_review_id(
            &movement,
            &digest,
            &documents,
            publication_snapshot.finish(),
        );
        let published_assets = publications
            .iter()
            .map(|plan| {
                let (from, to) = plan.addresses();
                PublishedAssetAddressMove { from, to }
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(FileMovePlan {
            from: movement.from,
            to: movement.to,
            review,
            documents,
            asset_digest: Some(digest),
            published_assets,
            generated_rust: Vec::new(),
        })
    }

    fn asset_xaml_patches(
        &self,
        uri: &str,
        state: &State,
        movement: &ProjectFileMove,
        published: bool,
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
            if published {
                if let Some(resources) = &resources {
                    if let Some(next) = resources.moved_asset_address(uri, value, movement)? {
                        let replacement = format!("{next}{suffix}");
                        patches.extend(site.patches(&replacement));
                        continue;
                    }
                }
                if is_packaged_asset_uri(value) {
                    continue;
                }
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
    digest: &str,
    documents: &[ReviewedDocument],
    publication: u64,
) -> String {
    let mut hash = DefaultHasher::new();
    (&movement.from, &movement.to).hash(&mut hash);
    publication.hash(&mut hash);
    digest.hash(&mut hash);
    for document in documents {
        (
            &document.edit.uri,
            document.edit.version,
            &document.before,
            &document.after,
        )
            .hash(&mut hash);
    }
    format!("{:016x}", hash.finish())
}
