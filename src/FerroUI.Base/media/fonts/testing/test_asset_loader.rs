use std::cell::RefCell;
use std::io::{self, Cursor};

use crate::platform::{AssetAssembly, AssetStream, IAssetLoader};
use crate::utilities::{Uri, UriExtensions, UriKind};

/// An asset loader over assets held in memory.
#[derive(Default)]
pub struct TestAssetLoader {
    assets: RefCell<Vec<(Uri, Vec<u8>)>>,
}

#[allow(dead_code)] // the full harness API is kept for the text tests built on top
impl TestAssetLoader {
    pub fn new() -> Self {
        Self::default()
    }

    /// An asset loader holding the given `(uri, data)` assets.
    pub fn with_assets(assets: &[(&str, &[u8])]) -> Self {
        let loader = Self::new();

        for (uri, data) in assets {
            loader.add_asset(uri, data.to_vec());
        }

        loader
    }

    /// Adds (or replaces) an asset.
    ///
    /// Panics when `uri` is not a valid URI.
    pub fn add_asset(&self, uri: &str, data: Vec<u8>) {
        let uri = Uri::new(uri, UriKind::RelativeOrAbsolute).expect("a valid asset uri");
        let mut assets = self.assets.borrow_mut();

        match assets.iter_mut().find(|(existing, _)| *existing == uri) {
            Some(existing) => existing.1 = data,
            None => assets.push((uri, data)),
        }
    }
}

impl IAssetLoader for TestAssetLoader {
    fn set_default_assembly(&self, _assembly: &AssetAssembly) {}

    fn open_and_get_assembly(
        &self,
        uri: &Uri,
        base_uri: Option<&Uri>,
    ) -> io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        Ok((self.open(uri, base_uri)?, AssetAssembly::new("TestAssets")))
    }

    fn get_assembly(&self, _uri: &Uri, _base_uri: Option<&Uri>) -> Option<AssetAssembly> {
        None
    }

    fn exists(&self, uri: &Uri, _base_uri: Option<&Uri>) -> bool {
        self.assets.borrow().iter().any(|(existing, _)| existing == uri)
    }

    fn open(&self, uri: &Uri, _base_uri: Option<&Uri>) -> io::Result<Box<dyn AssetStream>> {
        match self.assets.borrow().iter().find(|(existing, _)| existing == uri) {
            Some((_, data)) => Ok(Box::new(Cursor::new(data.clone()))),
            None => Err(io::Error::new(io::ErrorKind::NotFound, format!("The resource {uri} could not be found."))),
        }
    }

    fn get_assets(&self, uri: &Uri, _base_uri: Option<&Uri>) -> Vec<Uri> {
        let abs_path = UriExtensions::get_unescape_absolute_path(uri);

        self.assets
            .borrow()
            .iter()
            .filter(|(asset, _)| asset.is_absolute_uri() && UriExtensions::get_unescape_absolute_path(asset).contains(&abs_path))
            .map(|(asset, _)| asset.clone())
            .collect()
    }

    fn invalidate_assembly_cache(&self, _name: &str) {}

    fn invalidate_assembly_cache_all(&self) {}
}
