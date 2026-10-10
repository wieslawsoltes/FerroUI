//! The server against a peer: the application on one connection, and on
//! the other a double of the registry (`org.a11y.atspi.Socket` at the
//! root path, `org.a11y.atspi.Registry`) and a client that walks the
//! tree as an assistive technology does.
//!
//! Not from the reference, which has no tests of this project. The
//! set-up is the one of the other services of the crate
//! (`test_support.rs`): a socket pair, or a private session bus with
//! `FERROUI_FREEDESKTOP_TEST_BUS=session`. The connection of the
//! application has no object server, as the one to the accessibility bus.

use super::at_spi_constants::*;
use super::at_spi_server::AtSpiServer;
use super::at_spi_state::AtSpiState;
use crate::test_support::{expect_calls, log, peer_call, pump_until, show, Log, TestConnections};
use ferroui_base::layout::ILayoutManager;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::{AutomationPeer, ControlAutomationPeer};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, CheckBox, Control, StackPanel, TextBox, Window};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

const REGISTRY_UNIQUE_NAME: &str = ":1.9";

/// The events the double of the registry says are listened to.
type Listeners = Arc<Mutex<Vec<(String, String)>>>;

struct SocketDouble {
    calls: Log,
}

#[zbus::interface(name = "org.a11y.atspi.Socket")]
impl SocketDouble {
    fn embed(&self, plug: (String, OwnedObjectPath)) -> (String, OwnedObjectPath) {
        log(&self.calls, format!("Embed {}", plug.1.as_str()));
        (REGISTRY_UNIQUE_NAME.to_string(), OwnedObjectPath::try_from(ROOT_PATH).unwrap())
    }
}

struct RegistryDouble {
    listeners: Listeners,
}

#[zbus::interface(name = "org.a11y.atspi.Registry")]
impl RegistryDouble {
    fn get_registered_events(&self) -> Vec<(String, String)> {
        self.listeners.lock().unwrap().clone()
    }
}

struct Fixture {
    connections: TestConnections,
    calls: Log,
    signals: Log,
    server: Rc<AtSpiServer>,
}

impl Fixture {
    /// The application started on its connection, with the registry
    /// saying that `listeners` are listened to.
    fn new(listeners: &[&str]) -> Fixture {
        let calls: Log = Log::default();
        let registered: Listeners = Arc::new(Mutex::new(
            listeners.iter().map(|event| (REGISTRY_UNIQUE_NAME.to_string(), event.to_string())).collect(),
        ));
        let (socket_calls, registry_listeners) = (calls.clone(), registered);
        let connections = TestConnections::new_with(ROOT_PATH, false, move |builder| {
            builder
                .serve_at(ROOT_PATH, SocketDouble { calls: socket_calls })?
                .serve_at(REGISTRY_PATH, RegistryDouble { listeners: registry_listeners })
        });
        connections.start(BUS_NAME_REGISTRY);

        // What the application emits, as the other side receives it.
        let signals: Log = Log::default();
        let (collector, service) = (signals.clone(), connections.service.clone());
        let own_name = connections.client.unique_name().map(|name| name.to_string());
        std::thread::spawn(move || {
            let iterator = match &own_name {
                Some(sender) => zbus::blocking::MessageIterator::for_match_rule(
                    zbus::MatchRule::builder()
                        .msg_type(zbus::message::Type::Signal)
                        .sender(sender.as_str())
                        .unwrap()
                        .build(),
                    &service,
                    None,
                )
                .unwrap(),
                None => zbus::blocking::MessageIterator::from(&service),
            };
            for message in iterator.flatten() {
                let header = message.header();
                let interface = header.interface().map(|name| name.to_string()).unwrap_or_default();
                if message.message_type() != zbus::message::Type::Signal || !interface.starts_with("org.a11y.atspi.Event")
                {
                    continue;
                }
                let Ok((detail, detail1, _detail2, value, _properties)) =
                    message.body().deserialize::<(String, i32, i32, OwnedValue, HashMap<String, OwnedValue>)>()
                else {
                    continue;
                };
                let member = header.member().map(|name| name.to_string()).unwrap_or_default();
                let path = header.path().map(|path| path.to_string()).unwrap_or_default();
                log(&collector, format!("{path} {member} {detail} {detail1} {}", show(&value)));
            }
        });

        let server = AtSpiServer::new();
        server.start_on_connection(connections.client.clone());
        let fixture = Fixture { connections, calls, signals, server };
        // The tracker asked the registry what is listened to.
        fixture.connections.settle();
        fixture
    }

    fn call<B>(&self, path: &str, interface: &str, method: &str, body: B) -> zbus::Result<zbus::Message>
    where
        B: zbus::export::serde::ser::Serialize + zbus::zvariant::DynamicType + Send + 'static,
    {
        peer_call(&self.connections, path, interface, method, body)
    }

    fn accessible<T>(&self, path: &str, method: &str) -> T
    where
        T: for<'a> zbus::export::serde::Deserialize<'a> + zbus::zvariant::Type,
    {
        self.call(path, IFACE_ACCESSIBLE, method, ()).unwrap().body().deserialize::<T>().unwrap()
    }

    fn property(&self, path: &str, interface: &str, name: &str) -> String {
        crate::test_support::peer_property(&self.connections, path, interface, name).unwrap_or_else(|| "<none>".into())
    }

    fn error(&self, path: &str, interface: &str, method: &str) -> String {
        match self.call(path, interface, method, ()) {
            Err(zbus::Error::MethodError(name, _, _)) => name.to_string(),
            other => format!("{other:?}"),
        }
    }

    fn children(&self, path: &str) -> Vec<String> {
        self.accessible::<Vec<(String, OwnedObjectPath)>>(path, "GetChildren")
            .into_iter()
            .map(|(_, path)| path.to_string())
            .collect()
    }

    fn has_state(&self, path: &str, state: AtSpiState) -> bool {
        let words = self.accessible::<Vec<u32>>(path, "GetState");
        let bit = state as u32;
        words[(bit / 32) as usize] & (1 << (bit % 32)) != 0
    }

    /// The first object below `path` (in tree order) with the role name
    /// `role`.
    fn find(&self, path: &str, role: &str) -> Option<String> {
        for child in self.children(path) {
            if self.accessible::<String>(&child, "GetRoleName") == role {
                return Some(child);
            }
            if let Some(found) = self.find(&child, role) {
                return Some(found);
            }
        }
        None
    }

    /// Runs the dispatcher until the signals the other side received are
    /// `entries`, and forgets them.
    fn expect_signals(&self, entries: &[&str]) {
        expect_calls(&self.connections, &self.signals, entries);
    }

    fn add_window(&self, window: &Ref<Window>) -> Ref<AutomationPeer> {
        let peer = ControlAutomationPeer::create_peer_for_element(window);
        let peer = peer.get_automation_root().unwrap_or(peer);
        self.server.add_window(&peer);
        expect_calls(&self.connections, &self.calls, &[&format!("Embed {ROOT_PATH}")]);
        peer
    }
}

struct Ui {
    window: Ref<Window>,
    text_box: Ref<TextBox>,
    check_box: Ref<CheckBox>,
    clicks: Rc<Cell<u32>>,
}

fn ui() -> Ui {
    let button = Button::new();
    button.set_content(Some(Rc::new("Save".to_string())));
    let clicks = Rc::new(Cell::new(0));
    let counter = clicks.clone();
    button.click(move |_, _| counter.set(counter.get() + 1));

    let text_box = TextBox::new();
    text_box.set_text(Some("Hello world"));

    let check_box = CheckBox::new();
    check_box.set_content(Some(Rc::new("Agree".to_string())));

    let panel = StackPanel::new();
    panel.children().add(&button);
    panel.children().add(&text_box);
    panel.children().add(&check_box);

    let window = Window::new();
    window.set_title(Some("Accessible window".to_string()));
    window.set_content(Some(Control::boxed(&panel)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    Ui { window, text_box, check_box, clicks }
}

#[test]
fn dbus_the_root_object_answers_before_any_window() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(&[]);

    assert_eq!(fixture.accessible::<u32>(ROOT_PATH, "GetRole"), 75);
    assert_eq!(fixture.accessible::<String>(ROOT_PATH, "GetRoleName"), "application");
    assert_eq!(fixture.accessible::<i32>(ROOT_PATH, "GetIndexInParent"), -1);
    assert_eq!(fixture.children(ROOT_PATH), Vec::<String>::new());
    assert_eq!(fixture.accessible::<Vec<u32>>(ROOT_PATH, "GetState"), vec![1 << 1, 0]);
    assert_eq!(
        fixture.accessible::<Vec<String>>(ROOT_PATH, "GetInterfaces"),
        vec![IFACE_ACCESSIBLE.to_string(), IFACE_APPLICATION.to_string()]
    );
    assert_eq!(fixture.property(ROOT_PATH, IFACE_ACCESSIBLE, "ChildCount"), "0");
    assert_eq!(fixture.property(ROOT_PATH, IFACE_ACCESSIBLE, "version"), "1");
    assert_eq!(fixture.property(ROOT_PATH, IFACE_ACCESSIBLE, "Parent"), format!("(,{NULL_PATH})"));
    assert_eq!(fixture.property(ROOT_PATH, IFACE_APPLICATION, "ToolkitName"), "FerroUI");
    assert_eq!(fixture.property(ROOT_PATH, IFACE_APPLICATION, "AtspiVersion"), "2.1");
    assert_eq!(fixture.property(ROOT_PATH, IFACE_APPLICATION, "Id"), "0");

    // The registry sets the identifier of the application.
    let set = (IFACE_APPLICATION.to_string(), "Id".to_string(), zbus::zvariant::Value::from(7i32));
    fixture.call(ROOT_PATH, "org.freedesktop.DBus.Properties", "Set", set).unwrap();
    assert_eq!(fixture.property(ROOT_PATH, IFACE_APPLICATION, "Id"), "7");

    // The cache has no items: clients ask the objects.
    let items = fixture.call(CACHE_PATH, IFACE_CACHE, "GetItems", ()).unwrap();
    assert_eq!(items.body().signature().to_string(), "a((so)(so)(so)iiassusau)");

    // What is not there.
    assert_eq!(fixture.error("/org/ferroui/a11y/99", IFACE_ACCESSIBLE, "GetRole"), "org.freedesktop.DBus.Error.UnknownObject");
    assert_eq!(fixture.error(ROOT_PATH, IFACE_TEXT, "GetNSelections"), "org.freedesktop.DBus.Error.UnknownInterface");
    assert_eq!(fixture.error(ROOT_PATH, IFACE_ACCESSIBLE, "NoSuchMember"), "org.freedesktop.DBus.Error.UnknownMethod");
    assert_eq!(fixture.property(ROOT_PATH, IFACE_ACCESSIBLE, "NoSuchProperty"), "<none>");

    // Introspection: the interfaces of the root, and the paths below a
    // path that has no object.
    let xml: String = fixture
        .call(ROOT_PATH, "org.freedesktop.DBus.Introspectable", "Introspect", ())
        .unwrap()
        .body()
        .deserialize()
        .unwrap();
    for interface in [IFACE_ACCESSIBLE, IFACE_APPLICATION, IFACE_EVENT_OBJECT, "org.freedesktop.DBus.Properties"] {
        assert!(xml.contains(&format!("<interface name=\"{interface}\">")), "{interface} in {xml}");
    }
    assert!(xml.contains("<method name=\"GetChildAtIndex\">"));
    let xml: String = fixture
        .call("/org/a11y/atspi", "org.freedesktop.DBus.Introspectable", "Introspect", ())
        .unwrap()
        .body()
        .deserialize()
        .unwrap();
    assert!(xml.contains("<node name=\"accessible\"/>") && xml.contains("<node name=\"cache\"/>"), "{xml}");

    fixture.server.dispose();
}

#[test]
fn dbus_the_tree_of_a_window_is_walked() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(&[]);
    let ui = ui();
    let window_peer = fixture.add_window(&ui.window);

    // Adding the window again changes nothing.
    fixture.server.add_window(&window_peer);
    let windows = fixture.children(ROOT_PATH);
    assert_eq!(windows, vec!["/org/ferroui/a11y/1".to_string()]);
    let window = windows[0].as_str();

    assert_eq!(fixture.accessible::<String>(window, "GetRoleName"), "frame");
    assert_eq!(fixture.accessible::<u32>(window, "GetRole"), 23);
    assert_eq!(fixture.property(window, IFACE_ACCESSIBLE, "Name"), "Accessible window");
    assert_eq!(fixture.accessible::<i32>(window, "GetIndexInParent"), 0);
    assert!(fixture.property(window, IFACE_ACCESSIBLE, "Parent").ends_with(&format!(",{ROOT_PATH})")));
    assert!(fixture.has_state(window, AtSpiState::Active));
    assert!(fixture.accessible::<Vec<String>>(window, "GetInterfaces").contains(&IFACE_APPLICATION.to_string()));
    let layer: u32 = fixture.call(window, IFACE_COMPONENT, "GetLayer", ()).unwrap().body().deserialize().unwrap();
    assert_eq!(layer, WINDOW_LAYER);
    let application = fixture.accessible::<(String, OwnedObjectPath)>(window, "GetApplication");
    assert_eq!(application.1.as_str(), ROOT_PATH);

    let button = fixture.find(window, "push button").expect("a button in the tree");
    assert_eq!(fixture.property(&button, IFACE_ACCESSIBLE, "Name"), "Save");
    assert!(fixture.has_state(&button, AtSpiState::Enabled));
    assert!(fixture.has_state(&button, AtSpiState::Sensitive));
    assert!(fixture.has_state(&button, AtSpiState::Focusable));
    assert!(!fixture.has_state(&button, AtSpiState::Checkable));
    let interfaces = fixture.accessible::<Vec<String>>(&button, "GetInterfaces");
    assert_eq!(interfaces, vec![IFACE_ACCESSIBLE.to_string(), IFACE_ACTION.to_string(), IFACE_COMPONENT.to_string()]);
    let attributes = fixture.accessible::<HashMap<String, String>>(&button, "GetAttributes");
    assert_eq!(attributes.get("toolkit").map(String::as_str), Some("FerroUI"));
    assert_eq!(attributes.get("explicit-name").map(String::as_str), Some("true"));

    let entry = fixture.find(window, "entry").expect("a text box in the tree");
    assert!(fixture.has_state(&entry, AtSpiState::Editable));
    assert!(fixture.has_state(&entry, AtSpiState::SingleLine));
    assert!(fixture.accessible::<Vec<String>>(&entry, "GetInterfaces").contains(&IFACE_EDITABLE_TEXT.to_string()));

    let check_box = fixture.find(window, "check box").expect("a check box in the tree");
    assert_eq!(fixture.property(&check_box, IFACE_ACCESSIBLE, "Name"), "Agree");
    assert!(fixture.has_state(&check_box, AtSpiState::Checkable));
    assert!(!fixture.has_state(&check_box, AtSpiState::Checked));
    ui.check_box.set_is_checked(Some(true));
    assert!(fixture.has_state(&check_box, AtSpiState::Checked));

    // A child knows its parent and its place in it.
    let parent = fixture.property(&button, IFACE_ACCESSIBLE, "Parent");
    let parent_path = parent.trim_end_matches(')').rsplit(',').next().unwrap().to_string();
    let index = fixture.accessible::<i32>(&button, "GetIndexInParent");
    assert_eq!(fixture.children(&parent_path)[usize::try_from(index).unwrap()], button);
    let by_index = fixture.call(&parent_path, IFACE_ACCESSIBLE, "GetChildAtIndex", index).unwrap();
    assert_eq!(by_index.body().deserialize::<(String, OwnedObjectPath)>().unwrap().1.as_str(), button);
    let beyond = fixture.call(&parent_path, IFACE_ACCESSIBLE, "GetChildAtIndex", 99i32).unwrap();
    assert_eq!(beyond.body().deserialize::<(String, OwnedObjectPath)>().unwrap().1.as_str(), NULL_PATH);

    // The paths of the nodes show below the prefix of the port.
    let xml: String = fixture
        .call(APP_PATH_PREFIX, "org.freedesktop.DBus.Introspectable", "Introspect", ())
        .unwrap()
        .body()
        .deserialize()
        .unwrap();
    assert!(xml.contains("<node name=\"1\"/>"), "{xml}");

    // The window is removed: nothing of it answers any more.
    fixture.server.remove_window(&window_peer);
    assert_eq!(fixture.children(ROOT_PATH), Vec::<String>::new());
    assert_eq!(fixture.error(&button, IFACE_ACCESSIBLE, "GetRole"), "org.freedesktop.DBus.Error.UnknownObject");
    assert_eq!(fixture.error(window, IFACE_ACCESSIBLE, "GetRole"), "org.freedesktop.DBus.Error.UnknownObject");
    // No listener was registered, so nothing was emitted.
    fixture.expect_signals(&[]);
    fixture.server.dispose();
}

#[test]
fn dbus_actions_text_and_components() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(&[]);
    let ui = ui();
    fixture.add_window(&ui.window);
    let window = fixture.children(ROOT_PATH)[0].clone();

    // The action of the button.
    let button = fixture.find(&window, "push button").unwrap();
    assert_eq!(fixture.property(&button, IFACE_ACTION, "NActions"), "1");
    let name: String = fixture.call(&button, IFACE_ACTION, "GetName", 0i32).unwrap().body().deserialize().unwrap();
    assert_eq!(name, "click");
    let actions: Vec<(String, String, String)> =
        fixture.call(&button, IFACE_ACTION, "GetActions", ()).unwrap().body().deserialize().unwrap();
    assert_eq!(actions, vec![("Click".to_string(), "Performs the default action".to_string(), String::new())]);
    let done: bool = fixture.call(&button, IFACE_ACTION, "DoAction", 0i32).unwrap().body().deserialize().unwrap();
    assert!(done);
    pump_until(|| ui.clicks.get() == 1);
    let done: bool = fixture.call(&button, IFACE_ACTION, "DoAction", 5i32).unwrap().body().deserialize().unwrap();
    assert!(!done);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(ui.clicks.get(), 1);
    assert!(fixture.call(&button, IFACE_ACTION, "DoAction", "zero").is_err());

    // The check box is toggled through its action.
    let check_box = fixture.find(&window, "check box").unwrap();
    let name: String = fixture.call(&check_box, IFACE_ACTION, "GetName", 0i32).unwrap().body().deserialize().unwrap();
    assert_eq!(name, "toggle");
    fixture.call(&check_box, IFACE_ACTION, "DoAction", 0i32).unwrap();
    assert_eq!(ui.check_box.is_checked(), Some(true));
    assert!(fixture.has_state(&check_box, AtSpiState::Checked));

    // The text of the text box, read and written.
    let entry = fixture.find(&window, "entry").unwrap();
    assert_eq!(fixture.property(&entry, IFACE_TEXT, "CharacterCount"), "11");
    assert_eq!(fixture.property(&entry, IFACE_TEXT, "CaretOffset"), "0");
    let text: String = fixture.call(&entry, IFACE_TEXT, "GetText", (0i32, -1i32)).unwrap().body().deserialize().unwrap();
    assert_eq!(text, "Hello world");
    let at: (String, i32, i32) =
        fixture.call(&entry, IFACE_TEXT, "GetStringAtOffset", (7i32, 1u32)).unwrap().body().deserialize().unwrap();
    assert_eq!(at, ("world".to_string(), 6, 11));
    let unit: i32 = fixture.call(&entry, IFACE_TEXT, "GetCharacterAtOffset", 4i32).unwrap().body().deserialize().unwrap();
    assert_eq!(unit, 0x6f);
    let inserted: bool =
        fixture.call(&entry, IFACE_EDITABLE_TEXT, "InsertText", (5i32, ",".to_string(), 1i32)).unwrap().body().deserialize().unwrap();
    assert!(inserted);
    assert_eq!(ui.text_box.text().as_deref(), Some("Hello, world"));
    let deleted: bool =
        fixture.call(&entry, IFACE_EDITABLE_TEXT, "DeleteText", (0i32, 7i32)).unwrap().body().deserialize().unwrap();
    assert!(deleted);
    assert_eq!(ui.text_box.text().as_deref(), Some("world"));
    let set: bool =
        fixture.call(&entry, IFACE_EDITABLE_TEXT, "SetTextContents", "Bye".to_string()).unwrap().body().deserialize().unwrap();
    assert!(set);
    assert_eq!(ui.text_box.text().as_deref(), Some("Bye"));
    let text: String = fixture.call(&entry, IFACE_TEXT, "GetText", (0i32, -1i32)).unwrap().body().deserialize().unwrap();
    assert_eq!(text, "Bye");

    // The component: extents inside the window, and the focus.
    let window_extents: (i32, i32, i32, i32) =
        fixture.call(&window, IFACE_COMPONENT, "GetExtents", 0u32).unwrap().body().deserialize().unwrap();
    let extents: (i32, i32, i32, i32) =
        fixture.call(&button, IFACE_COMPONENT, "GetExtents", 1u32).unwrap().body().deserialize().unwrap();
    assert!(extents.2 > 0 && extents.3 > 0, "{extents:?}");
    assert!(extents.0 >= 0 && extents.0 + extents.2 <= window_extents.2, "{extents:?} in {window_extents:?}");
    let size: (i32, i32) = fixture.call(&button, IFACE_COMPONENT, "GetSize", ()).unwrap().body().deserialize().unwrap();
    assert_eq!(size, (extents.2, extents.3));
    let layer: u32 = fixture.call(&button, IFACE_COMPONENT, "GetLayer", ()).unwrap().body().deserialize().unwrap();
    assert_eq!(layer, WIDGET_LAYER);
    let focused: bool = fixture.call(&button, IFACE_COMPONENT, "GrabFocus", ()).unwrap().body().deserialize().unwrap();
    // The mock platform of the tests has no focus; the smoke run of the
    // X11 example sees the focus move.
    assert!(focused);

    fixture.server.dispose();
}

#[test]
fn dbus_events_follow_the_listeners_of_the_registry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    // Nobody listens: the tracker read an empty list.
    let fixture = Fixture::new(&[]);
    let ui = ui();
    let window_peer = fixture.add_window(&ui.window);
    let window = fixture.children(ROOT_PATH)[0].clone();
    let check_box = fixture.find(&window, "check box").unwrap();

    ui.check_box.set_is_checked(Some(true));
    fixture.expect_signals(&[]);

    // A listener registers for the events of objects.
    let registered = (REGISTRY_UNIQUE_NAME, "object:state-changed", Vec::<String>::new());
    fixture.connections.emit(REGISTRY_PATH, BUS_NAME_REGISTRY, "EventListenerRegistered", &registered);
    fixture.connections.settle();
    ui.check_box.set_is_checked(Some(false));
    fixture.expect_signals(&[
        &format!("{check_box} StateChanged checked 0 0"),
        &format!("{check_box} StateChanged indeterminate 0 0"),
    ]);
    ui.check_box.set_is_checked(Some(true));
    fixture.expect_signals(&[
        &format!("{check_box} StateChanged checked 1 0"),
        &format!("{check_box} StateChanged indeterminate 0 0"),
    ]);

    // Removing the window: the root loses a child, and every node of the
    // window is defunct, children before their parents.
    fixture.server.remove_window(&window_peer);
    fixture.connections.settle();
    let signals = std::mem::take(&mut *fixture.signals.lock().unwrap());
    assert!(signals[0].starts_with(&format!("{ROOT_PATH} ChildrenChanged remove 0 (")), "{signals:?}");
    assert!(signals[0].ends_with(&format!("{window})")), "{signals:?}");
    assert!(signals[1..].iter().all(|signal| signal.ends_with(" StateChanged defunct 1 0")), "{signals:?}");
    assert_eq!(signals.last().unwrap(), &format!("{window} StateChanged defunct 1 0"));
    assert!(signals.contains(&format!("{check_box} StateChanged defunct 1 0")));

    // The listener is gone: silence again.
    let window_peer = fixture.add_window_again(&ui.window);
    let deregistered = (REGISTRY_UNIQUE_NAME, "object:state-changed");
    fixture.connections.emit(REGISTRY_PATH, BUS_NAME_REGISTRY, "EventListenerDeregistered", &deregistered);
    fixture.connections.settle();
    std::mem::take(&mut *fixture.signals.lock().unwrap());
    ui.check_box.set_is_checked(Some(false));
    fixture.server.remove_window(&window_peer);
    fixture.expect_signals(&[]);
    fixture.server.dispose();
}

#[test]
fn dbus_a_listener_that_was_registered_before_the_start_is_heard() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(&["window:", "mouse:abs"]);
    let ui = ui();
    fixture.add_window(&ui.window);
    fixture.connections.settle();
    let window = fixture.children(ROOT_PATH)[0].clone();
    // The embedding announced the window as a child of the root.
    let signals = std::mem::take(&mut *fixture.signals.lock().unwrap());
    assert_eq!(signals.len(), 1, "{signals:?}");
    assert!(signals[0].starts_with(&format!("{ROOT_PATH} ChildrenChanged add 0 (")), "{signals:?}");
    assert!(signals[0].ends_with(&format!("{window})")), "{signals:?}");

    // A node exists once a client asked for it; from then on its changes are told.
    let entry = fixture.find(&window, "entry").unwrap();
    ui.text_box.set_text(Some("changed"));
    fixture.connections.settle();
    let signals = std::mem::take(&mut *fixture.signals.lock().unwrap());
    assert_eq!(signals, vec![format!("{entry} PropertyChange accessible-value 0 changed")]);
    fixture.server.dispose();
}

#[test]
fn dbus_children_that_arrive_after_a_client_asked_are_found() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let fixture = Fixture::new(&[]);

    // A window that is shown and asked for its children before it has content.
    // The window is added and walked before it is shown, when it has no template yet.
    let window = Window::new();
    fixture.add_window(&window);
    let path = fixture.children(ROOT_PATH)[0].clone();
    let before = fixture.children(&path);
    assert_eq!(fixture.find(&path, "push button"), None);
    window.show();
    assert_eq!(fixture.find(&path, "push button"), None);

    let button = Button::new();
    button.set_content(Some(Rc::new("Late".to_string())));
    window.set_content(Some(Control::boxed(&button)));
    window.layout_manager().execute_initial_layout_pass();

    let found = fixture.find(&path, "push button");
    assert!(found.is_some(), "the children before: {before:?}, after: {:?}", fixture.children(&path));
    assert_eq!(fixture.property(&found.unwrap(), IFACE_ACCESSIBLE, "Name"), "Late");
    fixture.server.dispose();
}

impl Fixture {
    /// Adds a window after the application was embedded: no second embed.
    fn add_window_again(&self, window: &Ref<Window>) -> Ref<AutomationPeer> {
        let peer = ControlAutomationPeer::create_peer_for_element(window);
        let peer = peer.get_automation_root().unwrap_or(peer);
        self.server.add_window(&peer);
        self.connections.settle();
        assert_eq!(self.calls.lock().unwrap().len(), 0);
        peer
    }
}
