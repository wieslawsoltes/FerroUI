use crate::platform::{AssetAssembly, AssetStream};
use std::io::Cursor;

/// Describes one asset and gives access to its content.
pub(crate) trait IAssetDescriptor {
    /// Opens the content of the asset.
    fn get_stream(&self) -> Box<dyn AssetStream>;

    /// The assembly that contains the asset.
    fn assembly(&self) -> &AssetAssembly;
}

/// An asset embedded in the application binary.
///
/// One type serves both kinds of embedded data (named resources and assets
/// addressed by path): each is a registered byte slice.
pub(crate) struct EmbeddedAssetDescriptor {
    assembly: AssetAssembly,
    bytes: &'static [u8],
}

impl EmbeddedAssetDescriptor {
    pub fn new(assembly: AssetAssembly, bytes: &'static [u8]) -> Self {
        Self { assembly, bytes }
    }
}

impl IAssetDescriptor for EmbeddedAssetDescriptor {
    fn get_stream(&self) -> Box<dyn AssetStream> {
        Box::new(Cursor::new(self.bytes))
    }

    fn assembly(&self) -> &AssetAssembly {
        &self.assembly
    }
}
