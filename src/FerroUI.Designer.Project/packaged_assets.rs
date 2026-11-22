//! Exact asset keys from the selected Cargo build, independent of XAML include routing.

use crate::{uri::decode, ProjectError, Result};
use ferroui_build_scan::packaged_assets::PackagedAssetIndex;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

pub(crate) const INDEX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub struct PackagedAssets {
    packages: BTreeMap<String, Vec<Package>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Package {
    root: PathBuf,
    assembly: String,
    inputs: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PackagedAssetSource<'a> {
    pub assembly: &'a str,
    pub file: PathBuf,
}

impl PackagedAssets {
    pub(crate) fn read(
        &mut self,
        output: &Path,
        assembly: &str,
        crate_name: &str,
        remaining: &mut u64,
    ) {
        let path = output.join(PackagedAssetIndex::FILE_NAME);
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            return;
        };
        if !metadata.is_file() || metadata.len() > *remaining {
            return;
        }
        let mut text = String::new();
        let size = fs::File::open(&path)
            .and_then(|file| file.take(*remaining + 1).read_to_string(&mut text));
        let Ok(size) = size else { return };
        let fits = size as u64 <= *remaining;
        *remaining = remaining.saturating_sub(size as u64);
        if !fits {
            return;
        }
        let Ok(index) = PackagedAssetIndex::parse(&text) else {
            return;
        };
        if index.assembly != assembly
            || index.crate_name != crate_name
            || assembly.is_empty()
            || !index.manifest.is_absolute()
        {
            return;
        }
        let Ok(manifest) = index.manifest.canonicalize() else {
            return;
        };
        let Some(root) = manifest.parent().filter(|_| {
            manifest
                .file_name()
                .is_some_and(|name| name == "Cargo.toml")
                && manifest.is_file()
        }) else {
            return;
        };
        let mut inputs = BTreeMap::new();
        for asset in index.assets {
            if asset.key.starts_with('/')
                && !asset.source.as_os_str().is_empty()
                && asset
                    .source
                    .components()
                    .all(|component| matches!(component, Component::Normal(_)))
            {
                // The registration table also keeps the last stream for a duplicate key.
                inputs.insert(asset.key, asset.source);
            }
        }
        let package = Package {
            root: root.to_owned(),
            assembly: index.assembly,
            inputs,
        };
        let known = self.packages.entry(assembly.to_uppercase()).or_default();
        if !known.contains(&package) {
            known.push(package);
        }
    }

    /// An explicit asset address is owned by the build's assembly even if this
    /// particular key was excluded from its export plan.
    pub fn knows_assembly(&self, uri: &str) -> bool {
        address(uri).is_some_and(|(assembly, _)| self.packages.contains_key(&assembly))
    }

    /// Resolve a published `ferres://` key. Assembly names match without case,
    /// asset keys match the case and bytes used by the native asset loader.
    pub fn resolve(&self, uri: &str) -> Result<Option<PackagedAssetSource<'_>>> {
        let Some((assembly, key)) = address(uri) else {
            return Ok(None);
        };
        let Some(packages) = self.packages.get(&assembly) else {
            return Ok(None);
        };
        let [package] = packages.as_slice() else {
            return Err(ProjectError(format!(
                "Multiple Cargo outputs claim the asset assembly in {uri}"
            )));
        };
        Ok(package
            .inputs
            .get(&key)
            .and_then(|path| package.source(path))
            .map(|file| PackagedAssetSource {
                assembly: &package.assembly,
                file,
            }))
    }

    pub fn contains_source(&self, canonical_file: &Path) -> bool {
        self.packages.values().flatten().any(|package| {
            canonical_file.starts_with(&package.root)
                && package
                    .inputs
                    .values()
                    .any(|path| package.source(path).as_deref() == Some(canonical_file))
        })
    }
}

impl Package {
    fn source(&self, relative: &Path) -> Option<PathBuf> {
        let file = self.root.join(relative).canonicalize().ok()?;
        (file.starts_with(&self.root) && file.is_file()).then_some(file)
    }
}

fn address(uri: &str) -> Option<(String, String)> {
    let (scheme, rest) = uri.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("ferres") {
        return None;
    }
    let rest = rest.split(['?', '#']).next()?;
    let (assembly, path) = rest.split_at(rest.find('/')?);
    Some((decode(assembly).ok()?.to_uppercase(), decode(path).ok()?))
}
