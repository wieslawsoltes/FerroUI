//! The accessibility tree of a view in the page.
//!
//! Not in the original, whose browser backend draws to its canvas only: a
//! screen reader finds nothing in the page and the element picker of the
//! developer tools finds the canvas. A view mirrors the automation tree of
//! its controls (the automation peers that UI Automation, AT-SPI and the
//! Android access helper read on the other platforms) into the
//! accessibility host of the view: one element per peer, nested as the
//! peers are, at the bounds of the peer, with the ARIA role, name, states
//! and value of the peer (`webapp/modules/ferroui/accessibility.ts`,
//! docs/porting/browser-accessibility.md).
//!
//! What changes is sent once per animation frame, and only what changed:
//!
//! - a peer whose children changed has its subtree walked again; the rest
//!   of the tree is not walked;
//! - a peer whose properties changed has its attributes read again;
//! - a layout pass reads the bounds of the known peers and nothing else;
//! - each element is sent only the fields that differ from what the page
//!   has.
//!
//! A page cannot tell whether a screen reader runs, so by default the mirror
//! is built on demand, as Flutter and Uno Platform do on the web: the host
//! holds a visually hidden "Enable accessibility" button, which a screen
//! reader finds first, and activating it builds the mirror.
//! [`BrowserPlatformOptions::accessibility`](crate::BrowserPlatformOptions::accessibility)
//! chooses on demand, always or never for new views;
//! [`FerroView::set_accessibility_enabled`](crate::FerroView::set_accessibility_enabled)
//! turns the mirror on or off for one view.

use crate::interop::{accessibility_helper, JsObject};
use crate::BrowserAccessibilityMode;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{Rect, Ref};
use ferroui_controls::automation::peers::{AutomationControlType, AutomationLandmarkType, AutomationPeer, ControlAutomationPeer};
use ferroui_controls::automation::provider::{
    IEmbeddedRootProvider, IExpandCollapseProvider, IInvokeProvider, IRangeValueProvider, ISelectionItemProvider,
    IToggleProvider, IValueProvider, ToggleState,
};
use ferroui_controls::automation::{AutomationLiveSetting, ExpandCollapseState};
use ferroui_controls::TopLevel;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::rc::{Rc, Weak};

ferroui_base::ferro_thread_local! {
    /// The accessibility trees of the views, by the id of their top-level.
    static TREES: RefCell<HashMap<i32, Weak<BrowserAccessibility>>> = RefCell::new(HashMap::new());
}

/// The ARIA role of a control type; `None` for an element without a role
/// (its text is read as it is).
pub(crate) fn aria_role(control_type: AutomationControlType, toggles: bool) -> Option<&'static str> {
    use AutomationControlType as T;
    Some(match control_type {
        T::Button | T::SplitButton => "button",
        T::CheckBox => "checkbox",
        T::RadioButton => "radio",
        T::ComboBox => "combobox",
        T::ComboBoxItem | T::ListItem => "option",
        T::List => "listbox",
        T::Edit => "textbox",
        T::Hyperlink => "link",
        T::Image => "img",
        T::Menu => "menu",
        T::MenuBar => "menubar",
        T::MenuItem if toggles => "menuitemcheckbox",
        T::MenuItem => "menuitem",
        T::ProgressBar => "progressbar",
        T::ScrollBar => "scrollbar",
        T::Slider => "slider",
        T::Spinner => "spinbutton",
        T::StatusBar => "status",
        T::Tab => "tablist",
        T::TabItem => "tab",
        T::ToolBar => "toolbar",
        T::ToolTip => "tooltip",
        T::Tree => "tree",
        T::TreeItem => "treeitem",
        T::DataGrid | T::Table | T::Calendar => "grid",
        T::DataItem => "row",
        T::Header => "rowgroup",
        T::HeaderItem => "columnheader",
        T::Document => "document",
        T::Separator => "separator",
        T::Group | T::Expander => "group",
        T::Window | T::Pane | T::ScrollViewer => "region",
        T::TitleBar => "banner",
        T::Text | T::Thumb | T::Custom | T::None => return None,
    })
}

/// The ARIA landmark role of a landmark type.
pub(crate) fn landmark_role(landmark: AutomationLandmarkType) -> &'static str {
    match landmark {
        AutomationLandmarkType::Banner => "banner",
        AutomationLandmarkType::Complementary => "complementary",
        AutomationLandmarkType::ContentInfo => "contentinfo",
        AutomationLandmarkType::Region => "region",
        AutomationLandmarkType::Form => "form",
        AutomationLandmarkType::Main => "main",
        AutomationLandmarkType::Navigation => "navigation",
        AutomationLandmarkType::Search => "search",
    }
}

/// What the page shows of a peer. The bounds are relative to the element
/// of the parent; the states are ARIA attributes by name.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct NodeState {
    pub role: String,
    pub name: String,
    pub help: String,
    pub text: String,
    pub placeholder: String,
    pub bounds: [f64; 4],
    pub states: Vec<(&'static str, String)>,
    pub class_name: String,
    pub automation_id: String,
}

/// Reads the attributes of a peer (everything but the bounds).
pub(crate) fn read_attributes(peer: &AutomationPeer, state: &mut NodeState) {
    let control_type = peer.get_automation_control_type();
    let toggle = peer.get_provider::<dyn IToggleProvider>();
    // A landmark (AutomationProperties.LandmarkType) names the region a container is.
    let role = match (aria_role(control_type, toggle.is_some()), peer.get_landmark_type()) {
        (None | Some("group" | "region"), Some(landmark)) => landmark_role(landmark),
        (role, _) => role.unwrap_or(""),
    };
    let name = peer.get_name();
    let value = peer.get_provider::<dyn IValueProvider>();
    let range = peer.get_provider::<dyn IRangeValueProvider>();

    state.role = role.to_owned();
    // An element without a role (a text) is read by its text; one with a role by its name.
    if role.is_empty() {
        state.name.clear();
        state.text = name;
    } else {
        state.text = match (&value, control_type) {
            (Some(value), AutomationControlType::Edit | AutomationControlType::ComboBox) => {
                value.value().unwrap_or_default()
            }
            _ => String::new(),
        };
        state.name = name;
    }
    state.help = peer.get_help_text();
    state.placeholder = peer.get_placeholder_text();
    state.class_name = peer.get_class_name();
    state.automation_id = peer.get_automation_id().unwrap_or_default();

    let mut states: Vec<(&'static str, String)> = Vec::new();
    let flag = |on: bool| if on { "true".to_owned() } else { String::new() };
    states.push(("aria-disabled", flag(!peer.is_enabled())));
    let toggle_value = toggle.map(|toggle| match toggle.toggle_state() {
        ToggleState::On => "true",
        ToggleState::Off => "false",
        ToggleState::Indeterminate => "mixed",
    });
    let (checked, pressed) = match (role, toggle_value) {
        ("button", Some(value)) => (None, Some(value)),
        (_, Some(value)) => (Some(value), None),
        _ => (None, None),
    };
    states.push(("aria-checked", checked.unwrap_or_default().to_owned()));
    states.push(("aria-pressed", pressed.unwrap_or_default().to_owned()));
    let expanded = peer.get_provider::<dyn IExpandCollapseProvider>().and_then(|provider| {
        match provider.expand_collapse_state() {
            ExpandCollapseState::Expanded | ExpandCollapseState::PartiallyExpanded => Some("true"),
            ExpandCollapseState::Collapsed => Some("false"),
            ExpandCollapseState::LeafNode => None,
        }
    });
    states.push(("aria-expanded", expanded.unwrap_or_default().to_owned()));
    let selected = peer.get_provider::<dyn ISelectionItemProvider>().map(|provider| provider.is_selected());
    states.push(("aria-selected", selected.map(|selected| selected.to_string()).unwrap_or_default()));
    let read_only = value.as_ref().map(|value| value.is_read_only()).or(range.as_ref().map(|range| range.is_read_only()));
    states.push(("aria-readonly", flag(read_only == Some(true) && role != "progressbar")));
    match &range {
        Some(range) => {
            states.push(("aria-valuemin", range.minimum().to_string()));
            states.push(("aria-valuemax", range.maximum().to_string()));
            states.push(("aria-valuenow", range.value().to_string()));
        }
        None => {
            states.push(("aria-valuemin", String::new()));
            states.push(("aria-valuemax", String::new()));
            states.push(("aria-valuenow", String::new()));
        }
    }
    let level = peer.get_heading_level();
    states.push(("aria-level", if level > 0 { level.to_string() } else { String::new() }));
    states.push((
        "aria-live",
        match peer.get_live_setting() {
            AutomationLiveSetting::Polite => "polite".to_owned(),
            AutomationLiveSetting::Assertive => "assertive".to_owned(),
            _ => String::new(),
        },
    ));
    state.states = states;
}

/// The change of an element from `old` (what the page has, `None` for a
/// new element) to `new`, as a JSON object of the changed fields; `None`
/// when nothing changed.
pub(crate) fn change_json(id: u32, place: Option<(u32, usize)>, old: Option<&NodeState>, new: &NodeState) -> Option<String> {
    let mut out = String::new();
    let _ = write!(out, "{{\"i\":{id}");
    let mut changed = false;
    if let Some((parent, index)) = place {
        let _ = write!(out, ",\"p\":{parent},\"x\":{index}");
        changed = true;
    }
    let mut field = |key: &str, old: Option<&String>, new: &String| {
        if old != Some(new) {
            out.push_str(",\"");
            out.push_str(key);
            out.push_str("\":");
            push_json_string(&mut out, new);
            changed = true;
        }
    };
    field("r", old.map(|old| &old.role), &new.role);
    field("n", old.map(|old| &old.name), &new.name);
    field("h", old.map(|old| &old.help), &new.help);
    field("t", old.map(|old| &old.text), &new.text);
    field("ph", old.map(|old| &old.placeholder), &new.placeholder);
    field("ct", old.map(|old| &old.class_name), &new.class_name);
    field("ci", old.map(|old| &old.automation_id), &new.automation_id);
    if old.map(|old| old.bounds) != Some(new.bounds) {
        let [x, y, width, height] = new.bounds;
        let _ = write!(out, ",\"b\":[{},{},{},{}]", round(x), round(y), round(width), round(height));
        changed = true;
    }
    let mut states = String::new();
    for (index, (name, value)) in new.states.iter().enumerate() {
        let before = old.and_then(|old| old.states.get(index)).map(|(_, value)| value);
        // A new element has no attributes: an empty state needs no removal.
        if before != Some(value) && !(before.is_none() && value.is_empty()) {
            states.push(if states.is_empty() { '{' } else { ',' });
            push_json_string(&mut states, name);
            states.push(':');
            push_json_string(&mut states, value);
        }
    }
    if !states.is_empty() {
        states.push('}');
        out.push_str(",\"s\":");
        out.push_str(&states);
        changed = true;
    }
    out.push('}');
    changed.then_some(out)
}

fn round(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn peer_key(peer: &AutomationPeer) -> usize {
    peer as *const AutomationPeer as usize
}

/// A peer that has an element in the page.
struct Node {
    id: u32,
    peer: Ref<AutomationPeer>,
    parent: u32,
    index: usize,
    children: Vec<usize>,
    // What the page has; `None` until the element was sent.
    sent: Option<NodeState>,
    // The bounds of the peer in the view, for the relative bounds of its children.
    absolute: Rect,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}

/// The accessibility tree of one view.
pub struct BrowserAccessibility {
    this: Weak<BrowserAccessibility>,
    top_level: Ref<TopLevel>,
    top_level_id: i32,
    host: JsObject,
    // The enable button is shown while the mirror is off.
    on_demand: bool,
    enabled: Cell<bool>,
    root: RefCell<Option<Ref<AutomationPeer>>>,
    nodes: RefCell<HashMap<usize, Node>>,
    ids: RefCell<HashMap<u32, usize>>,
    next_id: Cell<u32>,
    // What changed since the last frame.
    structure_dirty: RefCell<HashSet<usize>>,
    attributes_dirty: RefCell<HashSet<usize>>,
    layout_dirty: Cell<bool>,
    focus_dirty: Cell<bool>,
    focused: Cell<u32>,
    requested: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl BrowserAccessibility {
    /// The tree of the view of `top_level`, in the accessibility host
    /// `host` of its container `container`.
    pub(crate) fn new(
        top_level: Ref<TopLevel>,
        top_level_id: i32,
        host: JsObject,
        container: &JsObject,
        mode: BrowserAccessibilityMode,
    ) -> Rc<Self> {
        accessibility_helper::attach(&host, container, top_level_id);
        let this = Rc::new_cyclic(|this| Self {
            this: this.clone(),
            top_level,
            top_level_id,
            host,
            on_demand: mode == BrowserAccessibilityMode::OnDemand,
            enabled: Cell::new(false),
            root: RefCell::new(None),
            nodes: RefCell::new(HashMap::new()),
            ids: RefCell::new(HashMap::new()),
            next_id: Cell::new(1),
            structure_dirty: RefCell::new(HashSet::new()),
            attributes_dirty: RefCell::new(HashSet::new()),
            layout_dirty: Cell::new(false),
            focus_dirty: Cell::new(false),
            focused: Cell::new(0),
            requested: Cell::new(false),
            subscriptions: RefCell::new(Vec::new()),
        });
        TREES.with(|trees| trees.borrow_mut().insert(top_level_id, Rc::downgrade(&this)));
        match mode {
            BrowserAccessibilityMode::Enabled => this.set_enabled(true),
            BrowserAccessibilityMode::OnDemand => accessibility_helper::set_enable_button(&this.host, true),
            BrowserAccessibilityMode::Disabled => {}
        }
        this
    }

    /// The tree of the view of a top-level.
    pub(crate) fn get(top_level_id: i32) -> Option<Rc<BrowserAccessibility>> {
        TREES.with(|trees| trees.borrow().get(&top_level_id).and_then(Weak::upgrade))
    }

    /// The tree of the view of a top-level: an application that does not
    /// keep its [`FerroView`](crate::FerroView) turns the mirror on or off
    /// through it.
    pub fn of(top_level: &Ref<TopLevel>) -> Option<Rc<BrowserAccessibility>> {
        TREES.with(|trees| {
            trees.borrow().values().filter_map(Weak::upgrade).find(|tree| tree.top_level.ptr_eq(top_level))
        })
    }

    /// Whether the tree is mirrored into the page.
    pub fn is_enabled(&self) -> bool {
        self.enabled.get()
    }

    /// Turns the mirror on (the whole tree is sent at the next frame) or
    /// off (every element is removed and nothing is followed any more).
    pub fn set_enabled(&self, enabled: bool) {
        if self.enabled.replace(enabled) == enabled {
            return;
        }
        if enabled {
            accessibility_helper::set_enable_button(&self.host, false);
            let root = ControlAutomationPeer::create_peer_for_element(&self.top_level);
            let weak = self.this.clone();
            let mut subscriptions: Vec<Rc<dyn IDisposable>> = vec![self.top_level.layout_updated(move || {
                if let Some(this) = weak.upgrade() {
                    this.layout_dirty.set(true);
                    this.request();
                }
            })];
            if let Some(provider) = root.get_provider::<dyn IEmbeddedRootProvider>() {
                let weak = self.this.clone();
                subscriptions.push(provider.focus_changed(Rc::new(move || {
                    if let Some(this) = weak.upgrade() {
                        this.focus_dirty.set(true);
                        this.request();
                    }
                })));
            }
            *self.subscriptions.borrow_mut() = subscriptions;
            let key = peer_key(&root);
            *self.root.borrow_mut() = Some(root);
            self.structure_dirty.borrow_mut().insert(key);
            self.layout_dirty.set(true);
            self.focus_dirty.set(true);
            self.request();
        } else {
            for subscription in self.subscriptions.borrow_mut().drain(..) {
                subscription.dispose();
            }
            for (_, node) in self.nodes.borrow_mut().drain() {
                for subscription in node.subscriptions {
                    subscription.dispose();
                }
            }
            self.ids.borrow_mut().clear();
            self.structure_dirty.borrow_mut().clear();
            self.attributes_dirty.borrow_mut().clear();
            self.root.borrow_mut().take();
            self.focused.set(0);
            self.requested.set(false);
            accessibility_helper::clear(&self.host);
            accessibility_helper::set_enable_button(&self.host, self.on_demand);
        }
    }

    fn request(&self) {
        if self.enabled.get() && !self.requested.replace(true) {
            accessibility_helper::request_update(&self.host);
        }
    }

    /// Sends what changed since the last frame (the animation frame the
    /// page asked for with `request`).
    pub(crate) fn on_frame(&self) {
        self.requested.set(false);
        if !self.enabled.get() {
            return;
        }
        let Some(root) = self.root.borrow().clone() else {
            return;
        };
        let root_key = peer_key(&root);
        let mut changes: Vec<String> = Vec::new();

        // The subtrees whose children changed: walked again from the highest dirty peer.
        let dirty: Vec<usize> = self.structure_dirty.borrow_mut().drain().collect();
        let mut placed: Vec<usize> = Vec::new();
        let mut reached: HashSet<usize> = HashSet::new();
        let mut released: Vec<usize> = Vec::new();
        for key in dirty {
            let (peer, parent, index) = if key == root_key {
                (root.clone(), 0, 0)
            } else {
                let nodes = self.nodes.borrow();
                match nodes.get(&key) {
                    Some(node) => (node.peer.clone(), node.parent, node.index),
                    None => continue,
                }
            };
            if reached.contains(&key) {
                continue;
            }
            // What the subtree had, to release what it no longer has.
            self.collect_subtree(key, &mut released);
            if key == root_key {
                // The root itself has no element: the view is the container. Its children are
                // the top elements of the host.
                let children = peer.get_children();
                let mut child_keys = Vec::with_capacity(children.len());
                for (child_index, child) in children.iter().enumerate() {
                    child_keys.push(self.walk(child, 0, child_index, &mut placed, &mut reached));
                }
                reached.insert(root_key);
                self.ensure_root_node(&root, child_keys);
            } else {
                self.walk(&peer, parent, index, &mut placed, &mut reached);
            }
        }
        for key in released {
            if !reached.contains(&key) && key != root_key {
                if let Some(node) = self.nodes.borrow_mut().remove(&key) {
                    self.ids.borrow_mut().remove(&node.id);
                    for subscription in node.subscriptions {
                        subscription.dispose();
                    }
                    if node.sent.is_some() {
                        changes.push(format!("{{\"i\":{},\"d\":1}}", node.id));
                    }
                }
            }
        }

        // Attributes of new and changed peers, bounds after a layout pass, in tree order.
        let attributes_dirty: HashSet<usize> = self.attributes_dirty.borrow_mut().drain().collect();
        let placed: HashSet<usize> = placed.into_iter().collect();
        let layout = self.layout_dirty.replace(false);
        let mut order: Vec<usize> = Vec::new();
        self.tree_order(root_key, &mut order);
        let mut nodes = self.nodes.borrow_mut();
        let root_bounds = root.get_bounding_rectangle();
        for key in order {
            if key == root_key {
                continue;
            }
            let is_placed = placed.contains(&key);
            let is_new = nodes.get(&key).map(|node| node.sent.is_none()).unwrap_or(false);
            let read_attributes_now = is_new || attributes_dirty.contains(&key);
            if !(is_placed || read_attributes_now || layout) {
                continue;
            }
            let parent_absolute = {
                let node = &nodes[&key];
                match self.ids.borrow().get(&node.parent).and_then(|parent_key| nodes.get(parent_key)) {
                    Some(parent) => parent.absolute,
                    None => root_bounds,
                }
            };
            let node = nodes.get_mut(&key).expect("a node in tree order is known");
            let mut state = node.sent.clone().unwrap_or_default();
            if read_attributes_now {
                read_attributes(&node.peer, &mut state);
            }
            if layout || is_new || is_placed {
                let absolute = node.peer.get_bounding_rectangle();
                node.absolute = absolute;
                state.bounds = [
                    absolute.x - parent_absolute.x,
                    absolute.y - parent_absolute.y,
                    absolute.width.max(0.0),
                    absolute.height.max(0.0),
                ];
            }
            let place = is_placed.then_some((node.parent, node.index));
            if let Some(change) = change_json(node.id, place, node.sent.as_ref(), &state) {
                changes.push(change);
            }
            node.sent = Some(state);
        }
        drop(nodes);

        let mut focus = String::new();
        if self.focus_dirty.replace(false) {
            let focused = root
                .get_provider::<dyn IEmbeddedRootProvider>()
                .and_then(|provider| provider.get_focus())
                .and_then(|peer| self.nodes.borrow().get(&peer_key(&peer)).map(|node| node.id))
                .unwrap_or(0);
            if self.focused.replace(focused) != focused || !changes.is_empty() {
                let _ = write!(focus, ",\"f\":{focused}");
            }
        }
        if changes.is_empty() && focus.is_empty() {
            return;
        }
        let mut batch = String::with_capacity(changes.iter().map(String::len).sum::<usize>() + 16);
        batch.push_str("{\"c\":[");
        for (index, change) in changes.iter().enumerate() {
            if index > 0 {
                batch.push(',');
            }
            batch.push_str(change);
        }
        batch.push(']');
        batch.push_str(&focus);
        batch.push('}');
        accessibility_helper::apply(&self.host, &batch);
    }

    /// Walks a peer and its descendants, creating the nodes it does not
    /// have; answers the key of the peer.
    fn walk(
        &self,
        peer: &Ref<AutomationPeer>,
        parent: u32,
        index: usize,
        placed: &mut Vec<usize>,
        reached: &mut HashSet<usize>,
    ) -> usize {
        let key = peer_key(peer);
        let id = self.node_for(peer, key);
        {
            let mut nodes = self.nodes.borrow_mut();
            let node = nodes.get_mut(&key).expect("node_for made the node");
            if node.parent != parent || node.index != index || node.sent.is_none() {
                node.parent = parent;
                node.index = index;
                placed.push(key);
            }
        }
        reached.insert(key);
        let children = peer.get_children();
        let mut child_keys = Vec::with_capacity(children.len());
        for (child_index, child) in children.iter().enumerate() {
            child_keys.push(self.walk(child, id, child_index, placed, reached));
        }
        if let Some(node) = self.nodes.borrow_mut().get_mut(&key) {
            node.children = child_keys;
        }
        key
    }

    /// The id of the node of a peer, which is made (and followed) when the
    /// peer has none.
    fn node_for(&self, peer: &Ref<AutomationPeer>, key: usize) -> u32 {
        if let Some(node) = self.nodes.borrow().get(&key) {
            return node.id;
        }
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        let weak = self.this.clone();
        let children_changed = peer.children_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.structure_dirty.borrow_mut().insert(key);
                this.request();
            }
        });
        let weak = self.this.clone();
        let property_changed = peer.property_changed(move |_| {
            if let Some(this) = weak.upgrade() {
                this.attributes_dirty.borrow_mut().insert(key);
                this.request();
            }
        });
        self.nodes.borrow_mut().insert(
            key,
            Node {
                id,
                peer: peer.clone(),
                parent: 0,
                index: 0,
                children: Vec::new(),
                sent: None,
                absolute: Rect::default(),
                subscriptions: vec![children_changed, property_changed],
            },
        );
        self.ids.borrow_mut().insert(id, key);
        id
    }

    /// The root has a node without an element: it is followed for its
    /// children and keeps the top elements in order.
    fn ensure_root_node(&self, root: &Ref<AutomationPeer>, children: Vec<usize>) {
        let key = peer_key(root);
        if !self.nodes.borrow().contains_key(&key) {
            let weak = self.this.clone();
            let children_changed = root.children_changed(move || {
                if let Some(this) = weak.upgrade() {
                    this.structure_dirty.borrow_mut().insert(key);
                    this.request();
                }
            });
            self.nodes.borrow_mut().insert(
                key,
                Node {
                    id: 0,
                    peer: root.clone(),
                    parent: 0,
                    index: 0,
                    children: Vec::new(),
                    sent: Some(NodeState::default()),
                    absolute: Rect::default(),
                    subscriptions: vec![children_changed],
                },
            );
        }
        if let Some(node) = self.nodes.borrow_mut().get_mut(&key) {
            node.children = children;
        }
    }

    /// The keys of a subtree, without its root.
    fn collect_subtree(&self, key: usize, out: &mut Vec<usize>) {
        let children = self.nodes.borrow().get(&key).map(|node| node.children.clone()).unwrap_or_default();
        for child in children {
            out.push(child);
            self.collect_subtree(child, out);
        }
    }

    /// The keys of the tree, parents before children.
    fn tree_order(&self, key: usize, out: &mut Vec<usize>) {
        out.push(key);
        let children = self.nodes.borrow().get(&key).map(|node| node.children.clone()).unwrap_or_default();
        for child in children {
            self.tree_order(child, out);
        }
    }

    /// An action of the page on the element of a peer: a click, what a
    /// screen reader does to activate it.
    pub(crate) fn on_action(&self, id: u32, action: &str) {
        if action == "enable" {
            self.set_enabled(true);
            return;
        }
        if action != "click" {
            return;
        }
        let peer = {
            let ids = self.ids.borrow();
            let nodes = self.nodes.borrow();
            ids.get(&id).and_then(|key| nodes.get(key)).map(|node| node.peer.clone())
        };
        let Some(peer) = peer else {
            return;
        };
        if !peer.is_enabled() {
            return;
        }
        if let Some(invoke) = peer.get_provider::<dyn IInvokeProvider>() {
            let _ = invoke.invoke();
        } else if let Some(toggle) = peer.get_provider::<dyn IToggleProvider>() {
            let _ = toggle.toggle();
        } else if let Some(selection) = peer.get_provider::<dyn ISelectionItemProvider>() {
            let _ = selection.select();
        } else if let Some(expand) = peer.get_provider::<dyn IExpandCollapseProvider>() {
            let _ = match expand.expand_collapse_state() {
                ExpandCollapseState::Collapsed => expand.expand(),
                _ => expand.collapse(),
            };
        } else if peer.is_keyboard_focusable() {
            peer.set_focus();
        }
    }

    /// The number of peers with an element in the page.
    pub fn element_count(&self) -> usize {
        self.nodes.borrow().values().filter(|node| node.id != 0).count()
    }
}

impl Drop for BrowserAccessibility {
    fn drop(&mut self) {
        TREES.with(|trees| {
            if let Ok(mut trees) = trees.try_borrow_mut() {
                trees.remove(&self.top_level_id);
            }
        });
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        for (_, node) in self.nodes.get_mut().drain() {
            for subscription in node.subscriptions {
                subscription.dispose();
            }
        }
    }
}

#[cfg(test)]
#[path = "browser_accessibility_tests.rs"]
mod tests;
