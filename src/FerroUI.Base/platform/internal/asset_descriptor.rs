use crate::platform::{AssetAssembly, AssetStream};
use std::io::Cursor;

/// An asset embedded in the application binary: describes one asset and gives access to its
/// content.
///
/// One type serves both kinds of embedded data (named resources and assets
/// addressed by path): each is a registered byte slice. Upstream declares the
/// contract as the internal interface `IAssetDescriptor`, which the port has no
/// other implementation of (docs/porting/data/upstream-names.toml).
pub(crate) struct EmbeddedAssetDescriptor {
    assembly: AssetAssembly,
    bytes: &'static [u8],
}

impl EmbeddedAssetDescriptor {
    pub fn new(assembly: AssetAssembly, bytes: &'static [u8]) -> Self {
        Self { assembly, bytes }
    }

    /// Opens the content of the asset (`IAssetDescriptor.GetStream`).
    pub fn get_stream(&self) -> Box<dyn AssetStream> {
        Box::new(Cursor::new(self.bytes))
    }

    /// The assembly that contains the asset (`IAssetDescriptor.Assembly`).
    pub fn assembly(&self) -> &AssetAssembly {
        &self.assembly
    }
}
