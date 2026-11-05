use std::cell::RefCell;
use std::io::{self, Cursor};

use crate::platform::{AssetAssembly, AssetStream, IAssetLoader};
use crate::utilities::{UriExtensions, UriRef};

/// An asset loader over assets held in memory.
///
/// An asset is found by its reference without the fragment (decision O5 of
/// docs/idiomatic-rust-signatures.md: a font asset is added with the name of
/// its family, `resm:A.B.ttf?assembly=A#Name`).
#[derive(Default)]
pub struct TestAssetLoader {
    assets: RefCell<Vec<(UriRef, Vec<u8>)>>,
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
    /// Panics when `uri` is not a valid URI reference.
    pub fn add_asset(&self, uri: &str, data: Vec<u8>) {
        let uri = UriRef::parse(uri).expect("a valid asset uri");
        let key = uri.without_fragment();
        let mut assets = self.assets.borrow_mut();

        match assets.iter_mut().find(|(existing, _)| existing.without_fragment() == key) {
            Some(existing) => existing.1 = data,
            None => assets.push((uri, data)),
        }
    }

    fn find(&self, uri: &UriRef) -> Option<Vec<u8>> {
        let key = uri.without_fragment();

        self.assets.borrow().iter().find(|(existing, _)| existing.without_fragment() == key).map(|(_, data)| data.clone())
    }
}

impl IAssetLoader for TestAssetLoader {
    fn set_default_assembly(&self, _assembly: &AssetAssembly) {}

    fn open_and_get_assembly(
        &self,
        uri: &UriRef,
        base_uri: Option<&url::Url>,
    ) -> io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        Ok((self.open(uri, base_uri)?, AssetAssembly::new("TestAssets")))
    }

    fn get_assembly(&self, _uri: &UriRef, _base_uri: Option<&url::Url>) -> Option<AssetAssembly> {
        None
    }

    fn exists(&self, uri: &UriRef, _base_uri: Option<&url::Url>) -> bool {
        self.find(uri).is_some()
    }

    fn open(&self, uri: &UriRef, _base_uri: Option<&url::Url>) -> io::Result<Box<dyn AssetStream>> {
        match self.find(uri) {
            Some(data) => Ok(Box::new(Cursor::new(data))),
            None => Err(io::Error::new(io::ErrorKind::NotFound, format!("The resource {uri} could not be found."))),
        }
    }

    fn get_assets(&self, uri: &UriRef, _base_uri: Option<&url::Url>) -> Vec<url::Url> {
        let Some(uri) = uri.as_url() else { return Vec::new() };
        let abs_path = UriExtensions::get_unescape_absolute_path(uri);

        self.assets
            .borrow()
            .iter()
            .filter_map(|(asset, _)| asset.as_url())
            .filter(|asset| UriExtensions::get_unescape_absolute_path(asset).contains(&abs_path))
            .cloned()
            .collect()
    }

    fn invalidate_assembly_cache(&self, _name: &str) {}

    fn invalidate_assembly_cache_all(&self) {}
}
