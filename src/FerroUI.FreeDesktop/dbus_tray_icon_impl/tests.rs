// Not from the reference, which has no tests of this class. The tray icon
// runs here on one connection; a second one serves a double of the
// watcher of status notifier items and asks the item what a tray host
// asks.

use super::*;
use crate::test_support::{log, on_session_bus, peer_call, peer_property, pump_until, scope, Log, TestConnections};
use ferroui_controls::{NativeMenu, NativeMenuItem};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zbus::zvariant::OwnedValue;

const INTERFACE: &str = "org.kde.StatusNotifierItem";

struct WatcherDouble {
    log: Log,
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl WatcherDouble {
    fn register_status_notifier_item(&self, service: &str) {
        log(&self.log, service);
    }
}

struct TestIcon;

impl IWindowIconImpl for TestIcon {
    fn save(&self, _output_stream: &mut dyn std::io::Write) -> std::io::Result<()> {
        Ok(())
    }
}

struct Fixture {
    connections: TestConnections,
    log: Log,
}

impl Fixture {
    fn new() -> Fixture {
        let log: Log = Arc::new(Mutex::new(Vec::new()));
        let double = WatcherDouble { log: log.clone() };
        let connections = TestConnections::new(WATCHER_PATH, move |builder| builder.serve_at(WATCHER_PATH, double));
        Fixture { connections, log }
    }

    /// The watcher is on the bus before a tray icon is made. Without a bus
    /// the signal of the double of the bus is on its way to the other
    /// connection: it is let through first, because a subscription that is
    /// made while it travels would take it for a change of the owner (a
    /// bus sends a signal only to those who had asked for it).
    fn start_watcher(&self) {
        self.connections.start(WATCHER_NAME);
        self.connections.settle();
    }

    fn tray_icon(&self) -> Rc<DBusTrayIconImpl> {
        let tray_icon = DBusTrayIconImpl::with_connection(Some(self.connections.client.clone()));
        // Two pixels: opaque, then half transparent.
        tray_icon.set_icon_converter_delegate(Some(Rc::new(|icon| match icon {
            Some(_) => vec![2, 1, 0xFF11_2233, 0x8044_5566],
            None => Vec::new(),
        })));
        tray_icon
    }

    /// Waits until the watcher was told about `count` items, and gives
    /// their names.
    fn registered(&self, count: usize) -> Vec<String> {
        pump_until(|| self.log.lock().unwrap().len() >= count);
        self.connections.settle();
        let names = self.log.lock().unwrap().clone();
        assert_eq!(names.len(), count, "{names:?}");
        names
    }

    fn property(&self, name: &str) -> Option<String> {
        peer_property(&self.connections, STATUS_NOTIFIER_ITEM_PATH, INTERFACE, name)
    }

    /// Whether the connection of the tray icon owns `name`. Without a bus
    /// a connection keeps its names itself, and nobody can be asked.
    fn owns(&self, name: &str) -> Option<bool> {
        if !on_session_bus() {
            return None;
        }
        let dbus = zbus::blocking::fdo::DBusProxy::new(&self.connections.service).unwrap();
        Some(dbus.name_has_owner(zbus::names::BusName::try_from(name).unwrap()).unwrap())
    }
}

fn expected_name_prefix() -> String {
    format!("org.kde.StatusNotifierItem-{}-", std::process::id())
}

#[test]
fn dbus_an_item_is_registered_with_the_watcher_and_answers_a_host() {
    let _scope = scope();
    let fixture = Fixture::new();
    fixture.start_watcher();
    let tray_icon = fixture.tray_icon();
    assert!(tray_icon.is_active());

    let names = fixture.registered(1);
    assert!(names[0].starts_with(&expected_name_prefix()), "{names:?}");
    assert_ne!(fixture.owns(&names[0]), Some(false));

    // What a host reads.
    assert_eq!(fixture.property("Status").as_deref(), Some("Active"));
    assert_eq!(fixture.property("Category").as_deref(), Some(""));
    assert_eq!(fixture.property("WindowId").as_deref(), Some("0"));
    assert_eq!(fixture.property("ItemIsMenu").as_deref(), Some("false"));
    assert_eq!(fixture.property("IconPixmap").as_deref(), Some("[(0,0,[])]"));
    assert_eq!(fixture.property("OverlayIconPixmap").as_deref(), Some("[]"));
    assert_eq!(fixture.property("ToolTip").as_deref(), Some("(,[],,)"));
    let menu_path = fixture.property("Menu").unwrap();
    assert!(menu_path.starts_with("/org/ferroui/dbusmenu/"), "{menu_path}");

    tray_icon.set_tool_tip_text(Some("Tip"));
    assert_eq!(fixture.property("Title").as_deref(), Some("Tip"));
    assert_eq!(fixture.property("Id").as_deref(), Some("Tip"));
    assert_eq!(fixture.property("Category").as_deref(), Some("ApplicationStatus"));
    // No text leaves the title as it is.
    tray_icon.set_tool_tip_text(None);
    assert_eq!(fixture.property("Title").as_deref(), Some("Tip"));

    // The pixels are sent as bytes: alpha, red, green, blue.
    let icon: Rc<dyn IWindowIconImpl> = Rc::new(TestIcon);
    tray_icon.set_icon(Some(icon));
    assert_eq!(fixture.property("IconPixmap").as_deref(), Some("[(2,1,[255,17,34,51,128,68,85,102])]"));
    tray_icon.set_icon(None);
    assert_eq!(fixture.property("IconPixmap").as_deref(), Some("[(1,1,[255,0,0,0])]"));

    // Activation is the click of the tray icon; the other calls do nothing.
    let clicks = Rc::new(Cell::new(0));
    {
        let clicks = clicks.clone();
        tray_icon.set_on_clicked(Some(Rc::new(move || clicks.set(clicks.get() + 1))));
    }
    assert!(tray_icon.on_clicked().is_some());
    peer_call(&fixture.connections, STATUS_NOTIFIER_ITEM_PATH, INTERFACE, "Activate", (3i32, 4i32)).unwrap();
    assert_eq!(clicks.get(), 1);
    peer_call(&fixture.connections, STATUS_NOTIFIER_ITEM_PATH, INTERFACE, "SecondaryActivate", (3i32, 4i32)).unwrap();
    peer_call(&fixture.connections, STATUS_NOTIFIER_ITEM_PATH, INTERFACE, "ContextMenu", (3i32, 4i32)).unwrap();
    peer_call(&fixture.connections, STATUS_NOTIFIER_ITEM_PATH, INTERFACE, "Scroll", (1i32, "vertical".to_string())).unwrap();
    assert_eq!(clicks.get(), 1);

    // The menu of the icon is exported at the path the item names.
    let menu = NativeMenu::new();
    menu.add(NativeMenuItem::with_header("Quit"));
    tray_icon.menu_exporter().unwrap().set_native_menu(Some(menu));
    let body = (0i32, -1i32, vec!["label".to_string()]);
    let reply = peer_call(&fixture.connections, &menu_path, "com.canonical.dbusmenu", "GetLayout", body).unwrap();
    let (_, (_, _, children)): (u32, (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>)) = reply.body().deserialize().unwrap();
    assert_eq!(children.iter().map(|child| crate::test_support::show(child)).collect::<Vec<_>>(), ["(1,{label=Quit},[])"]);

    tray_icon.dispose();
    fixture.connections.settle();
    assert!(!tray_icon.is_active());
    assert_eq!(fixture.property("Status"), None);
    assert_ne!(fixture.owns(&names[0]), Some(true));
    let body = (0i32, -1i32, Vec::<String>::new());
    assert!(peer_call(&fixture.connections, &menu_path, "com.canonical.dbusmenu", "GetLayout", body).is_err());
    // Nothing happens after the end.
    tray_icon.set_tool_tip_text(Some("Late"));
    tray_icon.set_is_visible(false);
    tray_icon.set_is_visible(true);
    fixture.connections.settle();
    assert_eq!(fixture.registered(1), names);
}

#[test]
fn dbus_an_item_waits_for_a_watcher_and_follows_it() {
    let _scope = scope();
    let fixture = Fixture::new();
    let tray_icon = fixture.tray_icon();
    tray_icon.set_tool_tip_text(Some("Early"));
    fixture.connections.settle();

    // No watcher: nothing is registered and the object is not exported.
    assert!(fixture.registered(0).is_empty());
    assert_eq!(fixture.property("Status"), None);

    // A watcher comes: the item is registered, with what was set before.
    fixture.connections.start(WATCHER_NAME);
    let names = fixture.registered(1);
    assert!(names[0].starts_with(&expected_name_prefix()), "{names:?}");
    assert_eq!(fixture.property("Title").as_deref(), Some("Early"));

    // The watcher goes: the object is gone and the name is kept.
    fixture.connections.stop(WATCHER_NAME);
    pump_until(|| !tray_icon.service_connected.get());
    fixture.connections.settle();
    assert_eq!(fixture.property("Status"), None);
    assert_ne!(fixture.owns(&names[0]), Some(false));

    // Another watcher: registered again, under the same name.
    fixture.connections.start(WATCHER_NAME);
    let again = fixture.registered(2);
    assert_eq!(again[1], names[0]);
    assert_eq!(fixture.property("Status").as_deref(), Some("Active"));

    tray_icon.dispose();
}

#[test]
fn dbus_a_hidden_item_is_not_exported_and_comes_back_under_its_name() {
    let _scope = scope();
    let fixture = Fixture::new();
    fixture.start_watcher();
    let tray_icon = fixture.tray_icon();
    let names = fixture.registered(1);

    tray_icon.set_is_visible(false);
    // The same value again changes nothing.
    tray_icon.set_is_visible(false);
    pump_until(|| tray_icon.sys_tray_service_name_request.borrow().is_none());
    fixture.connections.settle();
    assert_eq!(fixture.property("Status"), None);
    assert_ne!(fixture.owns(&names[0]), Some(true));

    tray_icon.set_is_visible(true);
    let again = fixture.registered(2);
    assert_eq!(again[1], names[0]);
    assert_eq!(fixture.property("Status").as_deref(), Some("Active"));
    assert_ne!(fixture.owns(&names[0]), Some(false));

    // Hidden and shown again before the dispatcher ran: registered once more.
    tray_icon.set_is_visible(false);
    tray_icon.set_is_visible(true);
    let third = fixture.registered(3);
    assert_eq!(third[2], names[0]);
    assert_eq!(fixture.property("Status").as_deref(), Some("Active"));

    tray_icon.dispose();
}

#[test]
fn two_items_have_two_names() {
    let _scope = scope();
    let fixture = Fixture::new();
    fixture.start_watcher();
    let first = fixture.tray_icon();
    let first_names = fixture.registered(1);
    first.dispose();
    fixture.connections.settle();
    let second = fixture.tray_icon();
    let names = fixture.registered(2);
    assert_ne!(names[1], first_names[0]);
    assert!(names[1].starts_with(&expected_name_prefix()));
    second.dispose();
}

#[test]
fn without_a_bus_the_tray_icon_is_not_active() {
    let _scope = scope();
    let tray_icon = DBusTrayIconImpl::with_connection(None);
    assert!(!tray_icon.is_active());
    assert!(tray_icon.menu_exporter().is_none());
    tray_icon.set_icon_converter_delegate(Some(Rc::new(|_| vec![1, 1, 0])));
    let icon: Rc<dyn IWindowIconImpl> = Rc::new(TestIcon);
    tray_icon.set_icon(Some(icon));
    tray_icon.set_icon(None);
    tray_icon.set_tool_tip_text(Some("Tip"));
    tray_icon.set_is_visible(false);
    tray_icon.set_is_visible(true);
    tray_icon.dispose();
}

#[test]
fn the_pixmap_of_icon_data() {
    assert_eq!(DBusTrayIconImpl::empty_pixmap(), (1, 1, vec![255, 0, 0, 0]));
    assert_eq!(pixmap_from_icon_data(&[]), None);
    assert_eq!(pixmap_from_icon_data(&[1]), None);
    assert_eq!(pixmap_from_icon_data(&[0, 0]), Some((0, 0, vec![])));
    assert_eq!(
        pixmap_from_icon_data(&[1, 2, 0x0102_0304, 0xFFFE_FDFC]),
        Some((1, 2, vec![1, 2, 3, 4, 0xFF, 0xFE, 0xFD, 0xFC]))
    );
    // Pixels the data does not have are transparent.
    assert_eq!(pixmap_from_icon_data(&[2, 1, 0xFF00_0000]), Some((2, 1, vec![255, 0, 0, 0, 0, 0, 0, 0])));
}
