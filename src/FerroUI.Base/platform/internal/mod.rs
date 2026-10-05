//! Implementation details of the standard asset loader.

mod assembly_descriptor;
mod assembly_descriptor_resolver;
mod asset_descriptor;
mod asset_registry;
mod unmanaged_blob;

#[cfg(test)]
pub(crate) use assembly_descriptor::AssetMap;
pub(crate) use assembly_descriptor::{AssemblyDescriptor, IAssemblyDescriptor};
pub(crate) use assembly_descriptor_resolver::{AssemblyDescriptorResolver, IAssemblyDescriptorResolver};
pub(crate) use asset_descriptor::{EmbeddedAssetDescriptor, IAssetDescriptor};
pub use asset_registry::{register_asset_bundle, register_assets, register_manifest_resources, ASSET_BUNDLE_MAGIC};
pub(crate) use asset_registry::{registered_assembly, registered_assembly_names, RegisteredAssembly};
pub(crate) use unmanaged_blob::UnmanagedBlob;
