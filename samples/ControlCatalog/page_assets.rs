//! The hook through which the host of the catalog makes the assets of a page
//! available before the catalog creates the page.
//!
//! Not a port: upstream's catalog finds every asset in its assembly. The
//! desktop host embeds every asset in the binary and sets no hook. The
//! browser host (`samples/ControlCatalog.Browser`) starts the application
//! with the assets of the start-up only and fetches the asset bundle of a
//! page when the catalog is about to create the page (see
//! `build/page_bundles.rs`): the asset loader is synchronous, so what the
//! document of a page names has to be registered before the page is built.

use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

/// The completion of [`IPageAssets::ensure_page_assets`]: an error describes
/// why the assets are not available.
pub type PageAssetsFuture = Pin<Box<dyn Future<Output = Result<(), String>>>>;

/// What the host does before the catalog creates a page.
pub trait IPageAssets {
    /// Makes the assets of the page with the header `page` (the header of
    /// its entry in the page list) available to the asset loader: `None`
    /// when they are available already, otherwise the future of their
    /// arrival.
    fn ensure_page_assets(&self, page: &str) -> Option<PageAssetsFuture>;
}

thread_local! {
    static IMPLEMENTATION: RefCell<Option<Rc<dyn IPageAssets>>> = const { RefCell::new(None) };
}

/// The hook of the host, set once before the application starts.
pub struct PageAssets;

impl PageAssets {
    /// The hook of the host, when it set one.
    pub fn implementation() -> Option<Rc<dyn IPageAssets>> {
        IMPLEMENTATION.with(|implementation| implementation.borrow().clone())
    }

    pub fn set_implementation(value: Option<Rc<dyn IPageAssets>>) {
        IMPLEMENTATION.with(|implementation| *implementation.borrow_mut() = value);
    }

    /// What the catalog waits for before it creates the page with the
    /// header `page`: `None` when there is no hook or the assets are there.
    pub(crate) fn ensure(page: &str) -> Option<PageAssetsFuture> {
        Self::implementation()?.ensure_page_assets(page)
    }
}
