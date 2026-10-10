// Not from the reference, which has no tests of this class. The exporter
// runs here on one connection, and a peer on a second one asks it what a
// menu host asks: the layout, properties, events. The second connection
// also serves a double of the registrar of application menus.

use super::*;
use crate::test_support::{expect_calls, log, peer_call, peer_property, pump_until, scope, show, Log, TestConnections};
use ferroui_base::input::{Key, KeyGesture};
use std::sync::{Arc, Mutex};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const INTERFACE: &str = "com.canonical.dbusmenu";

struct RegistrarDouble {
    log: Log,
    refuse: bool,
}

#[zbus::interface(name = "com.canonical.AppMenu.Registrar")]
impl RegistrarDouble {
    fn register_window(&self, window_id: u32, menu_object_path: OwnedObjectPath) -> zbus::fdo::Result<()> {
        let known = menu_object_path.as_str().starts_with("/org/ferroui/dbusmenu/");
        log(&self.log, format!("RegisterWindow({window_id}, generated path: {known})"));
        if self.refuse {
            return Err(zbus::fdo::Error::Failed("refused".to_string()));
        }
        Ok(())
    }

    fn unregister_window(&self, window_id: u32) {
        log(&self.log, format!("UnregisterWindow({window_id})"));
    }
}

struct Fixture {
    connections: TestConnections,
    log: Log,
}

impl Fixture {
    fn new(refuse: bool) -> Fixture {
        let log: Log = Arc::new(Mutex::new(Vec::new()));
        let double = RegistrarDouble { log: log.clone(), refuse };
        let connections = TestConnections::new(REGISTRAR_PATH, move |builder| builder.serve_at(REGISTRAR_PATH, double));
        connections.start(REGISTRAR_NAME);
        Fixture { connections, log }
    }

    fn detached(&self) -> Rc<DBusMenuExporterImpl> {
        let path = DBusMenuExporter::generate_dbus_menu_obj_path();
        let exporter = DBusMenuExporter::try_create_detached_native_menu(&path, self.connections.client.clone());
        // The object is exported by a task of the dispatcher.
        pump_until(|| exporter.handle.borrow().is_some());
        self.connections.settle();
        exporter
    }

    /// `GetLayout` as a peer calls it: the revision and the layout as text.
    fn layout(&self, exporter: &DBusMenuExporterImpl, parent: i32, depth: i32, names: &[&str]) -> Option<(u32, String)> {
        let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        let reply = peer_call(&self.connections, exporter.path(), INTERFACE, "GetLayout", (parent, depth, names)).ok()?;
        let (revision, (id, properties, children)): (u32, (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>)) =
            reply.body().deserialize().unwrap();
        Some((revision, show_layout(id, &properties, &children)))
    }

    fn event(&self, exporter: &DBusMenuExporterImpl, id: i32, event_id: &str) {
        let body = (id, event_id.to_string(), Value::I32(0), 0u32);
        peer_call(&self.connections, exporter.path(), INTERFACE, "Event", body).unwrap();
    }
}

fn show_properties(properties: &HashMap<String, OwnedValue>) -> String {
    let mut entries: Vec<String> = properties.iter().map(|(key, value)| format!("{key}={}", show(value))).collect();
    entries.sort();
    format!("{{{}}}", entries.join(","))
}

fn show_layout(id: i32, properties: &HashMap<String, OwnedValue>, children: &[OwnedValue]) -> String {
    let children: Vec<String> = children.iter().map(|child| show(child)).collect();
    format!("({id},{},[{}])", show_properties(properties), children.join(","))
}

/// File (Open with a gesture, a separator, a checked box, a radio item, a
/// disabled item, a hidden item), and Empty, which opens a menu without
/// items.
fn sample_menu() -> (Ref<NativeMenu>, Ref<NativeMenuItem>) {
    let menu = NativeMenu::new();
    let file = NativeMenuItem::with_header("File");
    let file_menu = NativeMenu::new();
    let open = NativeMenuItem::with_header("Open");
    open.set_gesture(Some(KeyGesture::new(Key::O, KeyModifiers::CONTROL | KeyModifiers::SHIFT)));
    file_menu.add(open.clone());
    file_menu.add(NativeMenuItemSeparator::new());
    let check = NativeMenuItem::with_header("Check");
    check.set_toggle_type(MenuItemToggleType::CheckBox);
    check.set_is_checked(true);
    file_menu.add(check);
    let radio = NativeMenuItem::with_header("Radio");
    radio.set_toggle_type(MenuItemToggleType::Radio);
    file_menu.add(radio);
    let disabled = NativeMenuItem::with_header("Disabled");
    disabled.set_is_enabled(false);
    file_menu.add(disabled);
    let hidden = NativeMenuItem::new();
    hidden.set_is_visible(false);
    file_menu.add(hidden);
    file.set_menu(Some(file_menu));
    menu.add(file);
    let empty = NativeMenuItem::with_header("Empty");
    empty.set_menu(Some(NativeMenu::new()));
    menu.add(empty);
    (menu, open)
}

const SAMPLE_LAYOUT: &str = "(0,{},[\
(1,{children-display=submenu,label=File,visible=true},[\
(2,{label=Open,shortcut=[[Control,Shift,O]],visible=true},[]),\
(3,{type=separator},[]),\
(4,{label=Check,toggle-state=1,toggle-type=checkmark,visible=true},[]),\
(5,{label=Radio,toggle-state=0,toggle-type=radio,visible=true},[]),\
(6,{enabled=false,label=Disabled,visible=true},[]),\
(7,{label=<null>,visible=false},[])]),\
(8,{children-display=submenu,enabled=false,label=Empty,visible=true},[])])";

#[test]
fn dbus_the_layout_of_a_menu_is_what_a_host_is_told() {
    let _scope = scope();
    let fixture = Fixture::new(false);
    let exporter = fixture.detached();
    let exported_changes = Rc::new(Cell::new(0));
    let subscription = {
        let exported_changes = exported_changes.clone();
        exporter.on_is_native_menu_exported_changed(Rc::new(move || exported_changes.set(exported_changes.get() + 1)))
    };

    // The properties of the object.
    assert_eq!(peer_property(&fixture.connections, exporter.path(), INTERFACE, "Version").as_deref(), Some("4"));
    assert_eq!(peer_property(&fixture.connections, exporter.path(), INTERFACE, "TextDirection").as_deref(), Some("ltr"));
    assert_eq!(peer_property(&fixture.connections, exporter.path(), INTERFACE, "Status").as_deref(), Some("normal"));
    assert_eq!(peer_property(&fixture.connections, exporter.path(), INTERFACE, "IconThemePath").as_deref(), Some("[]"));

    // Without a menu: the root alone. The revision is 2: every exporter
    // starts with one reset.
    assert!(!exporter.is_native_menu_exported());
    assert_eq!(fixture.layout(&exporter, 0, -1, &[]), Some((2, "(0,{},[])".to_string())));
    assert!(exporter.is_native_menu_exported());
    assert_eq!(exported_changes.get(), 1);

    let (menu, _open) = sample_menu();
    exporter.set_native_menu(Some(menu));
    assert_eq!(fixture.layout(&exporter, 0, -1, &[]), Some((3, SAMPLE_LAYOUT.to_string())));
    assert_eq!(exported_changes.get(), 1);

    // One level, and the properties that were asked for. The items keep
    // their identifiers.
    assert_eq!(
        fixture.layout(&exporter, 0, 1, &["label"]),
        Some((3, "(0,{},[(1,{label=File},[]),(8,{label=Empty},[])])".to_string()))
    );
    // A sub menu by the identifier of its item.
    let (_, file) = fixture.layout(&exporter, 1, 1, &["type"]).unwrap();
    assert_eq!(file, "(1,{},[(2,{},[]),(3,{type=separator},[]),(4,{},[]),(5,{},[]),(6,{},[]),(7,{},[])])");
    // Depth 0: no children; an identifier nobody has: an item without anything.
    assert_eq!(fixture.layout(&exporter, 1, 0, &["label"]).unwrap().1, "(1,{label=File},[])");
    assert_eq!(fixture.layout(&exporter, 99, -1, &[]).unwrap().1, "(0,{},[])");

    // GetGroupProperties and GetProperty.
    let body = (vec![2i32, 3, 99], vec!["label".to_string(), "type".to_string()]);
    let reply = peer_call(&fixture.connections, exporter.path(), INTERFACE, "GetGroupProperties", body).unwrap();
    let groups: Vec<(i32, HashMap<String, OwnedValue>)> = reply.body().deserialize().unwrap();
    let groups: Vec<String> = groups.iter().map(|(id, properties)| format!("{id}{}", show_properties(properties))).collect();
    assert_eq!(groups, ["2{label=Open}", "3{type=separator}", "99{}"]);

    let property = |id: i32, name: &str| {
        let reply = peer_call(&fixture.connections, exporter.path(), INTERFACE, "GetProperty", (id, name.to_string())).unwrap();
        let value: OwnedValue = reply.body().deserialize().unwrap();
        show(&value)
    };
    assert_eq!(property(4, "toggle-type"), "checkmark");
    // A property an item does not have reads as the number 0.
    assert_eq!(property(2, "toggle-type"), "0");

    // AboutToShow never asks for an update.
    let reply = peer_call(&fixture.connections, exporter.path(), INTERFACE, "AboutToShow", (1i32,)).unwrap();
    assert!(!reply.body().deserialize::<bool>().unwrap());
    let reply = peer_call(&fixture.connections, exporter.path(), INTERFACE, "AboutToShowGroup", (vec![1i32, 2],)).unwrap();
    assert_eq!(reply.body().deserialize::<(Vec<i32>, Vec<i32>)>().unwrap(), (vec![], vec![]));

    subscription.dispose();
    exporter.dispose();
}

#[test]
fn dbus_a_click_event_raises_the_click_of_an_enabled_item() {
    let _scope = scope();
    let fixture = Fixture::new(false);
    let exporter = fixture.detached();
    let (menu, open) = sample_menu();
    exporter.set_native_menu(Some(menu));
    assert_eq!(fixture.layout(&exporter, 0, -1, &[]).unwrap().1, SAMPLE_LAYOUT);

    let clicks = Rc::new(Cell::new(0));
    let _subscription = {
        let clicks = clicks.clone();
        open.click(move |_| clicks.set(clicks.get() + 1))
    };

    fixture.event(&exporter, 2, "clicked");
    assert_eq!(clicks.get(), 1);
    // Other events, and identifiers nobody has, do nothing.
    fixture.event(&exporter, 2, "hovered");
    fixture.event(&exporter, 99, "clicked");
    assert_eq!(clicks.get(), 1);

    // A group of events.
    let events = vec![(2i32, "clicked".to_string(), Value::I32(0), 0u32), (2, "clicked".to_string(), Value::I32(0), 0)];
    let reply = peer_call(&fixture.connections, exporter.path(), INTERFACE, "EventGroup", (events,)).unwrap();
    assert!(reply.body().deserialize::<Vec<i32>>().unwrap().is_empty());
    assert_eq!(clicks.get(), 3);

    // A disabled item is not clicked. The change resets the layout, and
    // the item has another identifier afterwards.
    open.set_is_enabled(false);
    pump_until(|| exporter.revision.get() == 4);
    let (revision, layout) = fixture.layout(&exporter, 0, -1, &["label", "enabled"]).unwrap();
    assert_eq!(revision, 4);
    assert!(layout.contains("(10,{enabled=false,label=Open},[])"), "{layout}");
    fixture.event(&exporter, 10, "clicked");
    fixture.event(&exporter, 2, "clicked");
    assert_eq!(clicks.get(), 3);

    exporter.dispose();
}

#[test]
fn dbus_a_change_of_the_menu_resets_the_layout_once_and_tells_the_host() {
    let _scope = scope();
    let fixture = Fixture::new(false);
    let exporter = fixture.detached();
    let (menu, open) = sample_menu();
    exporter.set_native_menu(Some(menu.clone()));
    assert_eq!(fixture.layout(&exporter, 0, -1, &[]).unwrap().0, 3);

    // A peer listens for the signal before the changes are made.
    let (ready_sender, ready) = std::sync::mpsc::channel();
    let listener = {
        let service = fixture.connections.service.clone();
        let path = exporter.path().to_string();
        std::thread::spawn(move || {
            let rule = zbus::MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .interface(INTERFACE)
                .unwrap()
                .member("LayoutUpdated")
                .unwrap()
                .path(path)
                .unwrap()
                .build();
            let mut signals = zbus::blocking::MessageIterator::for_match_rule(rule, &service, None).unwrap();
            ready_sender.send(()).unwrap();
            let signal = signals.next().unwrap().unwrap();
            signal.body().deserialize::<(u32, i32)>().unwrap()
        })
    };
    pump_until(|| ready.try_recv().is_ok());

    // Three changes: a property of an item, the items of a sub menu, the
    // items of the menu.
    open.set_header(Some("Open...".to_string()));
    open.parent().unwrap().add(NativeMenuItem::with_header("Close"));
    menu.add(NativeMenuItem::with_header("Help"));
    assert_eq!(exporter.revision.get(), 3);
    pump_until(|| listener.is_finished());
    assert_eq!(listener.join().unwrap(), (4, 0));
    assert_eq!(exporter.revision.get(), 4);

    let (revision, layout) = fixture.layout(&exporter, 0, -1, &["label"]).unwrap();
    assert_eq!(revision, 4);
    assert!(layout.contains("label=Open...") && layout.contains("label=Close") && layout.contains("label=Help"), "{layout}");

    // An item that was removed is not listened to after the reset.
    let subscribers = open.property_changed_subscriber_count();
    exporter.set_native_menu(None);
    assert_eq!(open.property_changed_subscriber_count(), subscribers - 1);
    assert_eq!(fixture.layout(&exporter, 0, -1, &[]), Some((5, "(0,{},[])".to_string())));
    open.set_header(Some("Other".to_string()));
    fixture.connections.settle();
    assert_eq!(exporter.revision.get(), 5);

    exporter.dispose();
}

#[test]
fn dbus_the_menu_of_a_window_is_registered_and_unregistered() {
    let _scope = scope();
    let fixture = Fixture::new(false);
    let exporter = DBusMenuExporterImpl::new_top_level(fixture.connections.client.clone(), 77);
    expect_calls(&fixture.connections, &fixture.log, &["RegisterWindow(77, generated path: true)"]);
    assert!(fixture.layout(&exporter, 0, -1, &[]).is_some());

    exporter.dispose();
    // A second dispose does nothing.
    exporter.dispose();
    expect_calls(&fixture.connections, &fixture.log, &["UnregisterWindow(77)"]);
    assert!(fixture.layout(&exporter, 0, -1, &[]).is_none());
}

#[test]
fn dbus_a_registrar_that_refuses_is_not_asked_to_unregister() {
    let _scope = scope();
    let fixture = Fixture::new(true);
    let exporter = DBusMenuExporterImpl::new_top_level(fixture.connections.client.clone(), 78);
    expect_calls(&fixture.connections, &fixture.log, &["RegisterWindow(78, generated path: true)"]);
    // The menu is exported all the same.
    assert!(fixture.layout(&exporter, 0, -1, &[]).is_some());

    exporter.dispose();
    fixture.connections.settle();
    expect_calls(&fixture.connections, &fixture.log, &[]);
}

#[test]
fn dbus_a_detached_menu_is_not_registered() {
    let _scope = scope();
    let fixture = Fixture::new(false);
    let exporter = fixture.detached();
    assert!(fixture.layout(&exporter, 0, -1, &[]).is_some());
    exporter.dispose();
    fixture.connections.settle();
    expect_calls(&fixture.connections, &fixture.log, &[]);
    assert!(fixture.layout(&exporter, 0, -1, &[]).is_none());
}

#[test]
fn an_exporter_that_is_disposed_before_it_was_exported_exports_nothing() {
    let _scope = scope();
    let fixture = Fixture::new(false);
    let exporter = DBusMenuExporterImpl::new_top_level(fixture.connections.client.clone(), 79);
    exporter.dispose();
    fixture.connections.settle();
    assert!(exporter.handle.borrow().is_none());
    assert!(fixture.log.lock().unwrap().is_empty());
}

#[test]
fn a_generated_path_is_an_object_path_of_its_own() {
    let first = DBusMenuExporter::generate_dbus_menu_obj_path();
    let second = DBusMenuExporter::generate_dbus_menu_obj_path();
    assert_ne!(first, second);
    assert!(first.starts_with("/org/ferroui/dbusmenu/"));
    assert_eq!(first.len(), "/org/ferroui/dbusmenu/".len() + 32);
    assert!(ObjectPath::try_from(first.as_str()).is_ok());
}

#[test]
fn the_properties_of_items() {
    let _scope = scope();
    let item = NativeMenuItem::with_header("Item");
    let base: Ref<NativeMenuItemBase> = item.clone().upcast();
    let of = |name: &str| get_property(&(Some(base.clone()), item.menu()), name).map(|value| show(&value));

    assert_eq!(of("type"), None);
    assert_eq!(of("label").as_deref(), Some("Item"));
    assert_eq!(of("enabled"), None);
    assert_eq!(of("visible").as_deref(), Some("true"));
    assert_eq!(of("shortcut"), None);
    assert_eq!(of("toggle-type"), None);
    assert_eq!(of("toggle-state"), None);
    assert_eq!(of("icon-data"), None);
    assert_eq!(of("children-display"), None);
    assert_eq!(of("something else"), None);

    // A gesture without a modifier is no shortcut.
    item.set_gesture(Some(KeyGesture::from_key(Key::F5)));
    assert_eq!(of("shortcut"), None);
    item.set_gesture(Some(KeyGesture::new(Key::F5, KeyModifiers::ALT | KeyModifiers::META)));
    assert_eq!(of("shortcut").as_deref(), Some("[[Alt,Super,F5]]"));

    // All properties, when none is named.
    let all = get_properties(&(Some(base.clone()), None), &[]);
    let mut names: Vec<&str> = all.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["label", "shortcut", "visible"]);

    // The root has no properties.
    assert!(get_properties(&(None, Some(NativeMenu::new())), &[]).is_empty());
}
