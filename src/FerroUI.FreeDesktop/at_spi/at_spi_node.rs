//! The port of `AtSpiNode.cs`: an automation peer as an object of the
//! accessibility bus.

use super::application_node_application_handler::ApplicationNodeApplicationHandler;
use super::at_spi_constants::*;
use super::at_spi_server::AtSpiServer;
use super::dbus::connection::AtSpiConnection;
use super::dbus::interface::DBusInterface;
use super::handlers::at_spi_accessible_handler::AtSpiAccessibleHandler;
use super::handlers::at_spi_action_handler::AtSpiActionHandler;
use super::handlers::at_spi_component_handler::AtSpiComponentHandler;
use super::handlers::at_spi_editable_text_handler::AtSpiEditableTextHandler;
use super::handlers::at_spi_event_object_handler::AtSpiEventObjectHandler;
use super::handlers::to_i32;
use super::handlers::at_spi_event_window_handler::AtSpiEventWindowHandler;
use super::handlers::at_spi_image_handler::AtSpiImageHandler;
use super::handlers::at_spi_selection_handler::AtSpiSelectionHandler;
use super::handlers::at_spi_text_handler::AtSpiTextHandler;
use super::handlers::at_spi_value_handler::AtSpiValueHandler;
use super::root_at_spi_node::RootAtSpiNode;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, Ref};
use super::dbus::types::AtSpiAttributeSet;
use ferroui_controls::automation::peers::{AutomationControlType, AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, IRangeValueProvider, IRootProvider, IScrollProvider,
    ISelectionItemProvider, ISelectionProvider, IToggleProvider, IValueProvider, ToggleState,
};
use ferroui_controls::automation::{
    AutomationElementIdentifiers, AutomationProperties, AutomationPropertyChangedEventArgs, ExpandCollapsePatternIdentifiers,
    ExpandCollapseState, SelectionPatternIdentifiers, TogglePatternIdentifiers, ValuePatternIdentifiers,
};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::rc::{Rc, Weak};
use zbus::zvariant::Value;

/// The handlers of a node, one per interface it has.
#[derive(Clone, Default)]
pub(crate) struct NodeHandlers {
    pub(crate) accessible: Option<Rc<AtSpiAccessibleHandler>>,
    pub(crate) application: Option<Rc<ApplicationNodeApplicationHandler>>,
    pub(crate) component: Option<Rc<AtSpiComponentHandler>>,
    pub(crate) action: Option<Rc<AtSpiActionHandler>>,
    pub(crate) value: Option<Rc<AtSpiValueHandler>>,
    pub(crate) selection: Option<Rc<AtSpiSelectionHandler>>,
    pub(crate) text: Option<Rc<AtSpiTextHandler>>,
    pub(crate) editable_text: Option<Rc<AtSpiEditableTextHandler>>,
    pub(crate) image: Option<Rc<AtSpiImageHandler>>,
    pub(crate) event_object: Option<Rc<AtSpiEventObjectHandler>>,
    pub(crate) event_window: Option<Rc<AtSpiEventWindowHandler>>,
}

pub(crate) struct AtSpiNode {
    weak_self: Weak<AtSpiNode>,
    pub(super) detached: Cell<bool>,
    attached: Cell<bool>,
    children_dirty: Cell<bool>,
    attached_children: RefCell<Vec<Rc<AtSpiNode>>>,
    path: String,
    peer: Ref<AutomationPeer>,
    server: Weak<AtSpiServer>,
    parent: RefCell<Option<Weak<AtSpiNode>>>,
    path_registration: RefCell<Option<Rc<dyn IDisposable>>>,
    handlers: RefCell<NodeHandlers>,
    peer_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The part of a node of a window (`RootAtSpiNode`).
    root: Option<RootAtSpiNode>,
}

impl AtSpiNode {
    /// `Create`: a node of a window for a peer that is a root, a plain
    /// node otherwise.
    pub(crate) fn create(peer: &Ref<AutomationPeer>, server: &Rc<AtSpiServer>) -> Rc<AtSpiNode> {
        let root_provider = peer.get_provider::<dyn IRootProvider>();
        Rc::new_cyclic(|weak_self: &Weak<AtSpiNode>| AtSpiNode {
            weak_self: weak_self.clone(),
            detached: Cell::new(false),
            attached: Cell::new(false),
            children_dirty: Cell::new(true),
            attached_children: RefCell::new(Vec::new()),
            path: server.allocate_node_path(),
            peer: peer.clone(),
            server: Rc::downgrade(server),
            parent: RefCell::new(None),
            path_registration: RefCell::new(None),
            handlers: RefCell::new(NodeHandlers::default()),
            peer_subscriptions: RefCell::new(Vec::new()),
            root: root_provider.map(|provider| RootAtSpiNode::new(provider, weak_self.clone())),
        })
    }

    pub(crate) fn peer(&self) -> &Ref<AutomationPeer> {
        &self.peer
    }

    /// The server; `None` once it was dropped.
    pub(crate) fn server(&self) -> Option<Rc<AtSpiServer>> {
        self.server.upgrade()
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn is_attached(&self) -> bool {
        self.attached.get() && !self.detached.get()
    }

    pub(crate) fn parent(&self) -> Option<Rc<AtSpiNode>> {
        self.parent.borrow().as_ref().and_then(Weak::upgrade)
    }

    pub(crate) fn attached_children(&self) -> Vec<Rc<AtSpiNode>> {
        self.attached_children.borrow().clone()
    }

    /// The node as the node of a window.
    pub(crate) fn as_root(&self) -> Option<&RootAtSpiNode> {
        self.root.as_ref()
    }

    pub(crate) fn event_object_handler(&self) -> Option<Rc<AtSpiEventObjectHandler>> {
        self.handlers.borrow().event_object.clone()
    }

    pub(crate) fn event_window_handler(&self) -> Option<Rc<AtSpiEventWindowHandler>> {
        self.handlers.borrow().event_window.clone()
    }

    pub(crate) fn get_supported_interfaces(&self) -> BTreeSet<&'static str> {
        let handlers = self.handlers.borrow();
        let mut interfaces = BTreeSet::from([IFACE_ACCESSIBLE, IFACE_COMPONENT]);
        if handlers.application.is_some() {
            interfaces.insert(IFACE_APPLICATION);
        }
        if handlers.action.is_some() {
            interfaces.insert(IFACE_ACTION);
        }
        if handlers.value.is_some() {
            interfaces.insert(IFACE_VALUE);
        }
        if handlers.selection.is_some() {
            interfaces.insert(IFACE_SELECTION);
        }
        if handlers.text.is_some() {
            interfaces.insert(IFACE_TEXT);
        }
        if handlers.editable_text.is_some() {
            interfaces.insert(IFACE_EDITABLE_TEXT);
        }
        if handlers.image.is_some() {
            interfaces.insert(IFACE_IMAGE);
        }
        interfaces
    }

    pub(crate) fn build_and_register_handlers(&self, connection: &AtSpiConnection) {
        let previous_registration = self.path_registration.borrow_mut().take();
        let server = self.server.clone();
        let node = self.weak_self.clone();
        let peer = &self.peer;
        let mut handlers = NodeHandlers::default();
        let mut targets: Vec<Rc<dyn DBusInterface>> = Vec::new();

        // Accessible - always present
        let accessible = Rc::new(AtSpiAccessibleHandler::new(server.clone(), node.clone()));
        targets.push(accessible.clone());
        handlers.accessible = Some(accessible);

        if peer.get_provider::<dyn IRootProvider>().is_some() {
            let application = Rc::new(ApplicationNodeApplicationHandler::new());
            targets.push(application.clone());
            handlers.application = Some(application);
        }

        // Component - all visual elements
        let component = Rc::new(AtSpiComponentHandler::new(server.clone(), node.clone()));
        targets.push(component.clone());
        handlers.component = Some(component);

        if peer.get_provider::<dyn IInvokeProvider>().is_some()
            || peer.get_provider::<dyn IToggleProvider>().is_some()
            || peer.get_provider::<dyn IExpandCollapseProvider>().is_some()
            || peer.get_provider::<dyn IScrollProvider>().is_some()
            || peer.get_provider::<dyn ISelectionItemProvider>().is_some()
        {
            let action = Rc::new(AtSpiActionHandler::new(server.clone(), node.clone()));
            targets.push(action.clone());
            handlers.action = Some(action);
        }

        let has_range_value = peer.get_provider::<dyn IRangeValueProvider>().is_some();
        if has_range_value {
            let value = Rc::new(AtSpiValueHandler::new(server.clone(), node.clone()));
            targets.push(value.clone());
            handlers.value = Some(value);
        }

        if peer.get_provider::<dyn ISelectionProvider>().is_some() {
            let selection = Rc::new(AtSpiSelectionHandler::new(server.clone(), node.clone()));
            targets.push(selection.clone());
            handlers.selection = Some(selection);
        }

        if let (Some(value_provider), false) = (peer.get_provider::<dyn IValueProvider>(), has_range_value) {
            let text = Rc::new(AtSpiTextHandler::new(node.clone()));
            targets.push(text.clone());
            handlers.text = Some(text);
            if !value_provider.is_read_only() {
                let editable_text = Rc::new(AtSpiEditableTextHandler::new(node.clone()));
                targets.push(editable_text.clone());
                handlers.editable_text = Some(editable_text);
            }
        }

        if peer.get_automation_control_type() == AutomationControlType::Image {
            let image = Rc::new(AtSpiImageHandler::new(server.clone(), node.clone()));
            targets.push(image.clone());
            handlers.image = Some(image);
        }

        // Event handlers - always present
        let event_object = Rc::new(AtSpiEventObjectHandler::new(server.clone(), self.path.clone()));
        targets.push(event_object.clone());
        handlers.event_object = Some(event_object);
        if self.root.is_some() {
            let event_window = Rc::new(AtSpiEventWindowHandler::new(server, self.path.clone()));
            targets.push(event_window.clone());
            handlers.event_window = Some(event_window);
        }

        *self.handlers.borrow_mut() = handlers;

        // `ReplacePathRegistrationAsync`: the previous registration goes first.
        if let Some(previous) = previous_registration {
            previous.dispose();
        }
        *self.path_registration.borrow_mut() = Some(connection.register_objects(&self.path, targets));
    }

    pub(crate) fn get_accessible_name(peer: &AutomationPeer) -> String {
        let name = peer.get_name();
        if !name.trim().is_empty() {
            return name;
        }

        let visual_type_name = peer.get_class_name();
        if visual_type_name.trim().is_empty() {
            String::new()
        } else {
            visual_type_name
        }
    }

    /// Builds the AT-SPI object attributes reported for a peer: the attributes shared by
    /// `Accessible.GetAttributes` and Collection matching.
    pub(crate) fn get_attributes(peer: &Ref<AutomationPeer>) -> AtSpiAttributeSet {
        let mut attrs = AtSpiAttributeSet::new();
        attrs.insert("toolkit".to_string(), TOOLKIT_NAME.to_string());

        if !peer.get_name().is_empty() {
            attrs.insert("explicit-name".to_string(), "true".to_string());
        }

        if let Some(accelerator_key) = peer.get_accelerator_key().filter(|key| !key.is_empty()) {
            attrs.insert("accelerator-key".to_string(), accelerator_key);
        }

        if let Some(access_key) = peer.get_access_key().filter(|key| !key.is_empty()) {
            attrs.insert("access-key".to_string(), access_key);
        }

        let placeholder_text = peer.get_placeholder_text();
        if !placeholder_text.is_empty() {
            attrs.insert("placeholder-text".to_string(), placeholder_text);
        }

        let heading_level = peer.get_heading_level();
        if heading_level > 0 {
            attrs.insert("level".to_string(), heading_level.to_string());
        }

        if let Some(owner) = peer.cast::<ControlAutomationPeer>().and_then(|control_peer| control_peer.try_owner()) {
            let position = AutomationProperties::get_position_in_set(&owner);
            if position > 0 {
                attrs.insert("posinset".to_string(), position.to_string());
            }

            let size = AutomationProperties::get_size_of_set(&owner);
            if size > 0 {
                attrs.insert("setsize".to_string(), size.to_string());
            }
        }

        attrs
    }

    pub(crate) fn attach(&self, parent: Option<&Rc<AtSpiNode>>) {
        if self.detached.get() {
            return;
        }

        if self.attached.get() {
            self.set_parent(parent);
            return;
        }

        self.attached.set(true);
        self.children_dirty.set(true);
        self.set_parent(parent);

        let weak = self.weak_self.clone();
        let children_changed = self.peer.children_changed(move || {
            if let Some(node) = weak.upgrade() {
                node.on_peer_children_changed();
            }
        });
        let weak = self.weak_self.clone();
        let property_changed = self.peer.property_changed(move |e| {
            if let Some(node) = weak.upgrade() {
                node.on_peer_property_changed(e);
            }
        });
        *self.peer_subscriptions.borrow_mut() = vec![children_changed, property_changed];

        if let Some(connection) = self.server().and_then(|server| server.a11y_connection()) {
            self.build_and_register_handlers(&connection);
        }
    }

    pub(crate) fn set_parent(&self, parent: Option<&Rc<AtSpiNode>>) {
        *self.parent.borrow_mut() = parent.map(Rc::downgrade);
    }

    pub(crate) fn remove_attached_child(&self, child: &Rc<AtSpiNode>) -> bool {
        let mut children = self.attached_children.borrow_mut();
        match children.iter().position(|attached| Rc::ptr_eq(attached, child)) {
            Some(index) => {
                children.remove(index);
                true
            }
            None => false,
        }
    }

    // Present a selection container's realized item containers as its direct
    // AT-SPI children so SelectChild-by-index and parent lookups work.
    fn get_child_peers(&self) -> Vec<Ref<AutomationPeer>> {
        let children = self.peer.get_children();
        if self.peer.get_provider::<dyn ISelectionProvider>().is_none() {
            return children.to_vec();
        }

        let mut items = Vec::new();
        Self::collect_selection_item_peers(&children, &mut items);
        if items.is_empty() {
            children.to_vec()
        } else {
            items
        }
    }

    fn collect_selection_item_peers(peers: &[Ref<AutomationPeer>], result: &mut Vec<Ref<AutomationPeer>>) {
        for peer in peers {
            if peer.get_provider::<dyn ISelectionItemProvider>().is_some() {
                result.push(peer.clone());
            } else {
                Self::collect_selection_item_peers(&peer.get_children(), result);
            }
        }
    }

    pub(crate) fn ensure_children(&self) -> Vec<Rc<AtSpiNode>> {
        let (Some(server), Some(this)) = (self.server(), self.weak_self.upgrade()) else { return Vec::new() };
        if !self.is_attached() {
            return Vec::new();
        }

        if self.children_dirty.get() {
            Self::update_children(&server, &this, None);
        }

        self.attached_children.borrow().clone()
    }

    /// Lists the removals and additions that turn one child list into another: each removed
    /// child with its index in `old_children`, highest index first, into `removed`, and each
    /// added child with its index in `new_children`, lowest index first, into `added`.
    ///
    /// Applying the removals and then the additions in the order given turns the old list into
    /// the new one. A child that changed position is reported as removed and added. `key` is the
    /// identity of a child (the reference identity of upstream's class constraint).
    pub(crate) fn diff_children<T: Clone, K: Eq + Hash>(
        old_children: &[T],
        new_children: &[T],
        key: impl Fn(&T) -> K,
        removed: &mut Vec<(usize, T)>,
        added: &mut Vec<(usize, T)>,
    ) {
        let mut old_indices: HashMap<K, usize> = HashMap::with_capacity(old_children.len());
        for (i, child) in old_children.iter().enumerate() {
            old_indices.insert(key(child), i);
        }

        // Children that keep their place must stay in the same order in both lists. Keeping the
        // longest increasing run of old indices, taken in new order, reports the fewest moves.
        let mut common: Vec<&T> = Vec::new();
        let mut common_old_indices: Vec<usize> = Vec::new();
        for child in new_children {
            if let Some(old_index) = old_indices.get(&key(child)) {
                common.push(child);
                common_old_indices.push(*old_index);
            }
        }

        let mut kept: HashSet<K> = HashSet::new();
        for position in Self::longest_increasing_run(&common_old_indices) {
            kept.insert(key(common[position]));
        }

        for i in (0..old_children.len()).rev() {
            if !kept.contains(&key(&old_children[i])) {
                removed.push((i, old_children[i].clone()));
            }
        }

        for (i, child) in new_children.iter().enumerate() {
            if !kept.contains(&key(child)) {
                added.push((i, child.clone()));
            }
        }
    }

    // Returns the positions of a longest strictly increasing subsequence of the values.
    fn longest_increasing_run(values: &[usize]) -> Vec<usize> {
        // tails[k] is the position of the smallest value that ends an increasing run of length k + 1.
        let mut tails: Vec<usize> = Vec::new();
        let mut previous: Vec<Option<usize>> = vec![None; values.len()];
        for i in 0..values.len() {
            let (mut low, mut high) = (0, tails.len());
            while low < high {
                let middle = (low + high) / 2;
                if values[tails[middle]] < values[i] {
                    low = middle + 1;
                } else {
                    high = middle;
                }
            }

            previous[i] = if low > 0 { Some(tails[low - 1]) } else { None };
            if low == tails.len() {
                tails.push(i);
            } else {
                tails[low] = i;
            }
        }

        let mut run = Vec::with_capacity(tails.len());
        let mut i = tails.last().copied();
        while let Some(position) = i {
            run.push(position);
            i = previous[position];
        }
        run
    }

    // Rebuilds the child list. With an event handler, reports each change to clients as AT-SPI
    // specifies.
    fn update_children(server: &Rc<AtSpiServer>, this: &Rc<AtSpiNode>, event_handler: Option<Rc<AtSpiEventObjectHandler>>) {
        let old_children = this.attached_children.borrow().clone();
        let child_peers = this.get_child_peers();
        let mut new_children: Vec<Rc<AtSpiNode>> = Vec::with_capacity(child_peers.len());
        for child_peer in &child_peers {
            let child_node = server.get_or_create_node(child_peer);
            if server.attach_node(&child_node, Some(this)) {
                new_children.push(child_node);
            }
        }

        *this.attached_children.borrow_mut() = new_children.clone();
        this.children_dirty.set(false);

        if old_children.is_empty() && event_handler.is_none() {
            return;
        }

        let mut removed = Vec::new();
        let mut added = Vec::new();
        Self::diff_children(&old_children, &new_children, |child| Rc::as_ptr(child), &mut removed, &mut added);

        if !removed.is_empty() {
            let remaining: HashSet<*const AtSpiNode> = new_children.iter().map(Rc::as_ptr).collect();
            for (index, child) in &removed {
                // The reference must be sent before the child is detached, when it becomes the null reference.
                if let Some(event_handler) = &event_handler {
                    event_handler.emit_children_changed_signal("remove", to_i32(*index), Self::get_child_variant(server, child));
                }

                if !remaining.contains(&Rc::as_ptr(child)) && child.parent().is_some_and(|parent| Rc::ptr_eq(&parent, this)) {
                    server.detach_subtree_recursive(child);
                }
            }
        }

        let Some(event_handler) = event_handler else {
            return;
        };

        for (index, child) in &added {
            event_handler.emit_children_changed_signal("add", to_i32(*index), Self::get_child_variant(server, child));
        }
    }

    fn get_child_variant(server: &AtSpiServer, child: &Rc<AtSpiNode>) -> Value<'static> {
        server.get_reference(Some(child)).to_dbus_struct()
    }

    pub(crate) fn detach(&self) {
        if self.detached.get() {
            return;
        }

        if let Some(root) = &self.root {
            root.detach();
        }

        self.detached.set(true);
        self.attached.set(false);
        self.children_dirty.set(true);
        self.attached_children.borrow_mut().clear();
        *self.parent.borrow_mut() = None;
        for subscription in self.peer_subscriptions.borrow_mut().drain(..) {
            subscription.dispose();
        }
        self.dispose_path_registration();
    }

    pub(crate) fn dispose_path_registration(&self) {
        if let Some(registration) = self.path_registration.borrow_mut().take() {
            registration.dispose();
        }
    }

    fn on_peer_children_changed(&self) {
        let (Some(server), Some(this)) = (self.server(), self.weak_self.upgrade()) else { return };
        if server.a11y_connection().is_none() || !self.is_attached() {
            return;
        }

        // No client has read these children, so there is nothing to report. The next read builds them.
        if self.children_dirty.get() {
            return;
        }

        let event_handler = if server.has_event_listeners() { self.event_object_handler() } else { None };
        Self::update_children(&server, &this, event_handler);
    }

    fn on_peer_property_changed(&self, e: &AutomationPropertyChangedEventArgs) {
        let Some(server) = self.server() else { return };
        if server.a11y_connection().is_none() || !server.has_event_listeners() {
            return;
        }

        let Some(event_handler) = self.event_object_handler() else { return };
        let property = e.property();

        if std::ptr::eq(property, AutomationElementIdentifiers::name_property()) {
            event_handler
                .emit_property_change_signal("accessible-name", Value::from(Self::get_accessible_name(&self.peer)));
        } else if std::ptr::eq(property, AutomationElementIdentifiers::help_text_property()) {
            event_handler
                .emit_property_change_signal("accessible-description", Value::from(value_to_string(e.new_value())));
        } else if std::ptr::eq(property, TogglePatternIdentifiers::toggle_state_property()) {
            let new_state = e
                .new_value()
                .and_then(|value| value.downcast_ref::<ToggleState>().copied())
                .unwrap_or(ToggleState::Off);
            event_handler.emit_state_changed_signal("checked", i32::from(new_state == ToggleState::On), Value::from(0i32));
            event_handler.emit_state_changed_signal(
                "indeterminate",
                i32::from(new_state == ToggleState::Indeterminate),
                Value::from(0i32),
            );
        } else if std::ptr::eq(property, ExpandCollapsePatternIdentifiers::expand_collapse_state_property()) {
            let new_state = e
                .new_value()
                .and_then(|value| value.downcast_ref::<ExpandCollapseState>().copied())
                .unwrap_or(ExpandCollapseState::Collapsed);
            event_handler.emit_state_changed_signal(
                "expanded",
                i32::from(new_state == ExpandCollapseState::Expanded),
                Value::from(0i32),
            );
            event_handler.emit_state_changed_signal(
                "collapsed",
                i32::from(new_state == ExpandCollapseState::Collapsed),
                Value::from(0i32),
            );
        } else if std::ptr::eq(property, ValuePatternIdentifiers::value_property()) {
            event_handler.emit_property_change_signal("accessible-value", Value::from(value_to_string(e.new_value())));
        } else if std::ptr::eq(property, SelectionPatternIdentifiers::selection_property()) {
            event_handler.emit_selection_changed_signal();
        } else if std::ptr::eq(property, AutomationElementIdentifiers::bounding_rectangle_property()) {
            event_handler.emit_bounds_changed_signal();
        }
    }
}

/// `e.NewValue?.ToString() ?? string.Empty` for the values the name,
/// help text and value properties carry: text.
pub(crate) fn value_to_string(value: Option<&BoxedValue>) -> String {
    let Some(value) = value else { return String::new() };
    if let Some(text) = value.downcast_ref::<String>() {
        return text.clone();
    }
    if let Some(text) = value.downcast_ref::<Option<String>>() {
        return text.clone().unwrap_or_default();
    }
    if let Some(text) = value.downcast_ref::<&'static str>() {
        return (*text).to_string();
    }
    String::new()
}
