//! The asset files of the pages of the catalog, fetched by the host page
//! when the catalog is about to create a page that needs them.
//!
//! Not a port: the managed original has every asset in its assembly, which
//! the runtime downloads before `Main`. Here the build script of the sample
//! copies each asset to the site as a file of its own and lists the files
//! the start-up and each page use (`samples/ControlCatalog/build/
//! page_files.rs`): the host page registers the start-up files before it
//! starts the application, and `wwwroot/page-assets.js` fetches the files of
//! a page when [`BrowserPageAssets`] is asked for them, and the others in
//! the background after the first frame. The asset loader is unchanged and
//! synchronous: the catalog awaits the files of a page before it creates the
//! page (`control_catalog::PageAssets`).

use control_catalog::{IPageAssets, PageAssetsFuture};
use ferroui_base::platform::{register_assets, IAssetLoader};
use ferroui_base::utilities::{Uri, UriExtensions, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_browser::interop::completion_helper::await_promise;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./page-assets.js")]
extern "C" {
    /// `null` when the files of the page are registered, otherwise a
    /// promise that settles once they are.
    #[wasm_bindgen(js_name = ensurePageAssets)]
    fn ensure_page_assets(page: &str) -> JsValue;
}

/// The hook of the catalog for the browser: asks the host page for the
/// files of the page.
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

/// Registers the content of one asset file the host page fetched, under the
/// URI of the asset (`ferres://ControlCatalog/Assets/x.png`), before or
/// after the application started. The asset loader describes the assembly
/// of the asset again, so that it finds the new asset (it keeps what an
/// assembly had registered when it was first asked for it). Throws when the
/// URI is not the absolute URI of an asset.
// Deviation (browser-platform.md section 14): assets registered one by one, after the application started too.
#[wasm_bindgen(js_name = registerAsset)]
pub fn register_asset(uri: &str, content: Vec<u8>) -> Result<(), String> {
    let parsed = Uri::new(uri, UriKind::Absolute).map_err(|error| format!("{uri}: {error}"))?;
    if !UriExtensions::is_asset(&parsed) {
        return Err(format!("{uri} is not the URI of an asset"));
    }
    let assembly = UriExtensions::authority(&parsed);
    let path = UriExtensions::get_unescape_absolute_path(&parsed);
    // The registry keeps the content for the life of the page, as it keeps embedded assets.
    let content: &'static [u8] = Box::leak(content.into_boxed_slice());
    register_assets(assembly, &[(path.as_str(), content)]);
    if let Some(asset_loader) = FerroLocator::current().get_service::<dyn IAssetLoader>() {
        asset_loader.invalidate_assembly_cache(assembly);
    }
    Ok(())
}
