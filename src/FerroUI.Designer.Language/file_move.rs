//! Plan a workspace file move against the same source snapshots used for navigation.
use super::{file_move_literals, LanguageService, State};
use crate::{require, DocumentEdits, LanguageError, Result, TextEdit};
use ferroui_designer_core::{Document, TextPatch};
use ferroui_designer_project::{
    file_path, relative_asset_uri, relative_file_reference, XamlFileMove,
};
use serde::{Deserialize, Serialize};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileMoveDocument {
    pub edit: DocumentEdits,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FileMovePlan {
    pub from: String,
    pub to: String,
    pub review: String,
    pub documents: Vec<FileMoveDocument>,
}

impl LanguageService {
    /// Include unopened references and reload physical snapshots before each review.
    /// Open drafts keep precedence over disk even when an adjacent file changed.
    pub fn plan_file_move(&mut self, from: &str, to: &str) -> Result<FileMovePlan> {
        let source = self.document_uri(from)?.into_owned();
        let root = self.workspace_root(&source).cloned().ok_or_else(|| {
            LanguageError("Open the source in its workspace before moving it".into())
        })?;
        let movement = self.workspaces[&root].plan_xaml_move(from, to)?;
        self.refresh_move_projects()?;
        require(
            self.workspaces[&root].plan_xaml_move(from, to)? == movement,
            "The file or destination moved while planning",
        )?;
        self.review_file_move(movement)
    }

    fn refresh_move_projects(&mut self) -> Result<()> {
        let solutions = self
            .workspaces
            .values()
            .map(|workspace| workspace.solution.clone())
            .collect::<Vec<_>>();
        for mut solution in solutions {
            solution.refresh()?;
            self.index_solution(&solution)?;
        }
        let missing = self
            .documents
            .iter()
            .filter(|(uri, state)| {
                !state.opened
                    && uri.starts_with("file://")
                    && file_path(uri).is_ok_and(|path| !path.is_file())
            })
            .map(|(uri, _)| uri.clone())
            .collect::<Vec<_>>();
        for uri in missing {
            self.set_state(uri, None);
        }
        Ok(())
    }

    fn review_file_move(&self, movement: XamlFileMove) -> Result<FileMovePlan> {
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
        let mut documents = Vec::new();
        for (uri, state) in &self.documents {
            if self.workspace_root(uri).is_none() {
                continue;
            }
            if !state.document.is_valid() {
                check_incomplete_source(uri, state, &movement)?;
                continue;
            }
            let patches = self.move_patches(uri, state, &movement)?;
            if !patches.is_empty() || uri == &movement.from {
                documents.push(move_document(uri, state, patches)?);
            }
        }
        let disk = std::fs::read(file_path(&movement.from)?)
            .map_err(|error| LanguageError(error.to_string()))?;
        let mut id = DefaultHasher::new();
        (&movement.from, &movement.to, &disk).hash(&mut id);
        for document in &documents {
            (
                &document.edit.uri,
                document.edit.version,
                &document.before,
                &document.after,
            )
                .hash(&mut id);
        }
        Ok(FileMovePlan {
            review: format!("{:016x}", id.finish()),
            from: movement.from,
            to: movement.to,
            documents,
        })
    }

    fn move_patches(
        &self,
        owner: &str,
        state: &State,
        movement: &XamlFileMove,
    ) -> Result<Vec<TextPatch>> {
        let after_owner = if owner == movement.from {
            &movement.to
        } else {
            owner
        };
        let mut patches = Vec::new();
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
            let spelling = new_reference(path, after_owner, after_target, movement)?;
            let replacement = format!("{spelling}{suffix}");
            patches.extend(site.patches(&replacement));
        }
        Ok(patches)
    }
}

fn move_document(uri: &str, state: &State, patches: Vec<TextPatch>) -> Result<FileMoveDocument> {
    let before = state.document.source();
    let edits = patches
        .iter()
        .map(|patch| {
            Ok(TextEdit {
                range: state.index.range(patch.span)?,
                new_text: patch.replacement.clone(),
            })
        })
        .collect::<Result<_>>()?;
    let after = TextPatch::apply_all(before, patches)?;
    Document::new(&after)?;
    Ok(FileMoveDocument {
        edit: DocumentEdits {
            uri: uri.to_owned(),
            version: state.version,
            edits,
        },
        before: before.to_owned(),
        after,
    })
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

fn new_reference(path: &str, owner: &str, target: &str, movement: &XamlFileMove) -> Result<String> {
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
