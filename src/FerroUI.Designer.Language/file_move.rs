//! Plan a workspace file move against the same source snapshots used for navigation.
use super::{LanguageService, State, file_move_literals};
use super::source_review::reviewed_document as move_document;
pub use super::source_review::ReviewedDocument as FileMoveDocument;
use crate::{LanguageError, Result, require};
use ferroui_designer_core::{Document, TextPatch};
use ferroui_designer_project::{
    XamlFileMove, canonical_file_uri, file_path, relative_asset_uri, relative_file_reference,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

#[path = "file_move_rust.rs"]
mod rust;
#[path = "file_move_symbol_edits.rs"]
mod symbol_edits;
#[path = "file_move_saved.rs"]
mod saved;
use rust::CompiledAddresses;
pub(super) use rust::rust_files;
pub use rust::GeneratedRustRename;
pub use symbol_edits::FileMoveRustResolution;

const MAX_DRAFT_BYTES: usize = 8 * 1024 * 1024;

pub use super::source_review::SourceDraft as FileMoveDraft;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Ord, PartialOrd)]
pub struct PublishedAssetAddressMove {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileMovePlan {
    pub from: String,
    pub to: String,
    pub review: String,
    pub documents: Vec<FileMoveDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "assetDigest")]
    pub asset_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "publishedAssets")]
    pub published_assets: Vec<PublishedAssetAddressMove>,
    #[serde(default, skip_serializing_if = "Vec::is_empty", rename = "generatedRust")]
    pub generated_rust: Vec<GeneratedRustRename>,
}

impl LanguageService {
    /// Include unopened references and reload physical snapshots before each review.
    /// Open drafts keep precedence over disk even when an adjacent file changed.
    pub fn plan_file_move(&mut self, from: &str, to: &str) -> Result<FileMovePlan> {
        self.plan_file_move_with_drafts(from, to, &[])
    }

    pub fn plan_file_move_with_drafts(
        &mut self,
        from: &str,
        to: &str,
        drafts: &[FileMoveDraft],
    ) -> Result<FileMovePlan> {
        let source = self.document_uri(from)?.into_owned();
        let root = self.workspace_root(&source).cloned().ok_or_else(|| {
            LanguageError("Open the source in its workspace before moving it".into())
        })?;
        let movement = self.workspaces[&root].plan_xaml_move(from, to)?;
        self.refresh_project_index()?;
        require(
            self.workspaces[&root].plan_xaml_move(from, to)? == movement,
            "The file or destination moved while planning",
        )?;
        self.review_file_move(movement, drafts)
    }

    fn review_file_move(
        &self,
        movement: XamlFileMove,
        drafts: &[FileMoveDraft],
    ) -> Result<FileMovePlan> {
        let moved = self.state(&movement.from)?;
        require(
            moved.document.is_valid(),
            "Repair this XAML document before moving it",
        )?;
        require(
            self.documents
                .get(&movement.to)
                .is_none_or(|state| !state.opened),
            "The move destination already has an open buffer",
        )?;
        let drafts = move_drafts(drafts)?;
        let addresses = CompiledAddresses::prepare(self, &movement, &drafts)?;
        let mut documents = Vec::new();
        for (uri, state) in &self.documents {
            if self.workspace_root(uri).is_none() {
                continue;
            }
            if !state.document.is_valid() {
                check_incomplete_source(uri, state, &movement)?;
                continue;
            }
            let patches = self.move_patches(uri, state, &movement, &addresses)?;
            if !patches.is_empty() || uri == &movement.from {
                let document = move_document(uri, state.version, &state.index, patches)?;
                Document::new(&document.after)?;
                documents.push(document);
            }
        }
        let disk = std::fs::read(file_path(&movement.from)?)
            .map_err(|error| LanguageError(error.to_string()))?;
        let mut id = DefaultHasher::new();
        (&movement.from, &movement.to, &disk).hash(&mut id);
        documents.extend(self.move_rust_paths(&movement, &drafts, &addresses, &mut id)?);
        documents.extend(saved::plan(self, &movement, &drafts)?);
        documents.sort_by(|left, right| left.edit.uri.cmp(&right.edit.uri));
        for document in &documents {
            (
                &document.edit.uri,
                document.edit.version,
                &document.before,
                &document.after,
            )
                .hash(&mut id);
        }
        for rename in &addresses.generated_rust {
            (&rename.uri, rename.position.line, rename.position.character, &rename.old_name, &rename.new_name).hash(&mut id);
        }
        Ok(FileMovePlan {
            review: format!("{:016x}", id.finish()),
            from: movement.from,
            to: movement.to,
            documents,
            asset_digest: None,
            published_assets: Vec::new(),
            generated_rust: addresses.generated_rust,
        })
    }

    fn move_patches(
        &self,
        owner: &str,
        state: &State,
        movement: &XamlFileMove,
        addresses: &CompiledAddresses,
    ) -> Result<Vec<TextPatch>> {
        let after_owner = if owner == movement.from {
            &movement.to
        } else {
            owner
        };
        let mut patches = Vec::new();
        let resources = self.compiled_resources(owner);
        for site in file_move_literals::paths(&state.document, self.schema_for(owner)) {
            let suffix = if site.value.starts_with("resm:") {
                &['#'][..]
            } else {
                &['?', '#'][..]
            };
            let delimiter = site.value.find(suffix).unwrap_or(site.value.len());
            let (path, suffix) = site.value.split_at(delimiter);
            if path.is_empty() {
                continue;
            }
            if let Some(resources) = &resources {
                match resources.resolve(owner, path) {
                    // File-backed keys remain in the same compiled group. Both design
                    // and build resolve this spelling through the same logical route.
                    Ok(Some(_)) => continue,
                    Err(error) if owner == movement.from || resources.references(owner, path, &movement.from) => return Err(error.into()),
                    Err(_) => continue,
                    Ok(None) => {}
                }
            }
            if let Some(next) = addresses.replacement(self, owner, path) {
                if next != path {
                    let replacement = format!("{next}{suffix}");
                    patches.extend(site.patches(&replacement));
                }
                continue;
            }
            let Ok(target) = self.resource_uri(owner, path) else {
                continue;
            };
            if !target.starts_with("file://") {
                continue;
            }
            let after_target = if target == movement.from {
                &movement.to
            } else {
                &target
            };
            if self
                .resource_uri(after_owner, path)
                .is_ok_and(|test| test == *after_target)
            {
                continue;
            }
            if addresses.preserves_compiled_base()
                && !(path.starts_with('/') || path.contains(':'))
                && file_path(owner).is_ok_and(|path| {
                    path.ancestors().skip(1).find(|parent| parent.join("Cargo.toml").is_file())
                        == Some(movement.package_root.as_path())
                })
                && resources.as_ref().is_none_or(|resources| {
                    resources.has_compiled_base(owner) || !resources.covers_compiled_sources()
                })
            {
                // The Cargo compiler still resolves this literal relative to the
                // published key. Rebasing it for the physical file could break
                // the compiled tree even if it worked in an isolated preview.
                return Err(LanguageError(format!(
                    "The selected build does not map the compiled target of '{}' in {owner}. Use an explicit XAML resource URI or rebuild the project schema before reviewing the move.",
                    site.value
                )));
            }
            let spelling = new_reference(path, after_owner, after_target, movement)?;
            let replacement = format!("{spelling}{suffix}");
            patches.extend(site.patches(&replacement));
        }
        Ok(patches)
    }
}

pub(super) fn move_drafts(drafts: &[FileMoveDraft]) -> Result<BTreeMap<String, &FileMoveDraft>> {
    require(drafts.len() <= 256, "Too many open source drafts for review")?;
    let mut snapshots = BTreeMap::new();
    let mut remaining = MAX_DRAFT_BYTES;
    for draft in drafts {
        require(
            draft.version >= 0 && draft.source.len() <= remaining,
            "Open source drafts exceed the review source limit",
        )?;
        remaining -= draft.source.len();
        let uri = canonical_file_uri(&draft.uri)?;
        require(
            snapshots.insert(uri, draft).is_none(),
            "Multiple open buffers name the same file",
        )?;
    }
    Ok(snapshots)
}

fn check_incomplete_source(uri: &str, state: &State, movement: &XamlFileMove) -> Result<()> {
    let from = file_path(&movement.from)?;
    let name = from
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let text = state.document.source();
    require(
        !text.contains(name)
            && !relative_asset_uri(name).is_some_and(|encoded| {
                text.to_ascii_lowercase()
                    .contains(&encoded.to_ascii_lowercase())
            }),
        format!("Repair the draft that may reference this file before moving it: {uri}"),
    )
}

pub(super) fn new_reference(path: &str, owner: &str, target: &str, movement: &XamlFileMove) -> Result<String> {
    if let Some(resource) = path.strip_prefix("ferres://") {
        let assembly = resource.split('/').next().unwrap_or_default();
        return Ok(format!(
            "ferres://{assembly}/{}",
            package_reference(target, &movement.package_root)?
        ));
    }
    if let Some(resource) = path.strip_prefix("resm:") {
        let query = resource.find('?').unwrap_or(resource.len());
        let (key, suffix) = resource.split_at(query);
        let old_key = package_reference(&movement.from, &movement.package_root)?.replace('/', ".");
        let new_key = package_reference(target, &movement.package_root)?.replace('/', ".");
        let prefix = key.strip_suffix(&old_key).ok_or_else(|| {
            LanguageError(
                "Review the custom embedded resource mapping before moving this file".into(),
            )
        })?;
        return Ok(format!("resm:{prefix}{new_key}{suffix}"));
    }
    if path.starts_with("file:") || path.starts_with('/') {
        return Ok(target.to_owned());
    }
    let relative = relative_file_reference(owner, target)?;
    if path.starts_with("./") && !relative.starts_with("../") {
        Ok(format!("./{relative}"))
    } else {
        Ok(relative)
    }
}

fn package_reference(uri: &str, root: &Path) -> Result<String> {
    let path = file_path(uri)?;
    let relative = path
        .strip_prefix(root)
        .map_err(|error| LanguageError(error.to_string()))?;
    relative_asset_uri(&relative.to_string_lossy().replace('\\', "/"))
        .ok_or_else(|| LanguageError("The new resource address cannot be encoded".into()))
}
