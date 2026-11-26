//! Exact asset keys from the selected Cargo build, independent of XAML include routing.

use crate::{
    uri::{canonical_path, decode, encode},
    ProjectError, Result,
};
use ferroui_build_scan::packaged_assets::{AssetPublication, PackagedAssetIndex};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Component, Path, PathBuf},
};

#[path = "packaged_asset_moves.rs"]
mod moves;
pub use moves::PublishedAssetMove;

pub(crate) const INDEX_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackagedAssets {
    packages: BTreeMap<String, Vec<Package>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Package {
    root: PathBuf,
    assembly: String,
    inputs: BTreeMap<String, PathBuf>,
    publications: Vec<AssetPublication>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct PackagedAssetSource<'a> {
    pub assembly: &'a str,
    pub file: PathBuf,
}

#[derive(serde::Serialize)]
pub struct PublishedAsset<'a> {
    pub address: String,
    pub assembly: &'a str,
    pub key: &'a str,
    #[serde(rename = "sourceRoot", skip_serializing_if = "Option::is_none")]
    pub source_root: Option<&'a Path>,
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
        let Ok((index, publication)) = PackagedAssetIndex::parse_with_publication(&text) else {
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
            publications: publication.into_iter().collect(),
        };
        let known = self.packages.entry(assembly.to_uppercase()).or_default();
        if let Some(existing) = known.iter_mut().find(|existing| existing.root == package.root
            && existing.assembly == package.assembly && existing.inputs == package.inputs)
        {
            for publication in package.publications {
                if !existing.publications.contains(&publication) { existing.publications.push(publication); }
            }
        } else {
            known.push(package);
        }
    }

    /// An explicit asset address is owned by the build's assembly even if this
    /// particular key was excluded from its export plan.
    pub fn knows_assembly(&self, uri: &str) -> bool {
        !matches!(self.package_address(uri), Ok(None))
    }

    pub fn is_empty(&self) -> bool {
        self.packages
            .values()
            .flatten()
            .all(|package| package.inputs.is_empty())
    }

    /// List the unambiguous exports of the selected Cargo build. Resolve the
    /// address again before opening its current source file.
    pub fn published(&self) -> impl Iterator<Item = PublishedAsset<'_>> {
        self.packages
            .values()
            .filter_map(|outputs| {
                if let [package] = outputs.as_slice() {
                    Some(package)
                } else {
                    None
                }
            })
            .flat_map(|package| {
                package.inputs.keys().map(|key| PublishedAsset {
                    address: published_address(&package.assembly, key),
                    assembly: &package.assembly,
                    key,
                    source_root: (package.publications.len() == 1).then_some(package.root.as_path()),
                })
            })
    }

    /// Resolve a published `ferres://` key. Assembly names match without case,
    /// asset keys match the case and bytes used by the native asset loader.
    pub fn resolve(&self, uri: &str) -> Result<Option<PackagedAssetSource<'_>>> {
        Ok(self.input(uri)?.filter(|asset| asset.file.is_file()))
    }

    /// The physical input of a registered key, including a missing file whose
    /// path stays in the package. Preview can watch it and report missing bytes.
    pub fn input(&self, uri: &str) -> Result<Option<PackagedAssetSource<'_>>> {
        let Some((package, _, key)) = self.package_address(uri)? else {
            return Ok(None);
        };
        let Some(path) = package.inputs.get(&key) else {
            return Ok(None);
        };
        let file = canonical_path(&package.root.join(path))?;
        if !file.starts_with(&package.root) {
            return Err(ProjectError(format!(
                "The build input for {uri} leaves its Cargo package"
            )));
        }
        Ok(Some(PackagedAssetSource {
            assembly: &package.assembly,
            file,
        }))
    }

    /// List exact exported asset addresses below a folder for native consumers
    /// that enumerate an asset collection, such as the font loader.
    pub fn below(&self, folder: &str) -> Result<Vec<String>> {
        let Some((package, authority, mut prefix)) = self.package_address(folder)? else {
            return Ok(Vec::new());
        };
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        Ok(package
            .inputs
            .range(prefix.clone()..)
            .take_while(|(key, _)| key.starts_with(&prefix))
            .map(|(key, _)| format!("ferres://{authority}{}", encode(key)))
            .collect())
    }

    fn package_address<'a, 'uri>(
        &'a self,
        uri: &'uri str,
    ) -> Result<Option<(&'a Package, &'uri str, String)>> {
        let Some((assembly, key)) = address(uri) else {
            return Ok(None);
        };
        let literal = self.packages.get(&assembly.to_uppercase());
        let decoded = literal.is_none().then(|| decode(assembly).ok()).flatten();
        let packages = literal.or_else(|| self.packages.get(&decoded.as_ref()?.to_uppercase()));
        let mut matches = packages.into_iter().flatten().filter(|package| {
            literal.is_some() || decoded.as_deref() == Some(package.assembly.as_str())
        });
        let Some(package) = matches.next() else {
            return Ok(None);
        };
        if matches.next().is_some() {
            return Err(ProjectError(format!(
                "Multiple Cargo outputs claim the asset assembly in {uri}"
            )));
        }
        Ok(Some((package, assembly, key)))
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

/// Raster formats that the designers can insert as an authored Image source.
pub fn is_raster_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "ico"
            )
        })
}

impl Package {
    fn source(&self, relative: &Path) -> Option<PathBuf> {
        let file = self.root.join(relative).canonicalize().ok()?;
        (file.starts_with(&self.root) && file.is_file()).then_some(file)
    }
}

fn address(uri: &str) -> Option<(&str, String)> {
    let (scheme, rest) = uri.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("ferres") {
        return None;
    }
    let rest = rest.split(['?', '#']).next()?;
    let (assembly, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    Some((assembly, decode(path).ok()?))
}

fn published_address(assembly: &str, key: &str) -> String {
    let authority = encode(assembly).replace('/', "%2F").replace(':', "%3A");
    format!("ferres://{authority}{}", encode(key))
}

/// An absolute address of an individual asset in FerroUI's package registry.
pub fn is_packaged_asset_uri(uri: &str) -> bool {
    address(uri).is_some_and(|(authority, key)| {
        !authority.is_empty()
            && !authority.contains([':', '@', '\\'])
            && decode(authority).is_ok()
            && key.starts_with('/')
            && !key.ends_with('/')
            && !key.contains('\\')
    })
}
