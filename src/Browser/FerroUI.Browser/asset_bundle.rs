//! Assets that the host page downloads next to the WebAssembly module.
//!
//! Not in the managed original, whose runtime downloads the resources of an
//! application with its assemblies. Here assets are embedded in the module,
//! except those an application ships as an asset bundle
//! ([`register_asset_bundle`](ferroui_base::platform::register_asset_bundle)):
//! the host page fetches the bundle and passes it to [`register_asset_bundle`]
//! before it starts the application, so that a large set of pictures and
//! fonts neither counts against the size of the module nor waits for its
//! compilation.

use wasm_bindgen::prelude::*;

/// Registers the assets of an asset bundle with the asset loader; the host
/// page calls it with the content of the bundle file before the application
/// starts. Returns the number of assets, or throws with a description of the
/// defect of the bundle.
#[wasm_bindgen(js_name = registerAssetBundle)]
pub fn register_asset_bundle(bundle: Vec<u8>) -> Result<u32, String> {
    let bundle: &'static [u8] = Box::leak(bundle.into_boxed_slice());
    ferroui_base::platform::register_asset_bundle(bundle).map(|count| count as u32)
}
