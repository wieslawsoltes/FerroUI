use crate::utilities::Uri;
use std::fmt;
use std::io::{Read, Seek};
use std::sync::Arc;

/// A readable, seekable stream over the content of an asset.
pub trait AssetStream: Read + Seek {}

impl<T: Read + Seek> AssetStream for T {}

/// Identifies the unit assets are embedded in and looked up by: a crate that
/// registered assets (see [`register_assets`](crate::platform::register_assets)).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AssetAssembly {
    name: Arc<str>,
}

impl AssetAssembly {
    /// Identifies the crate with the given name.
    pub fn new(name: &str) -> Self {
        Self { name: Arc::from(name) }
    }

    /// The name of the crate.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for AssetAssembly {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// Loads assets compiled into the application binary.
///
/// The interface and its service registration are considered unstable; use
/// the functions of [`AssetLoader`](crate::platform::AssetLoader) instead.
pub trait IAssetLoader: 'static {
    /// Sets the default assembly from which to load embedded-resource
    /// (`resm:`) assets for which no assembly is specified.
    fn set_default_assembly(&self, assembly: &AssetAssembly);

    /// Checks if an asset with the specified URI exists.
    ///
    /// `base_uri` is a base URI to use if `uri` is relative.
    fn exists(&self, uri: &Uri, base_uri: Option<&Uri>) -> bool;

    /// Opens the asset with the requested URI.
    ///
    /// Fails with [`std::io::ErrorKind::NotFound`] when the asset could not
    /// be found.
    fn open(&self, uri: &Uri, base_uri: Option<&Uri>) -> std::io::Result<Box<dyn AssetStream>>;

    /// Opens the asset with the requested URI and returns the asset stream
    /// along with the assembly containing the asset.
    fn open_and_get_assembly(
        &self,
        uri: &Uri,
        base_uri: Option<&Uri>,
    ) -> std::io::Result<(Box<dyn AssetStream>, AssetAssembly)>;

    /// Extracts the assembly information from a URI.
    fn get_assembly(&self, uri: &Uri, base_uri: Option<&Uri>) -> Option<AssetAssembly>;

    /// Gets all assets of a folder and its subfolders that match the
    /// specified URI.
    fn get_assets(&self, uri: &Uri, base_uri: Option<&Uri>) -> Vec<Uri>;

    /// Removes the cached descriptor of the assembly with the given name, so
    /// that assets it registered afterwards become visible.
    fn invalidate_assembly_cache(&self, name: &str);

    /// Removes all cached assembly descriptors.
    fn invalidate_assembly_cache_all(&self);
}
