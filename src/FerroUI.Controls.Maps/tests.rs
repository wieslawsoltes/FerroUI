//! The tile store and the control, without the network.

use crate::tile_store::{TileStore, MAX_ATTEMPTS, MAX_LOADS};
use crate::*;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// Answers from a table; an address it does not have is `NotFound`.
#[derive(Default)]
struct FakeLoader {
    tiles: RefCell<HashMap<String, Result<Vec<u8>, TileError>>>,
    asked: RefCell<Vec<String>>,
}

impl ITileLoader for FakeLoader {
    fn load(&self, address: &str, _tile: TileId) -> TileFuture {
        self.asked.borrow_mut().push(address.to_owned());
        let answer = self
            .tiles
            .borrow()
            .get(address)
            .cloned()
            .unwrap_or(Err(TileError::NotFound));
        Box::pin(async move { answer })
    }
}

fn source() -> Rc<RasterTileSource> {
    Rc::new(RasterTileSource::new("https://tiles.example.test/{z}/{x}/{y}.png", "Example").unwrap())
}

fn tile(zoom: u8, x: u32, y: u32) -> TileId {
    TileId { zoom, x, y }
}

fn run_jobs() {
    Dispatcher::ui_thread().run_jobs(None);
}

fn store(loader: &Rc<FakeLoader>) -> TileStore<Vec<u8>> {
    let store = TileStore::new(1024, |bytes: &[u8]| {
        (bytes != b"bad").then(|| (bytes.to_vec(), bytes.len() as u64))
    });
    store.set_source(Some(source()), Some(loader.clone()));
    store
}

fn address(id: TileId) -> String {
    source().address(id)
}

#[test]
fn a_loaded_tile_is_kept_and_reported() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let loader = Rc::new(FakeLoader::default());
    loader
        .tiles
        .borrow_mut()
        .insert(address(tile(2, 1, 1)), Ok(b"image".to_vec()));
    let store = store(&loader);
    let arrived = Rc::new(Cell::new(0));
    let count = arrived.clone();
    store.set_on_loaded(move || count.set(count.get() + 1));
    store.request([tile(2, 1, 1)]);
    run_jobs();
    assert_eq!(arrived.get(), 1);
    let (value, part) = store.best(tile(2, 1, 1)).unwrap();
    assert_eq!(value.as_slice(), b"image");
    assert_eq!(part, (0.0, 0.0, 1.0));
    store.request([tile(2, 1, 1)]);
    run_jobs();
    assert_eq!(
        loader.asked.borrow().len(),
        1,
        "a tile in memory is not loaded again"
    );
}

#[test]
fn a_tile_that_is_not_there_is_drawn_from_its_ancestor() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let loader = Rc::new(FakeLoader::default());
    loader
        .tiles
        .borrow_mut()
        .insert(address(tile(3, 3, 5)), Ok(b"parent".to_vec()));
    let store = store(&loader);
    store.request([tile(3, 3, 5)]);
    run_jobs();
    let (value, part) = store.best(tile(5, 13, 22)).unwrap();
    assert_eq!(value.as_slice(), b"parent");
    assert_eq!(part, (0.25, 0.5, 0.25));
    assert!(store.best(tile(5, 0, 0)).is_none());
}

#[test]
fn a_missing_tile_and_bad_data_are_not_asked_for_again() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let loader = Rc::new(FakeLoader::default());
    loader
        .tiles
        .borrow_mut()
        .insert(address(tile(1, 0, 0)), Ok(b"bad".to_vec()));
    let store = store(&loader);
    for _ in 0..3 {
        store.request([tile(1, 0, 0), tile(1, 1, 1)]);
        run_jobs();
    }
    assert_eq!(loader.asked.borrow().len(), 2);
    assert!(store.best(tile(1, 0, 0)).is_none());
}

#[test]
fn a_failed_load_is_tried_a_few_times() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let loader = Rc::new(FakeLoader::default());
    loader.tiles.borrow_mut().insert(
        address(tile(1, 0, 0)),
        Err(TileError::Failed("offline".into())),
    );
    let store = store(&loader);
    for _ in 0..(MAX_ATTEMPTS + 3) {
        store.request([tile(1, 0, 0)]);
        run_jobs();
    }
    assert_eq!(loader.asked.borrow().len(), usize::from(MAX_ATTEMPTS));
}

/// A load that waits until the test lets it go.
#[derive(Default)]
struct HeldLoader {
    gates: RefCell<Vec<ferroui_system::task::oneshot::Sender<()>>>,
}

impl ITileLoader for HeldLoader {
    fn load(&self, _address: &str, _tile: TileId) -> TileFuture {
        let (gate, wait) = ferroui_system::task::oneshot::channel();
        self.gates.borrow_mut().push(gate);
        Box::pin(async move {
            let _ = wait.await;
            Ok(b"image".to_vec())
        })
    }
}

#[test]
fn only_a_few_tiles_load_at_once_and_a_new_source_drops_the_old_loads() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let loader = Rc::new(HeldLoader::default());
    let store: TileStore<Vec<u8>> = TileStore::new(1024, |bytes: &[u8]| {
        Some((bytes.to_vec(), bytes.len() as u64))
    });
    store.set_source(Some(source()), Some(loader.clone()));
    store.request((0..20).map(|x| tile(5, x, 0)));
    run_jobs();
    assert_eq!(store.loading_count(), MAX_LOADS);

    store.set_source(Some(source()), Some(loader.clone()));
    assert_eq!(store.loading_count(), 0);
    for gate in loader.gates.borrow_mut().drain(..) {
        let _ = gate.send(());
    }
    run_jobs();
    assert!(
        store.best(tile(5, 0, 0)).is_none(),
        "a tile of the old source is dropped"
    );
}

#[test]
fn the_properties_and_the_camera_follow_each_other() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let map = MapControl::new();
    assert_eq!(map.zoom_level(), 2.0);
    map.set_zoom_level(40.0);
    assert_eq!(map.zoom_level(), MAX_ZOOM, "the level is limited");
    map.set_center(BasicGeoposition {
        latitude: 89.0,
        longitude: 190.0,
    });
    let center = map.center();
    assert!((center.latitude - MAX_LATITUDE).abs() < 1e-9);
    assert!((center.longitude + 170.0).abs() < 1e-9);
    map.set_map_service_token(Some("token".to_owned()));
    assert_eq!(map.map_service_token().as_deref(), Some("token"));
}

#[test]
fn layers_are_added_and_removed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let map = MapControl::new();
    let layer = MapElementsLayer::new();
    layer.add(MapElement::Icon(MapIcon::new(BasicGeoposition::default())));
    map.add_layer(layer.clone());
    assert_eq!(map.layers().len(), 1);
    assert!(map.remove_layer(&layer));
    assert!(!map.remove_layer(&layer));
    assert!(map.layers().is_empty());
}

#[test]
fn the_disk_cache_answers_without_the_loader() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let folder = std::env::temp_dir().join(format!("ferroui-map-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let id = tile(2, 1, 1);
    {
        let mut cache = DiskTileCache::open(&folder, "source", 1024).unwrap();
        cache.put(id, b"stored").unwrap();
    }
    let loader = Rc::new(FakeLoader::default());
    let store = store(&loader);
    store.set_disk(Some(
        crate::tile_store::disk::DiskWorker::open(&folder, "source", 1024).unwrap(),
    ));
    store.request([id]);
    let start = std::time::Instant::now();
    while store.best(id).is_none() {
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "the stored tile did not arrive"
        );
        run_jobs();
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(store.best(id).unwrap().0.as_slice(), b"stored");
    assert!(loader.asked.borrow().is_empty());
    drop(store);
    let _ = std::fs::remove_dir_all(&folder);
}

fn key(map: &MapControl, key: ferroui_base::input::Key) -> bool {
    let mut args = ferroui_base::input::KeyEventArgs::new();
    args.key = key;
    <MapControl as ferroui_base::input::InputElementImpl>::on_key_down(map, &args);
    args.handled()
}

#[test]
fn the_arrow_keys_move_the_map_and_a_disabled_mode_stops_them() {
    use ferroui_base::input::Key;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let map = MapControl::new();
    map.set_zoom_level(4.0);
    let start = map.center().longitude;
    assert!(key(&map, Key::Right));
    assert!(map.center().longitude > start, "the view moves east");
    let moved = map.center().longitude;
    map.set_pan_interaction_mode(MapInteractionMode::Disabled);
    assert!(!key(&map, Key::Right));
    assert_eq!(map.center().longitude, moved);
    map.set_zoom_interaction_mode(MapInteractionMode::Disabled);
    assert!(!key(&map, Key::Add));
    assert_eq!(map.zoom_level(), 4.0);
}

#[test]
fn the_grid_source_needs_no_network() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let loader: SharedTileLoader = Rc::new(GridTileLoader);
    let store: TileStore<usize> = TileStore::new(1 << 24, |bytes: &[u8]| {
        Some((bytes.len(), bytes.len() as u64))
    });
    store.set_source(Some(Rc::new(grid_tile_source())), Some(loader));
    store.request([tile(3, 1, 2)]);
    run_jobs();
    assert!(*store.best(tile(3, 1, 2)).unwrap().0 > 256 * 256 * 4);
}
