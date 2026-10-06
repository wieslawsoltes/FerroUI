//! Tests of the split of the assets into the bundles the browser host loads
//! on demand (`build/page_bundles.rs`).
//!
//! Not ports: the split is an addition of the port.

use super::page_assets::{listed_pages, show_main_window};
use super::support::*;
use crate::view_models::MainWindowViewModel;
use ferroui_base::platform::{AssetAssembly, AssetStream, IAssetLoader, StandardAssetLoader};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{Uri, UriExtensions};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

include!(concat!(env!("OUT_DIR"), "/page_assets.rs"));

/// The standard asset loader, recording the assets of the catalog that are
/// opened (rooted paths).
struct RecordingAssetLoader {
    inner: StandardAssetLoader,
    opened: RefCell<BTreeSet<String>>,
}

impl RecordingAssetLoader {
    fn record(&self, uri: &Uri, base_uri: Option<&Uri>) {
        if let Ok(uri) = UriExtensions::ensure_absolute(uri, base_uri) {
            if UriExtensions::is_asset(&uri) && UriExtensions::authority(&uri).eq_ignore_ascii_case(crate::ASSEMBLY.name) {
                self.opened.borrow_mut().insert(UriExtensions::get_unescape_absolute_path(&uri));
            }
        }
    }
}

impl IAssetLoader for RecordingAssetLoader {
    fn set_default_assembly(&self, assembly: &AssetAssembly) {
        self.inner.set_default_assembly(assembly)
    }

    fn exists(&self, uri: &Uri, base_uri: Option<&Uri>) -> bool {
        self.inner.exists(uri, base_uri)
    }

    fn open(&self, uri: &Uri, base_uri: Option<&Uri>) -> std::io::Result<Box<dyn AssetStream>> {
        self.record(uri, base_uri);
        self.inner.open(uri, base_uri)
    }

    fn open_and_get_assembly(&self, uri: &Uri, base_uri: Option<&Uri>) -> std::io::Result<(Box<dyn AssetStream>, AssetAssembly)> {
        self.record(uri, base_uri);
        self.inner.open_and_get_assembly(uri, base_uri)
    }

    fn get_assembly(&self, uri: &Uri, base_uri: Option<&Uri>) -> Option<AssetAssembly> {
        self.inner.get_assembly(uri, base_uri)
    }

    fn get_assets(&self, uri: &Uri, base_uri: Option<&Uri>) -> Vec<Uri> {
        self.inner.get_assets(uri, base_uri)
    }

    fn invalidate_assembly_cache(&self, name: &str) {
        self.inner.invalidate_assembly_cache(name)
    }

    fn invalidate_assembly_cache_all(&self) {
        self.inner.invalidate_assembly_cache_all()
    }
}
/// Every asset of the catalog that a page of the page list opens while it is
/// created and shown by the main view, or while one of the samples of its
/// entry is created and shown, is in the start-up bundle or in a bundle of
/// the page: the browser host registers no other bundle before it creates
/// the page. (The demos a gallery page opens on a click are not created
/// here.)
#[test]
fn every_page_finds_its_assets_in_the_start_up_bundle_and_its_own_bundles() {
    let loader = Rc::new(RecordingAssetLoader { inner: StandardAssetLoader::new(None), opened: RefCell::new(BTreeSet::new()) });
    let _app = start_catalog_application_with(Some(loader.clone()));
    let view_model = MainWindowViewModel::new();
    let window = show_main_window(&view_model);
    let opened_assets = || -> BTreeSet<String> {
        std::mem::take(&mut *loader.opened.borrow_mut()).into_iter().filter(|path| !path.ends_with(".xaml")).collect()
    };
    let at_start = opened_assets();
    let outside: Vec<&String> = at_start.iter().filter(|path| !STARTUP_ASSETS.contains(&path.as_str())).collect();
    assert!(outside.is_empty(), "the start-up opens {outside:?}, which are not in the start-up bundle");

    let mut missing = Vec::new();
    let mut opened_by_pages = 0;
    for item in listed_pages(&view_model) {
        let header = item.header();
        view_model.navigate_to_item(&item);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(view_model.current_page_item().is_some_and(|current| Rc::ptr_eq(&current, &item)), "{header} is not shown");
        for sample in item.samples().iter().flat_map(|samples| samples.iter()) {
            // A sample whose document is listed in excluded.txt does not load yet (its creation panics).
            let factory = sample.factory();
            let Ok(control) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| factory())) else { continue };
            let sample_window = ferroui_controls::Window::new();
            sample_window.set_width(1100.0);
            sample_window.set_height(800.0);
            sample_window.set_content(Some(ferroui_controls::Control::boxed(&control)));
            sample_window.show();
            Dispatcher::ui_thread().run_jobs(None);
            sample_window.close();
        }

        let of_page = PAGE_ASSETS.iter().find(|(page, _)| *page == header).map_or(&[][..], |(_, assets)| *assets);
        for asset in opened_assets() {
            opened_by_pages += 1;
            if !STARTUP_ASSETS.contains(&asset.as_str()) && !of_page.contains(&asset.as_str()) {
                missing.push(format!("{header}: {asset}"));
            }
        }
    }
    assert!(opened_by_pages > 10, "the pages opened {opened_by_pages} assets: the recording does not work");
    assert!(missing.is_empty(), "assets that pages open but that are in no bundle of the page:\n{}", missing.join("\n"));
    window.close();
}

/// The split puts the photographs and fonts of the pages outside the
/// start-up bundle, and the large CJK font in a bundle of the pages that
/// build the font collection of `Assets/Fonts`.
#[test]
fn the_start_up_bundle_has_only_what_the_start_up_needs() {
    for asset in ["/Assets/icon.ico", "/Assets/banner-bg.png", "/Assets/logo1.png"] {
        assert!(STARTUP_ASSETS.contains(&asset), "{asset} is not in the start-up bundle");
    }
    for asset in ["/Assets/Fonts/WenQuanYiMicroHei-01.ttf", "/Assets/image1.jpg", "/Assets/Sanctuary/main_hero.jpg"] {
        assert!(!STARTUP_ASSETS.contains(&asset), "{asset} is in the start-up bundle");
    }
    let pages_of = |asset: &str| -> Vec<&str> {
        PAGE_ASSETS.iter().filter(|(_, assets)| assets.contains(&asset)).map(|(page, _)| *page).collect()
    };
    assert_eq!(vec!["TextBlock", "TextBox"], pages_of("/Assets/Fonts/WenQuanYiMicroHei-01.ttf"));
    assert_eq!(vec!["Container Queries"], pages_of("/Assets/image1.jpg"));
}
