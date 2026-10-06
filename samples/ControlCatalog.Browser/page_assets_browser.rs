//! The asset bundles of the pages of the catalog, fetched by the host page
//! when the catalog is about to create a page that needs them.
//!
//! Not a port: the managed original has every asset in its assembly, which
//! the runtime downloads before `Main`. Here the build script of the sample
//! splits the assets by the pages that use them (`samples/ControlCatalog/
//! build/page_bundles.rs`): the host page registers the start-up bundle
//! before it starts the application, and `wwwroot/page-assets.js` fetches
//! the bundles of a page when [`BrowserPageAssets`] is asked for them, and
//! the others in the background after the first frame. The asset loader is
//! unchanged and synchronous: the catalog awaits the bundles of a page
//! before it creates the page (`control_catalog::PageAssets`).

use control_catalog::{IPageAssets, PageAssetsFuture};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_browser::interop::completion_helper::await_promise;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./page-assets.js")]
extern "C" {
    /// `null` when the bundles of the page are registered, otherwise a
    /// promise that settles once they are.
    #[wasm_bindgen(js_name = ensurePageAssets)]
    fn ensure_page_assets(page: &str) -> JsValue;
}

/// The hook of the catalog for the browser: asks the host page for the
/// bundles of the page.
pub struct BrowserPageAssets;

impl IPageAssets for BrowserPageAssets {
    fn ensure_page_assets(&self, page: &str) -> Option<PageAssetsFuture> {
        let promise = ensure_page_assets(page);
        if promise.is_null() || promise.is_undefined() {
            return None;
        }
        Some(Box::pin(async move { await_promise(&promise).await.map(drop).map_err(|error| error.to_string()) }))
    }
}

/// Registers an asset bundle the host page fetched after the application
/// started, and has the asset loader describe the assemblies again, so that
/// it finds the new assets (it keeps what an assembly had registered when
/// it was first asked for it). Returns the number of assets, or throws with
/// a description of the defect of the bundle.
#[wasm_bindgen(js_name = registerPageAssetBundle)]
pub fn register_page_asset_bundle(bundle: Vec<u8>) -> Result<u32, String> {
    let count = ferroui_browser::register_asset_bundle(bundle)?;
    if let Some(asset_loader) = FerroLocator::current().get_service::<dyn IAssetLoader>() {
        asset_loader.invalidate_assembly_cache_all();
    }
    Ok(count)
}
