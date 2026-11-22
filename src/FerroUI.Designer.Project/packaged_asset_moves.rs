//! Relocate a published file using the selected build's asset inputs and export plan.

use super::{Package, PackagedAssets};
use crate::{
    file_path, require,
    uri::{canonical_path, encode},
    ProjectError, ProjectFileMove, Result,
};
use ferroui_build_scan::packaged_assets::AssetDeclaration;
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PublishedAssetMove {
    pub assembly: String,
    pub from_key: String,
    pub to_key: String,
    pub directories: Vec<AssetDeclaration>,
    pub registrations: Vec<AssetDeclaration>,
}

impl PublishedAssetMove {
    pub fn addresses(&self) -> (String, String) {
        (
            super::published_address(&self.assembly, &self.from_key),
            super::published_address(&self.assembly, &self.to_key),
        )
    }
}

impl PackagedAssets {
    pub fn has_move_provenance(&self, address: &str) -> bool {
        matches!(self.package_address(address), Ok(Some((package, _, key))) if package.publications.len() == 1 && package.inputs.contains_key(&key))
    }

    pub fn plan_moves(&self, movement: &ProjectFileMove) -> Result<Vec<PublishedAssetMove>> {
        let source = file_path(&movement.from)?;
        let destination = file_path(&movement.to)?;
        let mut plans = Vec::new();
        for builds in self.packages.values() {
            for package in builds
                .iter()
                .filter(|package| source.starts_with(&package.root))
            {
                for (key, input) in &package.inputs {
                    if package.source(input).as_ref() != Some(&source) {
                        continue;
                    }
                    require(
                        builds.len() == 1,
                        "Rebuild the selected Cargo target to resolve conflicting asset tables",
                    )?;
                    require(
                        package.root == movement.package_root,
                        "Choose the Cargo package that publishes the asset",
                    )?;
                    plans.push(package.plan_move(&source, &destination, key)?);
                }
            }
        }
        Ok(plans)
    }

    /// Rewrite an address only when this consumer's build resolves it to the moved input.
    pub fn moved_address(&self, uri: &str, movement: &ProjectFileMove) -> Result<Option<String>> {
        let Some((package, authority, key)) = self.package_address(uri)? else {
            return Ok(None);
        };
        let source = file_path(&movement.from)?;
        if package
            .inputs
            .get(&key)
            .and_then(|input| package.source(input))
            .as_ref()
            != Some(&source)
        {
            return Ok(None);
        }
        let destination = file_path(&movement.to)?;
        let next = rooted_key(&package.root, &destination).ok_or_else(leaves_package)?;
        let suffix = &uri[uri.find(['?', '#']).unwrap_or(uri.len())..];
        Ok(Some(format!(
            "ferres://{authority}{}{suffix}",
            encode(&next)
        )))
    }

    /// Apply an editor file move to the build's lookup table until Cargo republishes it.
    pub(crate) fn retarget(&mut self, moves: &BTreeMap<String, String>) {
        let moves = moves
            .iter()
            .filter_map(|(from, to)| {
                Some((
                    canonical_path(&file_path(from).ok()?).ok()?,
                    file_path(to).ok()?,
                ))
            })
            .collect::<BTreeMap<_, _>>();
        if moves.is_empty() {
            return;
        }
        for builds in self.packages.values_mut() {
            for package in builds {
                package.retarget(&moves);
            }
        }
    }

    pub(crate) fn tracks_input(&self, uri: &str) -> bool {
        let Ok(file) = file_path(uri) else {
            return false;
        };
        self.packages.values().flatten().any(|package| {
            file.strip_prefix(&package.root)
                .ok()
                .is_some_and(|path| package.inputs.values().any(|input| input == path))
        })
    }
}

impl Package {
    fn plan_move(&self, source: &Path, target: &Path, key: &str) -> Result<PublishedAssetMove> {
        let publication = match self.publications.as_slice() {
            [publication] => publication,
            [] => return Err(ProjectError("This asset is published by a Cargo export table. Rebuild the project XAML schema to review its registration.".into())),
            _ => return Err(ProjectError("Select one Cargo target and rebuild its schema to resolve conflicting asset declarations".into())),
        };
        require(
            rooted_key(&self.root, source).as_deref() == Some(key),
            "The asset has a custom export key. Review its generator before moving it.",
        )?;
        let to_key = rooted_key(&self.root, target).ok_or_else(leaves_package)?;
        let directories = self.embedding_directories(&publication.directories, target);
        require(
            !directories.is_empty(),
            "Choose a destination within a directory embedded by this Cargo build",
        )?;
        let occupied = self.inputs.keys().any(|other| {
            other != key
                && (other == &to_key
                    || publication.registrations.is_some()
                        && other.to_lowercase() == to_key.to_lowercase())
        });
        require(
            !occupied,
            "The destination already has a published asset key",
        )?;
        let registrations = publication
            .registrations
            .iter()
            .flatten()
            .filter(|site| site.value.to_lowercase() == key.to_lowercase())
            .cloned()
            .collect::<Vec<_>>();
        if publication.registrations.is_some() && registrations.is_empty() {
            return Err(ProjectError(
                "Rebuild the package to identify the asset's closed export registration".into(),
            ));
        }
        Ok(PublishedAssetMove {
            assembly: self.assembly.clone(),
            from_key: key.into(),
            to_key,
            directories,
            registrations,
        })
    }

    fn embedding_directories(&self, directories: &[AssetDeclaration], target: &Path) -> Vec<AssetDeclaration> {
        directories.iter()
            .filter(|site| {
                let Ok(directory) = self.root.join(&site.value).canonicalize() else {
                    return false;
                };
                directory.starts_with(&self.root)
                    && target
                        .parent()
                        .and_then(|parent| parent.strip_prefix(&directory).ok())
                        .is_some_and(|rest| {
                            rest.components().all(|component| {
                                !component.as_os_str().to_string_lossy().starts_with('.')
                            })
                        })
            })
            .cloned()
            .collect()
    }

    fn retarget(&mut self, moves: &BTreeMap<PathBuf, PathBuf>) {
        let mut changed = Vec::new();
        let mut next = BTreeMap::new();
        for (key, path) in &self.inputs {
            let source = self.root.join(path);
            let destination = canonical_path(&source)
                .ok()
                .and_then(|source| moves.get(&source));
            let target = destination.and_then(|to| {
                let to = canonical_path(to).ok()?;
                let plan = self.plan_move(&source, &to, key).ok()?;
                Some((plan.to_key, to.strip_prefix(&self.root).ok()?.to_path_buf()))
            });
            if let Some((address, path)) = target {
                changed.push((key.clone(), address.clone()));
                next.insert(address, path);
            } else {
                next.insert(key.clone(), path.clone());
            }
        }
        if changed.is_empty() || next.len() != self.inputs.len() {
            return;
        }
        self.inputs = next;
        if let Some(registrations) = self.publications.first_mut().and_then(|p| p.registrations.as_mut())
        {
            for registration in registrations {
                if let Some((_, to)) = changed
                    .iter()
                    .find(|(from, _)| from.to_lowercase() == registration.value.to_lowercase())
                {
                    registration.value = to.clone();
                }
            }
        }
    }
}

fn leaves_package() -> ProjectError {
    ProjectError("The asset leaves its Cargo package".into())
}

fn rooted_key(root: &Path, file: &Path) -> Option<String> {
    let mut key = String::new();
    for part in file.strip_prefix(root).ok()?.components() {
        let Component::Normal(name) = part else {
            return None;
        };
        key.push('/');
        key.push_str(name.to_str()?);
    }
    (!key.is_empty()).then_some(key)
}
