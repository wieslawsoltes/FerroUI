//! The tile store and the control, without the network.

use crate::tile_store::{TileStore, MAX_ATTEMPTS, MAX_LOADS};
use crate::*;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static ELEMENT_PEERS_MADE: Cell<usize> = const { Cell::new(0) };
}

/// Counts the peers of elements this thread made: none may exist without a client.
pub(crate) fn count_element_peer() {
    ELEMENT_PEERS_MADE.with(|count| count.set(count.get() + 1));
}

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

#[test]
fn each_source_has_its_own_folder_in_the_disk_cache() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let folder = std::env::temp_dir().join(format!("ferroui-map-sources-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let first = Rc::new(RasterTileSource::new("https://a.example.test/{z}/{x}/{y}", "").unwrap());
    // The same address for the first tile, another for every other one.
    let second = Rc::new(RasterTileSource::new("https://a.example.test/{z}/{y}/{x}", "").unwrap());
    assert_ne!(first.cache_key(), second.cache_key());
    let map = MapControl::new();
    map.set_disk_cache(Some(&folder), 1024).unwrap();
    map.set_tile_source(Some(first.clone()), None);
    assert!(folder.join(first.cache_key()).is_dir());
    assert!(!folder.join(second.cache_key()).exists());
    map.set_tile_source(Some(second.clone()), None);
    assert!(folder.join(second.cache_key()).is_dir());
    map.set_disk_cache(None, 0).unwrap();
    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn the_speed_of_a_release_comes_from_the_end_of_the_drag() {
    use ferroui_base::Point;
    use std::collections::VecDeque;
    let samples: VecDeque<(u64, Point)> = [
        (0, Point::new(0.0, 0.0)),
        (500, Point::new(0.0, 0.0)),
        (550, Point::new(50.0, 0.0)),
        (600, Point::new(100.0, -20.0)),
    ]
    .into();
    assert_eq!(MapControl::release_speed(&samples), (1000.0, -200.0));
    assert_eq!(MapControl::release_speed(&VecDeque::new()), (0.0, 0.0));
    let one: VecDeque<(u64, Point)> = [(10, Point::new(1.0, 1.0))].into();
    assert_eq!(MapControl::release_speed(&one), (0.0, 0.0));
}

mod input {
    use super::*;
    use crate::map_control::{content_transform, pointer_turn};
    use ferroui_base::input::{
        InputElement, Key, KeyEventArgs, KeyModifiers, MouseButton, PinchEndedEventArgs,
        PinchEventArgs, Pointer, PointerEventArgs, PointerPointProperties, PointerPressedEventArgs,
        PointerReleasedEventArgs, PointerType, PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_base::interactivity::Interactive;
    use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
    use ferroui_base::{Point, Ref, Size, Visual};
    use ferroui_controls::automation::peers::{
        AutomationControlType, AutomationPeer, ControlAutomationPeer,
    };
    use ferroui_controls::automation::provider::{IInvokeProvider, IRootProvider};
    use ferroui_controls::testing::MockWindowingPlatform;
    use ferroui_controls::{Control, Window};

    const WIDTH: f64 = 400.0;
    const HEIGHT: f64 = 300.0;

    /// A map in a root: a pointer position is relative to the visual tree.
    struct Placed {
        _window: Ref<Window>,
        map: Ref<MapControl>,
    }

    impl std::ops::Deref for Placed {
        type Target = Ref<MapControl>;

        fn deref(&self) -> &Ref<MapControl> {
            &self.map
        }
    }

    fn map() -> Placed {
        let placed = place_in_window(MapControl::new());
        placed.set_zoom_level(4.0);
        placed
    }

    fn place_in_window(map: Ref<MapControl>) -> Placed {
        map.set_width(WIDTH);
        map.set_height(HEIGHT);
        map.set_horizontal_alignment(HorizontalAlignment::Left);
        map.set_vertical_alignment(VerticalAlignment::Top);
        let window = Window::with_impl(MockWindowingPlatform::create_window_mock());
        window.set_content(Some(Control::boxed(map.clone())));
        window.show();
        window.update_layout();
        assert_eq!(map.bounds().size(), Size::new(WIDTH, HEIGHT));
        Placed {
            _window: window,
            map,
        }
    }

    fn root_of(map: &Ref<MapControl>) -> Ref<Visual> {
        map.visual_root().expect("the map is in a root")
    }

    /// A pointer that presses, moves and releases on the map, as the input system does.
    struct Finger {
        pointer: Rc<Pointer>,
        modifiers: KeyModifiers,
        buttons: RawInputModifiers,
        button: MouseButton,
        time: Cell<u64>,
    }

    impl Finger {
        fn new(kind: PointerType, button: MouseButton) -> Self {
            let buttons = match button {
                MouseButton::Right => RawInputModifiers::RIGHT_MOUSE_BUTTON,
                _ => RawInputModifiers::LEFT_MOUSE_BUTTON,
            };
            Self {
                pointer: Pointer::new(Pointer::get_next_free_id(), kind, true),
                modifiers: KeyModifiers::NONE,
                buttons,
                button,
                time: Cell::new(1000),
            }
        }

        fn stamp(&self) -> u64 {
            self.time.set(self.time.get() + 16);
            self.time.get()
        }

        fn down(&self, map: &Ref<MapControl>, at: (f64, f64)) {
            let kind = match self.button {
                MouseButton::Right => PointerUpdateKind::RightButtonPressed,
                _ => PointerUpdateKind::LeftButtonPressed,
            };
            let root = root_of(map);
            map.raise_event(&PointerPressedEventArgs::new(
                map.clone().upcast::<Interactive>(),
                self.pointer.clone(),
                &root,
                Point::new(at.0, at.1),
                self.stamp(),
                PointerPointProperties::new(self.buttons, kind),
                self.modifiers,
                1,
            ));
        }

        fn to(&self, map: &Ref<MapControl>, at: (f64, f64)) {
            let root = root_of(map);
            map.raise_event(&PointerEventArgs::new(
                Some(InputElement::pointer_moved_event()),
                map.clone().upcast::<Interactive>(),
                self.pointer.clone(),
                Some(&*root),
                Point::new(at.0, at.1),
                self.stamp(),
                PointerPointProperties::new(self.buttons, PointerUpdateKind::Other),
                self.modifiers,
            ));
        }

        fn up(&self, map: &Ref<MapControl>, at: (f64, f64)) {
            let kind = match self.button {
                MouseButton::Right => PointerUpdateKind::RightButtonReleased,
                _ => PointerUpdateKind::LeftButtonReleased,
            };
            let root = root_of(map);
            map.raise_event(&PointerReleasedEventArgs::new(
                map.clone().upcast::<Interactive>(),
                self.pointer.clone(),
                &root,
                Point::new(at.0, at.1),
                self.stamp(),
                PointerPointProperties::new(RawInputModifiers::NONE, kind),
                self.modifiers,
                self.button,
            ));
        }
    }

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} is not {b}");
    }

    fn finger_with_barrel(map: &Ref<MapControl>, from: (f64, f64), to: (f64, f64)) {
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Pen, true);
        let held = RawInputModifiers::LEFT_MOUSE_BUTTON | RawInputModifiers::PEN_BARREL_BUTTON;
        let root = root_of(map);
        map.raise_event(&PointerPressedEventArgs::new(
            map.clone().upcast::<Interactive>(),
            pointer.clone(),
            &root,
            Point::new(from.0, from.1),
            1000,
            PointerPointProperties::new(held, PointerUpdateKind::LeftButtonPressed),
            KeyModifiers::NONE,
            1,
        ));
        map.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            map.clone().upcast::<Interactive>(),
            pointer.clone(),
            Some(&*root),
            Point::new(to.0, to.1),
            1016,
            PointerPointProperties::new(held, PointerUpdateKind::Other),
            KeyModifiers::NONE,
        ));
        map.raise_event(&PointerReleasedEventArgs::new(
            map.clone().upcast::<Interactive>(),
            pointer,
            &root,
            Point::new(to.0, to.1),
            1032,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        ));
    }

    fn press_key(map: &MapControl, key: Key, modifiers: KeyModifiers) -> bool {
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.key_modifiers = modifiers;
        <MapControl as ferroui_base::input::InputElementImpl>::on_key_down(map, &args);
        args.handled()
    }

    /// Runs what a smooth motion would do over a few seconds of frames.
    fn settle(map: &MapControl) {
        map.settle_for_test();
    }

    fn clicks(layer: &Rc<MapElementsLayer>) -> Rc<RefCell<Vec<MapElementClickEventArgs>>> {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let store = seen.clone();
        layer
            .map_element_click()
            .add(Rc::new(move |args: &MapElementClickEventArgs| {
                store.borrow_mut().push(args.clone());
            }));
        seen
    }

    fn peer_of(map: &Ref<MapControl>) -> Ref<AutomationPeer> {
        ControlAutomationPeer::create_peer_for_element(map)
    }

    /// A map with an icon, a titled line and an area, all in view.
    fn map_with_elements() -> (Placed, Rc<MapElementsLayer>) {
        let map = map();
        let layer = MapElementsLayer::new();
        let place = |latitude, longitude| BasicGeoposition {
            latitude,
            longitude,
        };
        layer.add(MapElement::Icon(MapIcon::new(place(2.0, 3.0))));
        let mut line = MapPolyline::new(vec![place(-2.0, -8.0), place(-2.0, 8.0)]);
        line.title = Some("Ferry".to_owned());
        layer.add(MapElement::Polyline(line));
        layer.add(MapElement::Polygon(MapPolygon::new(vec![
            place(5.0, -6.0),
            place(5.0, -2.0),
            place(8.0, -2.0),
        ])));
        map.add_layer(layer.clone());
        (map, layer)
    }

    #[test]
    fn no_peer_exists_until_a_client_asks() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let before = ELEMENT_PEERS_MADE.with(Cell::get);
        let (map, layer) = map_with_elements();
        layer.add(MapElement::Icon(MapIcon::new(BasicGeoposition::default())));
        layer.set_is_visible(false);
        layer.set_is_visible(true);
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert!(press_key(&map, Key::Enter, KeyModifiers::NONE));
        map.set_heading(45.0);
        drawn(&map);
        assert!(
            ControlAutomationPeer::from_element(&map).is_none(),
            "no peer of the map"
        );
        assert_eq!(
            ELEMENT_PEERS_MADE.with(Cell::get),
            before,
            "no peer of an element"
        );
        peer_of(&map).get_children();
        assert_eq!(ELEMENT_PEERS_MADE.with(Cell::get), before + 4);
    }

    #[test]
    fn the_elements_are_children_with_a_name_a_type_and_bounds() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, _layer) = map_with_elements();
        let peer = peer_of(&map);
        let children = peer.get_children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].get_name(), "Map icon at 2.000, 3.000");
        assert_eq!(children[1].get_name(), "Ferry");
        assert_eq!(children[0].get_class_name(), "MapIcon");
        assert_eq!(children[2].get_class_name(), "MapPolygon");
        assert_eq!(
            children[0].get_automation_control_type(),
            AutomationControlType::Button
        );
        assert_eq!(children[0].get_parent(), Some(peer.clone()));
        let at = map.view().place_to_view(BasicGeoposition {
            latitude: 2.0,
            longitude: 3.0,
        });
        let bounds = children[0].get_bounding_rectangle();
        near(bounds.x + bounds.width / 2.0, at.0, 1e-6);
        near(bounds.y + bounds.height / 2.0, at.1, 1e-6);
        assert!(!children[0].is_offscreen());
    }

    #[test]
    fn the_bounds_follow_the_heading_and_an_element_out_of_view_is_offscreen() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, _layer) = map_with_elements();
        let peer = peer_of(&map);
        let icon = peer.get_children()[0].clone();
        let north_up = icon.get_bounding_rectangle();
        map.set_heading(90.0);
        let turned = icon.get_bounding_rectangle();
        let at = map.view().place_to_view(BasicGeoposition {
            latitude: 2.0,
            longitude: 3.0,
        });
        near(turned.x + turned.width / 2.0, at.0, 1e-6);
        near(turned.y + turned.height / 2.0, at.1, 1e-6);
        assert_ne!(north_up, turned);
        map.set_center(BasicGeoposition {
            latitude: 50.0,
            longitude: 120.0,
        });
        assert!(icon.is_offscreen());
        assert!(icon.get_bounding_rectangle().is_empty());
    }

    #[test]
    fn invoke_raises_the_click_of_the_element() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, layer) = map_with_elements();
        let seen = clicks(&layer);
        let children = peer_of(&map).get_children();
        let invoke = children[1]
            .get_provider::<dyn IInvokeProvider>()
            .expect("Invoke");
        invoke.invoke().unwrap();
        let seen_now = seen.borrow();
        assert_eq!(seen_now.len(), 1);
        assert!(
            matches!(&seen_now[0].element, MapElement::Polyline(line) if line.title.as_deref() == Some("Ferry"))
        );
        drop(seen_now);
        map.set_is_enabled(false);
        assert!(invoke.invoke().is_err(), "a disabled map refuses");
        assert_eq!(seen.borrow().len(), 1);
    }

    #[test]
    fn the_children_follow_the_layers_and_a_kept_element_keeps_its_peer() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, layer) = map_with_elements();
        let peer = peer_of(&map);
        let first = peer.get_children()[0].clone();
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        peer.children_changed(move || count.set(count.get() + 1))
            .detach();
        layer.add(MapElement::Icon(MapIcon::new(BasicGeoposition::default())));
        assert_eq!(changes.get(), 1);
        let children = peer.get_children();
        assert_eq!(children.len(), 4);
        assert_eq!(children[0], first);
        layer.set_is_visible(false);
        assert!(peer.get_children().is_empty());
        layer.set_is_visible(true);
        assert!(map.remove_layer(&layer));
        assert!(peer.get_children().is_empty());
        assert!(!first.is_keyboard_focusable() || first.get_name() == "Map icon at 2.000, 3.000");
    }

    #[test]
    fn tab_walks_the_elements_and_leaves_the_map_at_the_end() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, layer) = map_with_elements();
        let at = |map: &Placed| map.focused_element().map(|(_, index)| index);
        assert_eq!(at(&map), None);
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert_eq!(at(&map), Some(0));
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert_eq!(at(&map), Some(2));
        assert!(press_key(&map, Key::Tab, KeyModifiers::SHIFT));
        assert_eq!(at(&map), Some(1));
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert!(
            !press_key(&map, Key::Tab, KeyModifiers::NONE),
            "past the last, Tab leaves the map"
        );
        assert_eq!(at(&map), None);
        assert!(
            !press_key(&map, Key::Tab, KeyModifiers::SHIFT),
            "from the map, Shift+Tab leaves it"
        );
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert!(
            press_key(&map, Key::Tab, KeyModifiers::SHIFT),
            "back past the first, on the map"
        );
        assert_eq!(at(&map), None);
        assert!(Rc::strong_count(&layer) >= 1);
    }

    #[test]
    fn enter_and_space_activate_and_escape_gives_the_focus_back() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, layer) = map_with_elements();
        let seen = clicks(&layer);
        assert!(
            !press_key(&map, Key::Enter, KeyModifiers::NONE),
            "no element: the key is free"
        );
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert!(press_key(&map, Key::Enter, KeyModifiers::NONE));
        assert!(press_key(&map, Key::Space, KeyModifiers::NONE));
        assert_eq!(seen.borrow().len(), 2);
        near(seen.borrow()[0].location.latitude, 2.0, 1e-6);
        near(seen.borrow()[0].location.longitude, 3.0, 1e-6);
        assert!(press_key(&map, Key::Escape, KeyModifiers::NONE));
        assert!(map.focused_element().is_none());
        assert!(!press_key(&map, Key::Escape, KeyModifiers::NONE));
    }

    #[test]
    fn focus_brings_an_element_into_view() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let far = BasicGeoposition {
            latitude: 40.0,
            longitude: 100.0,
        };
        let layer = MapElementsLayer::new();
        layer.add(MapElement::Icon(MapIcon::new(far)));
        map.add_layer(layer);
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        let (x, y) = map.view().place_to_view(far);
        assert!(
            (0.0..=WIDTH).contains(&x) && (0.0..=HEIGHT).contains(&y),
            "({x}, {y})"
        );
    }

    #[test]
    fn a_focused_element_that_goes_away_gives_the_focus_back_to_the_map() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, layer) = map_with_elements();
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        layer.clear();
        assert!(map.focused_element().is_none());
        assert!(!press_key(&map, Key::Enter, KeyModifiers::NONE));
    }

    #[test]
    fn a_press_of_a_pointer_ends_the_keyboard_focus_of_an_element() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let (map, _layer) = map_with_elements();
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        let finger = Finger::new(PointerType::Mouse, MouseButton::Left);
        finger.down(&map, (40.0, 40.0));
        finger.up(&map, (40.0, 40.0));
        assert!(map.focused_element().is_none());
    }

    #[test]
    fn the_root_of_the_tree_reports_and_follows_the_focused_element() {
        let _app = UnitTestApplication::start(TestServices::focusable_window());
        let (map, _layer) = map_with_elements();
        let root_peer = ControlAutomationPeer::create_peer_for_element(&map._window);
        let root = root_peer
            .get_provider::<dyn IRootProvider>()
            .expect("a root");
        let changes = Rc::new(Cell::new(0));
        let count = changes.clone();
        let _subscription = root.focus_changed(Rc::new(move || count.set(count.get() + 1)));
        assert!(map.focus());
        let map_peer = peer_of(&map);
        assert_eq!(root.get_focus(), Some(map_peer.clone()));
        let after_focus = changes.get();

        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        let children = map_peer.get_children();
        assert_eq!(root.get_focus(), Some(children[0].clone()));
        assert!(children[0].has_keyboard_focus());
        assert_eq!(changes.get(), after_focus + 1);

        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert_eq!(root.get_focus(), Some(children[1].clone()));
        assert!(!children[0].has_keyboard_focus());
        assert_eq!(changes.get(), after_focus + 2);

        assert!(press_key(&map, Key::Escape, KeyModifiers::NONE));
        assert_eq!(root.get_focus(), Some(map_peer));
        assert_eq!(changes.get(), after_focus + 3);
    }

    #[test]
    fn set_focus_of_a_peer_focuses_its_element() {
        let _app = UnitTestApplication::start(TestServices::focusable_window());
        let (map, _layer) = map_with_elements();
        let children = peer_of(&map).get_children();
        children[2].set_focus();
        assert_eq!(map.focused_element().map(|(_, index)| index), Some(2));
        assert!(map.is_focused());
        assert!(children[2].has_keyboard_focus());
    }

    #[test]
    fn the_focus_ring_is_drawn_only_for_a_focused_element() {
        use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
        let _app = UnitTestApplication::start(
            TestServices::styled_window()
                .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
        );
        let (map, _layer) = map_with_elements();
        assert!(map.focus());
        let ellipses = |log: &[String]| {
            log.iter()
                .filter(|entry| entry.starts_with("DrawEllipse"))
                .count()
        };
        let without = ellipses(&drawn(&map));
        assert!(press_key(&map, Key::Tab, KeyModifiers::NONE));
        assert_eq!(
            ellipses(&drawn(&map)),
            without + 2,
            "two rings around the icon"
        );
        assert!(press_key(&map, Key::Escape, KeyModifiers::NONE));
        assert_eq!(ellipses(&drawn(&map)), without);
    }

    #[test]
    fn the_content_transform_is_the_math_of_the_view() {
        let center = BasicGeoposition {
            latitude: 12.0,
            longitude: 34.0,
        };
        for heading in [0.0, 30.0, 90.0, 211.0] {
            let view = MapView::new(center, 5.0, heading, WIDTH, HEIGHT);
            let matrix = content_transform(&view);
            for (x, y) in [(0.0, 0.0), (400.0, 0.0), (17.0, 250.0)] {
                let drawn = matrix.transform(Point::new(x, y));
                let (expected_x, expected_y) = view.frame_to_view(x, y);
                near(drawn.x, expected_x, 1e-6);
                near(drawn.y, expected_y, 1e-6);
            }
        }
    }

    /// Draws the map on a context that records what it is asked.
    fn drawn(map: &MapControl) -> Vec<String> {
        use ferroui_base::media::{DrawingContext, PlatformDrawingContext};
        use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
        let log = DrawingLog::new();
        let mut platform = MockDrawingContextImpl::new(log.clone());
        {
            let mut core = PlatformDrawingContext::borrowed(&mut platform);
            let mut context = DrawingContext::new(&mut core);
            <MapControl as ferroui_base::VisualImpl>::render(map, &mut context);
        }
        log.take()
    }

    /// A map on the offline grid whose tiles are blank bitmaps, with one of each element.
    fn drawing_map() -> Placed {
        use ferroui_base::media::imaging::Bitmap;
        use ferroui_base::platform::{AlphaFormat, PixelFormat};
        use ferroui_base::{PixelSize, Vector};
        let map = MapControl::with_decoder(|_| {
            let pixels = vec![0u8; 256 * 256 * 4];
            let bitmap = Bitmap::from_pixels(
                PixelFormat::RGBA8888,
                AlphaFormat::Premul,
                &pixels,
                PixelSize::new(256, 256),
                Vector::new(96.0, 96.0),
                256 * 4,
            );
            Some((bitmap, 256 * 256 * 4))
        });
        let placed = place_in_window(map);
        let source = RasterTileSource::new("tile://grid/{z}/{x}/{y}", "").unwrap();
        placed.set_tile_source(Some(Rc::new(source)), Some(Rc::new(GridTileLoader)));
        let layer = MapElementsLayer::new();
        let spot = BasicGeoposition {
            latitude: 3.0,
            longitude: 4.0,
        };
        layer.add(MapElement::Polygon(MapPolygon::new(vec![
            spot,
            BasicGeoposition {
                latitude: 3.0,
                longitude: 8.0,
            },
            BasicGeoposition {
                latitude: 6.0,
                longitude: 8.0,
            },
        ])));
        layer.add(MapElement::Polyline(MapPolyline::new(vec![
            spot,
            BasicGeoposition {
                latitude: -3.0,
                longitude: -4.0,
            },
        ])));
        layer.add(MapElement::Icon(MapIcon::new(spot)));
        placed.add_layer(layer);
        // The first pass asks for the tiles, the jobs load them, the second pass draws them.
        drawn(&placed);
        for _ in 0..8 {
            run_jobs();
            drawn(&placed);
        }
        run_jobs();
        placed
    }

    #[test]
    fn a_north_up_map_draws_its_tiles_without_a_transform() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = drawing_map();
        let log = drawn(&map);
        let tiles = map.view().tiles(19).len();
        assert!(tiles > 0);
        assert_eq!(
            log.iter()
                .filter(|entry| entry.starts_with("DrawBitmap"))
                .count(),
            tiles
        );
        assert!(
            !log.iter().any(|entry| entry.starts_with("SetTransform")),
            "{log:?}"
        );
    }

    #[test]
    fn a_turned_map_turns_its_tiles_and_keeps_its_elements_in_view_coordinates() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = drawing_map();
        map.set_heading(30.0);
        for _ in 0..8 {
            drawn(&map);
            run_jobs();
        }
        let log = drawn(&map);
        let view = map.view();
        let position = |prefix: &str| log.iter().position(|entry| entry.starts_with(prefix));
        let last_bitmap = log
            .iter()
            .rposition(|entry| entry.starts_with("DrawBitmap"))
            .unwrap();
        let first_bitmap = position("DrawBitmap").unwrap();
        let first_element = position("DrawGeometry").unwrap();
        assert_eq!(
            log.iter()
                .filter(|entry| entry.starts_with("DrawBitmap"))
                .count(),
            view.tiles(19).len()
        );
        assert_eq!(
            log[first_bitmap - 1],
            format!("SetTransform {}", content_transform(&view))
        );
        assert_eq!(
            log[last_bitmap + 1],
            format!("SetTransform {}", ferroui_base::Matrix::IDENTITY)
        );
        assert!(
            first_element > last_bitmap,
            "the elements are drawn over the tiles, unturned"
        );
    }

    #[test]
    fn the_turn_of_a_pointer_is_clockwise_and_ignores_the_middle() {
        let center = Point::new(200.0, 150.0);
        near(
            pointer_turn(center, Point::new(300.0, 150.0), Point::new(200.0, 250.0)),
            90.0,
            1e-9,
        );
        near(
            pointer_turn(center, Point::new(200.0, 250.0), Point::new(300.0, 150.0)),
            -90.0,
            1e-9,
        );
        assert_eq!(
            pointer_turn(center, Point::new(202.0, 151.0), Point::new(300.0, 150.0)),
            0.0
        );
    }

    #[test]
    fn the_heading_property_is_kept_in_range_and_keeps_the_center() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let center = BasicGeoposition {
            latitude: 20.0,
            longitude: 30.0,
        };
        map.set_center(center);
        map.set_heading(-90.0);
        assert_eq!(map.heading(), 270.0);
        near(map.center().latitude, 20.0, 1e-9);
        near(map.center().longitude, 30.0, 1e-9);
        assert_eq!(map.view().heading, 270.0);
    }

    #[test]
    fn a_right_drag_turns_the_map_with_the_pointer() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let center = map.center();
        let finger = Finger::new(PointerType::Mouse, MouseButton::Right);
        finger.down(&map, (300.0, 150.0));
        finger.to(&map, (200.0, 250.0));
        finger.up(&map, (200.0, 250.0));
        // A quarter turn clockwise of the content: the heading goes back by 90 degrees.
        near(map.heading(), 270.0, 1e-6);
        assert_eq!(
            map.center(),
            center,
            "a turn about the middle keeps the center"
        );
    }

    #[test]
    fn control_and_drag_turns_the_map_for_a_pen_or_a_trackpad() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let mut finger = Finger::new(PointerType::Pen, MouseButton::Left);
        finger.modifiers = KeyModifiers::CONTROL;
        finger.down(&map, (300.0, 150.0));
        finger.to(&map, (200.0, 50.0));
        finger.up(&map, (200.0, 50.0));
        near(map.heading(), 90.0, 1e-6);
    }

    #[test]
    fn the_barrel_button_of_a_pen_turns_the_map() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let center = map.center();
        finger_with_barrel(&map, (300.0, 150.0), (200.0, 50.0));
        near(map.heading(), 90.0, 1e-6);
        assert_eq!(
            map.center(),
            center,
            "the barrel button turns and does not drag"
        );
    }

    #[test]
    fn the_barrel_button_does_nothing_when_rotation_is_disabled() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        map.set_rotate_interaction_mode(MapInteractionMode::Disabled);
        let center = map.center();
        finger_with_barrel(&map, (300.0, 150.0), (200.0, 50.0));
        assert_eq!(map.heading(), 0.0);
        assert_eq!(map.center(), center);
    }

    #[test]
    fn a_disabled_rotation_ignores_the_right_button_and_leaves_it_free() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        map.set_rotate_interaction_mode(MapInteractionMode::Disabled);
        let finger = Finger::new(PointerType::Mouse, MouseButton::Right);
        finger.down(&map, (300.0, 150.0));
        finger.to(&map, (200.0, 250.0));
        finger.up(&map, (200.0, 250.0));
        assert_eq!(map.heading(), 0.0);
        assert!(!press_key(&map, Key::Right, KeyModifiers::CONTROL));
        assert!(!press_key(&map, Key::Home, KeyModifiers::NONE));
    }

    #[test]
    fn a_drag_on_a_turned_map_keeps_the_place_under_the_pointer() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        map.set_heading(40.0);
        let held = map.view().view_to_place(150.0, 100.0);
        let finger = Finger::new(PointerType::Touch, MouseButton::Left);
        finger.down(&map, (150.0, 100.0));
        finger.to(&map, (190.0, 130.0));
        let (x, y) = map.view().place_to_view(held);
        near(x, 190.0, 1e-6);
        near(y, 130.0, 1e-6);
        finger.up(&map, (190.0, 130.0));
    }

    #[test]
    fn two_fingers_turn_the_map_about_their_middle_across_the_wrap_of_the_bearing() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let origin = Point::new(120.0, 90.0);
        let held = map.view().view_to_place(origin.x, origin.y);
        let pinch =
            |angle: f64| map.raise_event(&PinchEventArgs::with_angle(1.0, origin, angle, 0.0));
        pinch(350.0);
        assert_eq!(map.heading(), 0.0, "the first event only sets the bearing");
        pinch(10.0);
        near(map.heading(), 340.0, 1e-6);
        let (x, y) = map.view().place_to_view(held);
        near(x, origin.x, 1e-6);
        near(y, origin.y, 1e-6);
        map.raise_event(&PinchEndedEventArgs::new());
        pinch(100.0);
        near(map.heading(), 340.0, 1e-6);
    }

    #[test]
    fn a_pinch_zooms_and_turns_only_what_is_enabled() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        map.set_rotate_interaction_mode(MapInteractionMode::Disabled);
        let origin = Point::new(200.0, 150.0);
        map.raise_event(&PinchEventArgs::with_angle(1.0, origin, 10.0, 0.0));
        map.raise_event(&PinchEventArgs::with_angle(2.0, origin, 80.0, 0.0));
        near(map.zoom_level(), 5.0, 1e-9);
        assert_eq!(map.heading(), 0.0);
    }

    #[test]
    fn the_keys_turn_the_map_and_home_goes_back_to_north() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let center = map.center();
        assert!(press_key(&map, Key::Right, KeyModifiers::CONTROL));
        assert!(press_key(&map, Key::Right, KeyModifiers::CONTROL));
        settle(&map);
        near(map.heading(), 30.0, 1e-6);
        assert!(press_key(&map, Key::Left, KeyModifiers::CONTROL));
        settle(&map);
        near(map.heading(), 15.0, 1e-6);
        assert_eq!(
            map.center(),
            center,
            "control and an arrow turns and does not move the map"
        );
        assert!(press_key(&map, Key::Home, KeyModifiers::NONE));
        settle(&map);
        near(map.heading(), 0.0, 1e-6);
    }

    #[test]
    fn a_click_reports_the_element_and_the_place_at_any_heading() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        for heading in [0.0, 65.0, 180.0] {
            let map = map();
            map.set_heading(heading);
            let place = BasicGeoposition {
                latitude: 8.0,
                longitude: 12.0,
            };
            let layer = MapElementsLayer::new();
            layer.add(MapElement::Icon(MapIcon::new(place)));
            map.add_layer(layer.clone());
            let seen = clicks(&layer);
            let at = map.view().place_to_view(place);
            let finger = Finger::new(PointerType::Mouse, MouseButton::Left);
            finger.down(&map, at);
            finger.up(&map, at);
            let seen = seen.borrow();
            assert_eq!(seen.len(), 1, "heading {heading}");
            assert_eq!(seen[0].element, MapElement::Icon(MapIcon::new(place)));
            near(seen[0].location.latitude, 8.0, 1e-6);
            near(seen[0].location.longitude, 12.0, 1e-6);
        }
    }

    #[test]
    fn a_small_wobble_is_still_a_click_and_a_drag_is_not() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let place = map.center();
        let layer = MapElementsLayer::new();
        layer.add(MapElement::Icon(MapIcon::new(place)));
        map.add_layer(layer.clone());
        let seen = clicks(&layer);
        let finger = Finger::new(PointerType::Touch, MouseButton::Left);
        finger.down(&map, (200.0, 150.0));
        finger.to(&map, (202.0, 151.0));
        assert_eq!(map.center(), place, "within the slop nothing moves");
        finger.up(&map, (202.0, 151.0));
        assert_eq!(seen.borrow().len(), 1);

        finger.down(&map, (200.0, 150.0));
        finger.to(&map, (240.0, 150.0));
        finger.up(&map, (240.0, 150.0));
        assert_eq!(seen.borrow().len(), 1, "a drag is not a click");
    }

    #[test]
    fn a_click_works_with_panning_disabled_and_misses_empty_map() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        map.set_pan_interaction_mode(MapInteractionMode::Disabled);
        let place = map.center();
        let layer = MapElementsLayer::new();
        layer.add(MapElement::Icon(MapIcon::new(place)));
        map.add_layer(layer.clone());
        let seen = clicks(&layer);
        let finger = Finger::new(PointerType::Mouse, MouseButton::Left);
        finger.down(&map, (40.0, 40.0));
        finger.up(&map, (40.0, 40.0));
        assert!(seen.borrow().is_empty());
        finger.down(&map, (200.0, 150.0));
        finger.to(&map, (260.0, 150.0));
        assert_eq!(map.center(), place, "the map does not move");
        finger.up(&map, (260.0, 150.0));
        assert!(seen.borrow().is_empty(), "moved away: no click");
        finger.down(&map, (200.0, 150.0));
        finger.up(&map, (200.0, 150.0));
        assert_eq!(seen.borrow().len(), 1);
    }

    #[test]
    fn a_second_finger_hands_the_drag_to_the_pinch() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let map = map();
        let layer = MapElementsLayer::new();
        layer.add(MapElement::Icon(MapIcon::new(
            map.view().view_to_place(100.0, 100.0),
        )));
        map.add_layer(layer.clone());
        let seen = clicks(&layer);
        let first = Finger::new(PointerType::Touch, MouseButton::Left);
        let second = Finger::new(PointerType::Touch, MouseButton::Left);
        first.down(&map, (100.0, 100.0));
        second.down(&map, (300.0, 200.0));
        let center = map.center();
        second.to(&map, (340.0, 200.0));
        first.to(&map, (140.0, 100.0));
        assert_eq!(
            map.center(),
            center,
            "two fingers are a pinch and do not drag"
        );
        first.up(&map, (100.0, 100.0));
        assert!(
            seen.borrow().is_empty(),
            "the end of a pinch is not a click"
        );
    }
}
