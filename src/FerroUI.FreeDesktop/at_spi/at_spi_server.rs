//! The port of `AtSpiServer.cs`: the application on the accessibility
//! bus. It connects, registers the root object and the cache, embeds the
//! application in the registry when the first window is added, and keeps
//! the nodes of the automation peers by path and by peer.

use super::application_accessible_handler::ApplicationAccessibleHandler;
use super::application_at_spi_node::ApplicationAtSpiNode;
use super::application_node_application_handler::ApplicationNodeApplicationHandler;
use super::at_spi_cache_handler::AtSpiCacheHandler;
use super::at_spi_constants::*;
use super::at_spi_node::AtSpiNode;
use super::at_spi_registry_event_tracker::AtSpiRegistryEventTracker;
use super::dbus::connection::AtSpiConnection;
use super::dbus::interface::DBusInterface;
use super::dbus::proxies::{OrgA11yAtspiSocketProxy, OrgA11yBusProxy};
use super::dbus::types::AtSpiObjectReference;
use super::handlers::at_spi_event_object_handler::AtSpiEventObjectHandler;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Ref;
use ferroui_controls::automation::peers::AutomationPeer;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use zbus::proxy::CacheProperties;
use zbus::zvariant::Value;

pub struct AtSpiServer {
    weak_self: Weak<AtSpiServer>,
    nodes_by_path: RefCell<HashMap<String, Rc<AtSpiNode>>>,
    nodes_by_peer: RefCell<HashMap<Ref<AutomationPeer>, Rc<AtSpiNode>>>,

    next_node_id: Cell<i32>,
    a11y_connection: RefCell<Option<Rc<AtSpiConnection>>>,
    unique_name: RefCell<String>,

    app_root: RefCell<Option<Rc<ApplicationAtSpiNode>>>,
    cache_handler: RefCell<Option<Rc<AtSpiCacheHandler>>>,
    app_root_event_handler: RefCell<Option<Rc<AtSpiEventObjectHandler>>>,
    registry_tracker: RefCell<Option<Rc<AtSpiRegistryEventTracker>>>,
    app_root_registration: RefCell<Option<Rc<dyn IDisposable>>>,
    cache_registration: RefCell<Option<Rc<dyn IDisposable>>>,
    /// An embed call is in flight (`_embedTask`).
    embed_in_flight: Cell<bool>,
    is_embedded: Cell<bool>,
}

impl AtSpiServer {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            nodes_by_path: RefCell::new(HashMap::new()),
            nodes_by_peer: RefCell::new(HashMap::new()),
            next_node_id: Cell::new(0),
            a11y_connection: RefCell::new(None),
            unique_name: RefCell::new(String::new()),
            app_root: RefCell::new(None),
            cache_handler: RefCell::new(None),
            app_root_event_handler: RefCell::new(None),
            registry_tracker: RefCell::new(None),
            app_root_registration: RefCell::new(None),
            cache_registration: RefCell::new(None),
            embed_in_flight: Cell::new(false),
            is_embedded: Cell::new(false),
        })
    }

    pub(crate) fn a11y_connection(&self) -> Option<Rc<AtSpiConnection>> {
        self.a11y_connection.borrow().clone()
    }

    pub(crate) fn unique_name(&self) -> String {
        self.unique_name.borrow().clone()
    }

    pub(crate) fn has_event_listeners(&self) -> bool {
        self.registry_tracker.borrow().as_ref().is_none_or(|tracker| tracker.has_event_listeners())
    }

    /// Connects to the accessibility bus and registers the root object
    /// and the cache.
    ///
    /// # Errors
    /// The message of what failed: the address of the accessibility bus
    /// could not be resolved, or the bus could not be connected to.
    pub async fn start_async(&self) -> Result<(), String> {
        let address = Self::get_accessibility_bus_address_async().await;
        if address.trim().is_empty() {
            return Err("Failed to resolve the accessibility bus address.".to_string());
        }

        // No object server: the method calls of this connection are
        // answered by the loop of `AtSpiConnection`.
        let connection = zbus::blocking::connection::Builder::address(address.as_str())
            .and_then(|builder| builder.build())
            .map(zbus::blocking::Connection::into_inner)
            .map_err(|e| e.to_string())?;
        self.start_on_connection(connection);
        Ok(())
    }

    /// The rest of `StartAsync`, on a connection to the accessibility
    /// bus that exists (and has no object server).
    pub(crate) fn start_on_connection(&self, connection: zbus::Connection) {
        self.embed_in_flight.set(false);
        self.is_embedded.set(false);

        let connection = AtSpiConnection::new(connection);
        *self.unique_name.borrow_mut() = connection.unique_name();
        *self.a11y_connection.borrow_mut() = Some(connection.clone());

        *self.app_root.borrow_mut() = Some(Rc::new(ApplicationAtSpiNode::new(None)));
        *self.cache_handler.borrow_mut() = Some(Rc::new(AtSpiCacheHandler));

        self.build_and_register_app_root();
        self.register_cache_path();

        let tracker = AtSpiRegistryEventTracker::new(connection.connection().clone());
        *self.registry_tracker.borrow_mut() = Some(tracker.clone());
        // Registry tracking is best-effort; AT-SPI server remains functional without it.
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            tracker.initialize_async().await;
        }));
    }

    pub fn add_window(&self, window_peer: &Ref<AutomationPeer>) {
        let (Some(_), Some(app_root)) = (self.a11y_connection(), self.app_root.borrow().clone()) else { return };

        // Idempotent check
        if self.try_get_attached_node(Some(window_peer)).is_some_and(|node| node.as_root().is_some()) {
            return;
        }

        let window_node = self.get_or_create_node(window_peer);
        let Some(root) = window_node.as_root() else { return };
        root.set_app_root(Some(app_root.clone()));

        if !self.attach_node(&window_node, None) {
            return;
        }

        if !app_root.window_children().iter().any(|child| Rc::ptr_eq(child, &window_node)) {
            app_root.add_window_child(&window_node);
        }

        // GTK-like root registration behavior:
        // embed once, then only emit incremental children-changed for later windows.
        if self.is_embedded.get() {
            self.emit_window_child_added(&window_node);
        } else {
            // Embed may already be in flight. The embed completion path
            // emits children-changed for all currently tracked windows.
            self.ensure_embedded_and_announce();
        }
    }

    fn ensure_embedded_and_announce(&self) {
        // Ignore repeated embed requests while one is already in flight.
        if self.is_embedded.get() || self.embed_in_flight.replace(true) {
            return;
        }

        let Some(this) = self.weak_self.upgrade() else { return };
        drop(Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            this.embed_and_announce_once_async().await;
        }));
    }

    async fn embed_and_announce_once_async(&self) {
        if let Err(e) = self.embed_application_async().await {
            // Embed failed - screen reader won't discover us.
            // Reset so the next AddWindow retries.
            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::FREE_DESKTOP_PLATFORM) {
                logger.log(None, &format!("AT-SPI embed failed; will retry when windows are added: {e}"));
            }
            self.embed_in_flight.set(false);
            return;
        }

        self.is_embedded.set(true);
        self.embed_in_flight.set(false);

        // Now that the screen reader knows about us, emit children-changed
        // for every window that was added before the embed completed.
        let (Some(event_handler), Some(app_root)) =
            (self.app_root_event_handler.borrow().clone(), self.app_root.borrow().clone())
        else {
            return;
        };
        if !self.has_event_listeners() {
            return;
        }

        for (i, child) in app_root.window_children().iter().enumerate() {
            let child_ref = self.get_reference(Some(child));
            event_handler.emit_children_changed_signal("add", index_of(i), child_ref.to_dbus_struct());
        }
    }

    fn emit_window_child_added(&self, window_node: &Rc<AtSpiNode>) {
        let (Some(event_handler), Some(app_root)) =
            (self.app_root_event_handler.borrow().clone(), self.app_root.borrow().clone())
        else {
            return;
        };
        if !self.has_event_listeners() {
            return;
        }

        let child_ref = self.get_reference(Some(window_node));
        let count = app_root.window_children().len();
        event_handler.emit_children_changed_signal("add", index_of(count) - 1, child_ref.to_dbus_struct());
    }

    pub fn remove_window(&self, window_peer: &Ref<AutomationPeer>) {
        let (Some(_), Some(app_root)) = (self.a11y_connection(), self.app_root.borrow().clone()) else { return };

        let Some(window_node) = self.try_get_attached_node(Some(window_peer)).filter(|node| node.as_root().is_some())
        else {
            return;
        };

        // Emit children-changed("remove") on app root before removal (guarded by event listeners)
        if let (true, Some(event_handler)) = (self.has_event_listeners(), self.app_root_event_handler.borrow().clone())
        {
            let index = app_root
                .window_children()
                .iter()
                .position(|child| Rc::ptr_eq(child, &window_node))
                .map_or(-1, index_of);
            let child_ref = self.get_reference(Some(&window_node));
            event_handler.emit_children_changed_signal("remove", index, child_ref.to_dbus_struct());
        }

        self.detach_subtree_recursive(&window_node);
    }

    /// `DisposeAsync`. Nothing of it waits here: registrations are
    /// removed at once and the connection closes when its last message
    /// was sent.
    pub fn dispose(&self) {
        self.is_embedded.set(false);
        self.embed_in_flight.set(false);

        if let Some(tracker) = self.registry_tracker.borrow_mut().take() {
            tracker.dispose();
        }

        let nodes: Vec<Rc<AtSpiNode>> = self.nodes_by_path.borrow().values().cloned().collect();

        // Dispose all D-Bus registrations before the connection.
        for node in &nodes {
            node.dispose_path_registration();
        }

        if let Some(registration) = self.app_root_registration.borrow_mut().take() {
            registration.dispose();
        }
        if let Some(registration) = self.cache_registration.borrow_mut().take() {
            registration.dispose();
        }

        if let Some(connection) = self.a11y_connection.borrow_mut().take() {
            connection.dispose();
        }

        for node in &nodes {
            self.release_node(node);
        }

        self.unique_name.borrow_mut().clear();
        *self.cache_handler.borrow_mut() = None;
        *self.app_root.borrow_mut() = None;
        *self.app_root_event_handler.borrow_mut() = None;
        self.nodes_by_path.borrow_mut().clear();
        self.nodes_by_peer.borrow_mut().clear();
    }

    pub(crate) fn get_reference(&self, node: Option<&Rc<AtSpiNode>>) -> AtSpiObjectReference {
        match node {
            Some(node) if node.is_attached() && self.nodes_by_path.borrow().contains_key(node.path()) => {
                AtSpiObjectReference::new(self.unique_name(), node.path())
            }
            _ => self.get_null_reference(),
        }
    }

    pub(crate) fn get_null_reference(&self) -> AtSpiObjectReference {
        AtSpiObjectReference::new("", NULL_PATH)
    }

    pub(crate) fn get_root_reference(&self) -> AtSpiObjectReference {
        AtSpiObjectReference::new(self.unique_name(), ROOT_PATH)
    }

    pub(crate) fn emit_window_activation_change(&self, window_node: &Rc<AtSpiNode>, active: bool) {
        if self.a11y_connection().is_none() || !self.has_event_listeners() {
            return;
        }

        if let Some(event_handler) = window_node.event_object_handler() {
            event_handler.emit_state_changed_signal("active", i32::from(active), Value::from(0i32));
        }

        let Some(window_handler) = window_node.event_window_handler() else { return };

        if active {
            window_handler.emit_activate_signal();
        } else {
            window_handler.emit_deactivate_signal();
        }
    }

    pub(crate) fn emit_focus_change(&self, focused_node: Option<&Rc<AtSpiNode>>) {
        let Some(focused_node) = focused_node else { return };
        if self.a11y_connection().is_none() || !self.has_event_listeners() {
            return;
        }

        if let Some(event_handler) = focused_node.event_object_handler() {
            event_handler.emit_state_changed_signal("focused", 1, Value::from(0i32));
        }
    }

    pub(crate) fn allocate_node_path(&self) -> String {
        let id = self.next_node_id.get() + 1;
        self.next_node_id.set(id);
        node_path(id)
    }

    pub(crate) fn get_or_create_node(&self, peer: &Ref<AutomationPeer>) -> Rc<AtSpiNode> {
        if let Some(node) = self.nodes_by_peer.borrow().get(peer) {
            return node.clone();
        }

        let this = self.weak_self.upgrade().expect("the server is alive while it is called");
        let node = AtSpiNode::create(peer, &this);
        self.nodes_by_peer.borrow_mut().insert(peer.clone(), node.clone());
        node
    }

    pub(crate) fn try_get_node(&self, peer: Option<&Ref<AutomationPeer>>) -> Option<Rc<AtSpiNode>> {
        self.nodes_by_peer.borrow().get(peer?).cloned()
    }

    pub(crate) fn try_get_attached_node(&self, peer: Option<&Ref<AutomationPeer>>) -> Option<Rc<AtSpiNode>> {
        self.try_get_node(peer)
            .filter(|node| node.is_attached() && self.nodes_by_path.borrow().contains_key(node.path()))
    }

    pub(crate) fn attach_node(&self, node: &Rc<AtSpiNode>, parent: Option<&Rc<AtSpiNode>>) -> bool {
        let Some(connection) = self.a11y_connection() else { return false };

        if parent.is_some_and(|parent| !parent.is_attached()) {
            return false;
        }

        if node.is_attached() {
            let current = node.parent();
            let same = match (&current, parent) {
                (Some(current), Some(parent)) => Rc::ptr_eq(current, parent),
                (None, None) => true,
                _ => false,
            };
            if !same {
                if let Some(current) = current {
                    current.remove_attached_child(node);
                }
                node.set_parent(parent);
            }

            node.build_and_register_handlers(&connection);
            self.nodes_by_path.borrow_mut().insert(node.path().to_string(), node.clone());
            return true;
        }

        node.attach(parent);
        if !node.is_attached() {
            return false;
        }

        self.nodes_by_path.borrow_mut().insert(node.path().to_string(), node.clone());
        true
    }

    pub(crate) fn detach_subtree_recursive(&self, root_node: &Rc<AtSpiNode>) {
        let mut to_remove = Vec::new();
        Self::collect_subtree(root_node, &mut to_remove);
        self.remove_nodes(&to_remove, true);
    }

    fn collect_subtree(node: &Rc<AtSpiNode>, result: &mut Vec<Rc<AtSpiNode>>) {
        for child in node.attached_children() {
            Self::collect_subtree(&child, result);
        }

        result.push(node.clone());
    }

    fn remove_nodes(&self, nodes: &[Rc<AtSpiNode>], emit_defunct: bool) {
        for node in nodes {
            if !node.is_attached() || !self.nodes_by_path.borrow().contains_key(node.path()) {
                continue;
            }

            if let (true, true, Some(event_handler)) =
                (emit_defunct, self.has_event_listeners(), node.event_object_handler())
            {
                event_handler.emit_state_changed_signal("defunct", 1, Value::from("0"));
            }

            self.nodes_by_path.borrow_mut().remove(node.path());
            if let Some(parent) = node.parent() {
                parent.remove_attached_child(node);
            }

            if node.as_root().is_some() {
                if let Some(app_root) = self.app_root.borrow().clone() {
                    app_root.remove_window_child(node);
                }
            }

            self.release_node(node);
        }
    }

    fn build_and_register_app_root(&self) {
        let (Some(connection), Some(app_root)) = (self.a11y_connection(), self.app_root.borrow().clone()) else {
            return;
        };

        if let Some(registration) = self.app_root_registration.borrow_mut().take() {
            registration.dispose();
        }

        let accessible_handler = Rc::new(ApplicationAccessibleHandler::new(self.weak_self.clone(), app_root.clone()));
        let application_handler = Rc::new(ApplicationNodeApplicationHandler::new());
        let event_handler = Rc::new(AtSpiEventObjectHandler::new(self.weak_self.clone(), app_root.path().to_string()));
        *self.app_root_event_handler.borrow_mut() = Some(event_handler.clone());

        let targets: Vec<Rc<dyn DBusInterface>> = vec![accessible_handler, application_handler, event_handler];
        *self.app_root_registration.borrow_mut() = Some(connection.register_objects(ROOT_PATH, targets));
    }

    fn register_cache_path(&self) {
        let (Some(connection), Some(cache_handler)) = (self.a11y_connection(), self.cache_handler.borrow().clone())
        else {
            return;
        };

        if let Some(registration) = self.cache_registration.borrow_mut().take() {
            registration.dispose();
        }

        let targets: Vec<Rc<dyn DBusInterface>> = vec![cache_handler];
        *self.cache_registration.borrow_mut() = Some(connection.register_objects(CACHE_PATH, targets));
    }

    async fn get_accessibility_bus_address_async() -> String {
        let result: zbus::Result<String> = async {
            let connection = zbus::blocking::connection::Builder::session()?.build()?.into_inner();
            let proxy = OrgA11yBusProxy::builder(&connection)
                .destination(BUS_NAME_A11Y)?
                .path(PATH_A11Y)?
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            proxy.get_address().await
        }
        .await;

        match result {
            Ok(address) => address,
            Err(e) => {
                if let Some(logger) = Logger::try_get(LogEventLevel::Debug, LogArea::FREE_DESKTOP_PLATFORM) {
                    logger.log(None, &format!("Failed to resolve AT-SPI accessibility bus address: {e}"));
                }
                String::new()
            }
        }
    }

    async fn embed_application_async(&self) -> zbus::Result<()> {
        let Some(connection) = self.a11y_connection() else { return Ok(()) };

        let proxy = OrgA11yAtspiSocketProxy::builder(connection.connection())
            .destination(BUS_NAME_REGISTRY)?
            .path(ROOT_PATH)?
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        proxy.embed(&AtSpiObjectReference::new(self.unique_name(), ROOT_PATH).to_wire()).await?;
        Ok(())
    }

    fn release_node(&self, node: &Rc<AtSpiNode>) {
        node.detach();
        self.nodes_by_peer.borrow_mut().remove(node.peer());
    }
}

/// The path of the node with the number `id`.
pub(crate) fn node_path(id: i32) -> String {
    format!("{APP_PATH_PREFIX}/{id}")
}

fn index_of(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

#[cfg(test)]
mod path_tests {
    // Not from the reference, which has no tests of this project.
    use super::*;

    #[test]
    fn node_paths_are_numbered_below_the_prefix_of_the_port() {
        let server = AtSpiServer::new();
        assert_eq!(server.allocate_node_path(), "/org/ferroui/a11y/1");
        assert_eq!(server.allocate_node_path(), "/org/ferroui/a11y/2");
        assert_eq!(node_path(17), "/org/ferroui/a11y/17");
        assert!(zbus::zvariant::ObjectPath::try_from(node_path(17)).is_ok());
    }

    #[test]
    fn a_server_that_was_not_started_has_only_null_references() {
        let server = AtSpiServer::new();
        assert_eq!(server.get_null_reference(), AtSpiObjectReference::new("", "/org/a11y/atspi/null"));
        assert_eq!(server.get_reference(None), server.get_null_reference());
        assert_eq!(server.get_root_reference(), AtSpiObjectReference::new("", "/org/a11y/atspi/accessible/root"));
        // Without a tracker the server is chatty.
        assert!(server.has_event_listeners());
    }
}
