//! Asset streams scoped to an isolated native preview of one Cargo build.

use ferroui_base::{
    platform::{AssetAssembly, AssetStream, IAssetLoader},
    utilities::{UriExtensions, UriRef},
};
use ferroui_designer_project::{file_uri, PackagedAssetSource, PackagedAssets};
use ferroui_markup_xaml::source_overlay::SourceSnapshot;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    time::SystemTime,
};
use url::Url;

/// The published asset table and the files requested during one native render.
#[derive(Clone, Default)]
pub struct PreviewAssets {
    catalog: Arc<PackagedAssets>,
    inputs: Rc<RefCell<BTreeMap<Url, AssetInput>>>,
}

#[derive(Eq, PartialEq)]
struct AssetInput {
    path: PathBuf,
    version: Option<(SystemTime, u64)>,
}

impl AssetInput {
    fn new(path: PathBuf) -> Self {
        let version = fs::metadata(&path)
            .ok()
            .filter(|metadata| metadata.is_file())
            .and_then(|metadata| Some((metadata.modified().ok()?, metadata.len())));
        Self { path, version }
    }
}

impl PreviewAssets {
    pub fn new(catalog: &Arc<PackagedAssets>) -> Self {
        Self {
            catalog: catalog.clone(),
            inputs: Rc::default(),
        }
    }

    pub fn apply(&self, snapshot: SourceSnapshot) -> SourceSnapshot {
        if self.catalog.is_empty() {
            return snapshot;
        }
        snapshot.with_asset_loader(|registered| {
            Rc::new(PreviewAssetLoader {
                registered,
                assets: self.clone(),
            })
        })
    }

    /// Include missing registered inputs so a recreated file can refresh the stage.
    pub fn dependencies(&self) -> BTreeSet<String> {
        self.inputs
            .borrow()
            .values()
            .filter_map(|input| file_uri(&input.path).ok())
            .collect()
    }

    pub(crate) fn changed(&self, catalog: &Arc<PackagedAssets>) -> bool {
        (!Arc::ptr_eq(&self.catalog, catalog) && self.catalog != *catalog)
            || self.inputs.borrow().iter().any(|(uri, previous)| {
                catalog
                    .input(uri.as_str())
                    .ok()
                    .flatten()
                    .is_none_or(|asset| AssetInput::new(asset.file) != *previous)
            })
    }

    pub(crate) fn observe(&self, catalog: &Arc<PackagedAssets>) -> Self {
        let inputs = self
            .inputs
            .borrow()
            .keys()
            .filter_map(|uri| {
                let asset = catalog.input(uri.as_str()).ok().flatten()?;
                Some((uri.clone(), AssetInput::new(asset.file)))
            })
            .collect();
        Self {
            catalog: catalog.clone(),
            inputs: Rc::new(RefCell::new(inputs)),
        }
    }
}

struct PreviewAssetLoader {
    registered: Rc<dyn IAssetLoader>,
    assets: PreviewAssets,
}

impl PreviewAssetLoader {
    fn lookup(
        &self,
        uri: &UriRef,
        base: Option<&Url>,
    ) -> io::Result<Option<PackagedAssetSource<'_>>> {
        let Ok(address) = UriExtensions::ensure_absolute(uri, base) else {
            return Ok(None);
        };
        let asset = self
            .assets
            .catalog
            .input(address.as_str())
            .map_err(io::Error::other)?;
        if let Some(asset) = &asset {
            self.assets
                .inputs
                .borrow_mut()
                .entry(address)
                .or_insert_with(|| AssetInput::new(asset.file.clone()));
        }
        Ok(asset)
    }
}

impl IAssetLoader for PreviewAssetLoader {
    fn set_default_assembly(&self, assembly: &AssetAssembly) {
        self.registered.set_default_assembly(assembly);
    }

    fn exists(&self, uri: &UriRef, base: Option<&Url>) -> bool {
        match self.lookup(uri, base) {
            Ok(Some(asset)) => asset.file.is_file(),
            Ok(None) => self.registered.exists(uri, base),
            Err(_) => false,
        }
    }

    fn open(&self, uri: &UriRef, base: Option<&Url>) -> io::Result<Box<dyn AssetStream>> {
        Ok(self.open_and_get_assembly(uri, base)?.0)
    }

    fn open_and_get_assembly(
        &self,
        uri: &UriRef,
        base: Option<&Url>,
    ) -> io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        if let Some(asset) = self.lookup(uri, base)? {
            if !asset.file.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "The packaged asset {uri} is missing: {}",
                        asset.file.display()
                    ),
                ));
            }
            return Ok((
                Box::new(File::open(asset.file)?),
                AssetAssembly::new(asset.assembly),
            ));
        }
        self.registered.open_and_get_assembly(uri, base)
    }

    fn get_assembly(&self, uri: &UriRef, base: Option<&Url>) -> Option<AssetAssembly> {
        match self.lookup(uri, base) {
            Ok(Some(asset)) => Some(AssetAssembly::new(asset.assembly)),
            Ok(None) => self.registered.get_assembly(uri, base),
            Err(_) => None,
        }
    }

    fn get_assets(&self, uri: &UriRef, base: Option<&Url>) -> Vec<Url> {
        let registered = self.registered.get_assets(uri, base);
        let Ok(folder) = UriExtensions::ensure_absolute(uri, base) else {
            return registered;
        };
        let Ok(addresses) = self.assets.catalog.below(folder.as_str()) else {
            return Vec::new();
        };
        let mut seen = BTreeSet::new();
        addresses
            .into_iter()
            .filter_map(|address| Url::parse(&address).ok())
            .chain(registered)
            .filter(|url| seen.insert(url.clone()))
            .collect()
    }

    fn invalidate_assembly_cache(&self, name: &str) {
        self.registered.invalidate_assembly_cache(name);
    }

    fn invalidate_assembly_cache_all(&self) {
        self.registered.invalidate_assembly_cache_all();
    }
}
