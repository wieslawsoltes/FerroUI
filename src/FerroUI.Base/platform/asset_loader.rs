use super::{AssetAssembly, AssetStream, IAssetLoader};
use crate::utilities::Uri;
use crate::{FerroLocator, LocatorExtensions};
use std::rc::Rc;

/// The scheme of the URIs that identify assets registered by a crate:
/// `<scheme>://<crate name>/<path>`.
pub const ASSET_SCHEME: &str = "ferres";

/// Provides access to the assets compiled into the application binary,
/// through the registered [`IAssetLoader`].
pub struct AssetLoader;

impl AssetLoader {
    fn get_asset_loader() -> Rc<dyn IAssetLoader> {
        FerroLocator::current().get_required_service::<dyn IAssetLoader>()
    }

    /// Sets the default assembly from which to load embedded-resource
    /// (`resm:`) assets for which no assembly is specified.
    pub fn set_default_assembly(assembly: &AssetAssembly) {
        Self::get_asset_loader().set_default_assembly(assembly)
    }

    /// Checks if an asset with the specified URI exists.
    pub fn exists(uri: &Uri, base_uri: Option<&Uri>) -> bool {
        Self::get_asset_loader().exists(uri, base_uri)
    }

    /// Opens the asset with the requested URI.
    pub fn open(uri: &Uri, base_uri: Option<&Uri>) -> std::io::Result<Box<dyn AssetStream>> {
        Self::get_asset_loader().open(uri, base_uri)
    }

    /// Opens the asset with the requested URI and returns the asset stream
    /// along with the assembly containing the asset.
    pub fn open_and_get_assembly(
        uri: &Uri,
        base_uri: Option<&Uri>,
    ) -> std::io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        Self::get_asset_loader().open_and_get_assembly(uri, base_uri)
    }

    /// Extracts the assembly information from a URI.
    pub fn get_assembly(uri: &Uri, base_uri: Option<&Uri>) -> Option<AssetAssembly> {
        Self::get_asset_loader().get_assembly(uri, base_uri)
    }

    /// Gets all assets of a folder and its subfolders that match the
    /// specified URI.
    pub fn get_assets(uri: &Uri, base_uri: Option<&Uri>) -> Vec<Uri> {
        Self::get_asset_loader().get_assets(uri, base_uri)
    }

    /// Removes the cached descriptor of the assembly with the given name.
    pub fn invalidate_assembly_cache(name: &str) {
        Self::get_asset_loader().invalidate_assembly_cache(name)
    }

    /// Removes all cached assembly descriptors.
    pub fn invalidate_assembly_cache_all() {
        Self::get_asset_loader().invalidate_assembly_cache_all()
    }
}
