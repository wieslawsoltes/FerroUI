//! The access helper of a view: the automation peers of the top-level as
//! virtual views of the accessibility tree of the system.
//!
//! Each peer that is reached gets a number, its virtual view, and the node
//! info providers of the provider contracts it has. The system asks for
//! the node of a virtual view, performs actions on it and asks which
//! virtual view is at a point; changes of a peer are sent as accessibility
//! events. The base class of the reference is `explore_by_touch_helper.rs`
//! here; the Java class `FerroAccessHelper` is the accessibility delegate
//! and the node provider of the view and forwards to this object.

use crate::automation::expand_collapse_node_info_provider::ExpandCollapseNodeInfoProvider;
use crate::automation::invoke_node_info_provider::InvokeNodeInfoProvider;
use crate::automation::node_info::{
    NodeInfo, CONTENT_CHANGE_TYPE_CONTENT_DESCRIPTION, CONTENT_CHANGE_TYPE_SUBTREE, CONTENT_CHANGE_TYPE_TEXT,
};
use crate::automation::range_value_node_info_provider::RangeValueNodeInfoProvider;
use crate::automation::scroll_node_info_provider::ScrollNodeInfoProvider;
use crate::automation::selection_item_node_info_provider::SelectionItemNodeInfoProvider;
use crate::automation::toggle_node_info_provider::ToggleNodeInfoProvider;
use crate::automation::value_node_info_provider::ValueNodeInfoProvider;
use crate::automation::{INodeInfoProvider, NodeActionArguments};
use crate::explore_by_touch_helper::{
    ExploreByTouchHelper, IAccessibilityHost, IExploreByTouchCallbacks, IVirtualViewOwner, INVALID_ID,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{PixelPoint, PixelRect, Point, Ref};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, ControlAutomationPeer, InteropAutomationPeer,
};
use ferroui_controls::automation::provider::{
    IEmbeddedRootProvider, IExpandCollapseProvider, IInvokeProvider, IRangeValueProvider, IScrollProvider,
    ISelectionItemProvider, IToggleProvider, IValueProvider,
};
use ferroui_controls::automation::AutomationElementIdentifiers;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// Whether a peer of the control type is a container of other elements.
pub(crate) fn is_container_type(control_type: AutomationControlType) -> bool {
    matches!(
        control_type,
        AutomationControlType::Calendar
            | AutomationControlType::ComboBoxItem
            | AutomationControlType::Custom
            | AutomationControlType::DataGrid
            | AutomationControlType::DataItem
            | AutomationControlType::Document
            | AutomationControlType::Expander
            | AutomationControlType::Group
            | AutomationControlType::List
            | AutomationControlType::ListItem
            | AutomationControlType::Menu
            | AutomationControlType::MenuBar
            | AutomationControlType::MenuItem
            | AutomationControlType::None
            | AutomationControlType::Pane
            | AutomationControlType::ScrollViewer
            | AutomationControlType::SplitButton
            | AutomationControlType::Tab
            | AutomationControlType::TabItem
            | AutomationControlType::Table
            | AutomationControlType::TitleBar
            | AutomationControlType::ToolBar
            | AutomationControlType::Tree
            | AutomationControlType::TreeItem
            | AutomationControlType::Window
    )
}

/// The bounds of a node (left, top, right, bottom): the rectangle between
/// the two corners of the bounds of its peer on the screen.
pub(crate) fn bounds_in_screen_of(top_left: PixelPoint, bottom_right: PixelPoint) -> (i32, i32, i32, i32) {
    let screen_rect = PixelRect::from_points(top_left, bottom_right);
    (screen_rect.x, screen_rect.y, screen_rect.x + screen_rect.width, screen_rect.y + screen_rect.height)
}

/// What the helper asks of the view it belongs to and of its top-level.
pub(crate) trait IAccessHelperView {
    /// A pixel of the view as a point of the client area of the top-level.
    fn point_to_client(&self, point: PixelPoint) -> Point;

    /// A point of the root element of the top-level on the screen, or
    /// `None` while the top-level has no input root.
    fn point_to_screen(&self, point: Point) -> Option<PixelPoint>;
}

/// What the helper keeps for a peer it gave a virtual view.
struct PeerRegistration {
    node_info_providers: Vec<Box<dyn INodeInfoProvider>>,
    /// The handlers on the events of the peer and of its control.
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}

impl PeerRegistration {
    fn dispose(&self) {
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
    }
}

fn peer_key(peer: &AutomationPeer) -> usize {
    peer as *const AutomationPeer as usize
}

pub(crate) struct FerroAccessHelper {
    this: Weak<FerroAccessHelper>,
    base: ExploreByTouchHelper,

    peers: RefCell<HashMap<i32, Ref<AutomationPeer>>>,
    /// The virtual view of a peer, by the address of the peer (which
    /// `peers` keeps alive).
    peer_ids: RefCell<HashMap<usize, i32>>,

    peer_node_info_providers: RefCell<HashMap<usize, Rc<PeerRegistration>>>,

    /// Virtual view IDs must be allocated from a monotonic counter rather
    /// than derived from the size of `peer_node_info_providers`: entries
    /// are removed when their owner leaves the visual tree, so the count of
    /// the map does not grow monotonically and reusing it would hand out an
    /// ID that is still in use.
    next_peer_view_id: Cell<i32>,

    view: Rc<dyn IAccessHelperView>,
}

impl FerroAccessHelper {
    /// The helper of a view: `root_peer` is the peer of the top-level of
    /// the view, which gets the virtual view 0.
    pub(crate) fn new(
        view: Rc<dyn IAccessHelperView>,
        host: Rc<dyn IAccessibilityHost>,
        root_peer: Ref<AutomationPeer>,
    ) -> Rc<Self> {
        let this = Rc::new_cyclic(|this: &Weak<Self>| Self {
            this: this.clone(),
            base: ExploreByTouchHelper::new(host),
            peers: RefCell::new(HashMap::new()),
            peer_ids: RefCell::new(HashMap::new()),
            peer_node_info_providers: RefCell::new(HashMap::new()),
            next_peer_view_id: Cell::new(0),
            view,
        });

        this.get_or_create_node_info_providers_from_peer(&root_peer);

        this
    }

    fn get_node_info_providers_from_virtual_view_id(&self, virtual_view_id: i32) -> Option<Rc<PeerRegistration>> {
        let peer = self.peers.borrow().get(&virtual_view_id).cloned()?;
        self.peer_node_info_providers.borrow().get(&peer_key(&peer)).cloned()
    }

    fn is_interop_peer(peer: &Ref<AutomationPeer>) -> bool {
        peer.clone().cast::<InteropAutomationPeer>().is_some()
    }

    /// The virtual view of a peer the helper knows.
    fn peer_id(&self, peer: &AutomationPeer) -> Option<i32> {
        self.peer_ids.borrow().get(&peer_key(peer)).copied()
    }

    /// The virtual view of a peer, which is given one when it has none.
    pub(crate) fn get_or_create_node_info_providers_from_peer(&self, peer: &Ref<AutomationPeer>) -> i32 {
        if let Some(peer_view_id) = self.peer_id(peer) {
            if self.peer_node_info_providers.borrow().contains_key(&peer_key(peer)) {
                return peer_view_id;
            }
        }

        let key = peer_key(peer);
        let peer_view_id = self.next_peer_view_id.get();
        self.next_peer_view_id.set(peer_view_id + 1);
        self.peers.borrow_mut().insert(peer_view_id, peer.clone());
        self.peer_ids.borrow_mut().insert(key, peer_view_id);

        let mut subscriptions: Vec<Rc<dyn IDisposable>> = Vec::new();

        let weak = self.this.clone();
        subscriptions.push(peer.children_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.base.invalidate_virtual_view_with(peer_view_id, CONTENT_CHANGE_TYPE_SUBTREE);
            }
        }));
        let weak = self.this.clone();
        subscriptions.push(peer.property_changed(move |ev| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            if ev.property() == AutomationElementIdentifiers::name_property() {
                this.base.invalidate_virtual_view_with(peer_view_id, CONTENT_CHANGE_TYPE_TEXT);
            } else if ev.property() == AutomationElementIdentifiers::help_text_property() {
                this.base.invalidate_virtual_view_with(peer_view_id, CONTENT_CHANGE_TYPE_CONTENT_DESCRIPTION);
            } else if ev.property() == AutomationElementIdentifiers::bounding_rectangle_property()
                || ev.property() == AutomationElementIdentifiers::class_name_property()
            {
                this.base.invalidate_virtual_view(peer_view_id);
            }
        }));

        // Drop the registration once the peer's control leaves the visual tree, otherwise
        // every control ever explored by accessibility is kept alive for the lifetime of
        // the view: the peer holds a strong reference to its owner, and these three
        // maps were never pruned. On a long-running app that rebuilds its UI (for
        // instance digital signage swapping screens), this retains each dead visual tree in
        // full.
        // The root peer (ID 0) is deliberately never unregistered: the virtual view at a
        // point and the visible virtual views index the peer 0 directly.
        // It is a single entry owned by the view itself, and dies with the helper.
        if peer_view_id != 0 {
            if let Some(control_peer) = peer.clone().cast::<ControlAutomationPeer>() {
                if let Some(owner) = control_peer.try_owner() {
                    let weak = self.this.clone();
                    subscriptions.push(owner.detached_from_visual_tree(move |_| {
                        let Some(this) = weak.upgrade() else {
                            return;
                        };
                        this.peers.borrow_mut().remove(&peer_view_id);
                        this.peer_ids.borrow_mut().remove(&key);
                        let registration = this.peer_node_info_providers.borrow_mut().remove(&key);
                        if let Some(registration) = registration {
                            registration.dispose();
                        }
                    }));
                }
            }
        }

        let owner: Weak<dyn IVirtualViewOwner> = self.this.clone();
        let mut node_info_providers: Vec<Box<dyn INodeInfoProvider>> = Vec::new();
        if peer.get_provider::<dyn IExpandCollapseProvider>().is_some() {
            node_info_providers.push(Box::new(ExpandCollapseNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }
        if peer.get_provider::<dyn IInvokeProvider>().is_some() {
            node_info_providers.push(Box::new(InvokeNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }
        if peer.get_provider::<dyn IRangeValueProvider>().is_some() {
            node_info_providers.push(Box::new(RangeValueNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }
        if peer.get_provider::<dyn IScrollProvider>().is_some() {
            node_info_providers.push(Box::new(ScrollNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }
        if peer.get_provider::<dyn ISelectionItemProvider>().is_some() {
            node_info_providers.push(Box::new(SelectionItemNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }
        if peer.get_provider::<dyn IToggleProvider>().is_some() {
            node_info_providers.push(Box::new(ToggleNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }
        if peer.get_provider::<dyn IValueProvider>().is_some() {
            node_info_providers.push(Box::new(ValueNodeInfoProvider::new(owner.clone(), peer.clone(), peer_view_id)));
        }

        self.peer_node_info_providers.borrow_mut().insert(
            key,
            Rc::new(PeerRegistration { node_info_providers, subscriptions: RefCell::new(subscriptions) }),
        );

        peer_view_id
    }

    fn root_peer(&self) -> Option<Ref<AutomationPeer>> {
        self.peers.borrow().get(&0).cloned()
    }

    fn try_perform_node_action(
        node_info_provider: &dyn INodeInfoProvider,
        action: i32,
        arguments: Option<&NodeActionArguments>,
    ) -> bool {
        // An element that is not enabled, an operation that is not valid and one that is
        // not supported are all "not performed".
        node_info_provider.perform_node_action(action, arguments).unwrap_or(false)
    }

    // ---- what the node provider and the view of the Java layer call -----------------------

    /// The node of a virtual view.
    pub(crate) fn create_node_for_virtual_view(&self, virtual_view_id: i32) -> NodeInfo {
        self.base.create_node_for_child(self, virtual_view_id)
    }

    /// An action of a virtual view; whether it was performed.
    pub(crate) fn perform_action(&self, virtual_view_id: i32, action: i32, arguments: Option<&NodeActionArguments>) -> bool {
        self.base.perform_action_for_child(self, virtual_view_id, action, arguments)
    }

    /// The virtual view that has the focus of the type, or [`INVALID_ID`].
    pub(crate) fn find_focus(&self, focus_type: i32) -> i32 {
        self.base.find_focus(focus_type)
    }

    /// A hover event of the view; whether the helper took it.
    pub(crate) fn dispatch_hover_event(&self, action: i32, x: f32, y: f32) -> bool {
        self.base.dispatch_hover_event(self, action, x, y)
    }

    /// A key event of the view that the framework did not handle; whether
    /// the helper took it.
    pub(crate) fn dispatch_key_event(&self, action: i32, key_code: i32, has_no_modifiers: bool, repeat_count: i32) -> bool {
        self.base.dispatch_key_event(self, action, key_code, has_no_modifiers, repeat_count)
    }

    /// The focus of the view changed.
    pub(crate) fn on_focus_changed(&self, gain_focus: bool) {
        self.base.on_focus_changed(gain_focus);
    }

    /// The base of the helper: the focus and hover state of its virtual
    /// views.
    #[cfg(test)]
    pub(crate) fn base(&self) -> &ExploreByTouchHelper {
        &self.base
    }
}

impl IVirtualViewOwner for FerroAccessHelper {
    fn invalidate_virtual_view(&self, virtual_view_id: i32) {
        self.base.invalidate_virtual_view(virtual_view_id);
    }

    fn invalidate_virtual_view_with(&self, virtual_view_id: i32, change_types: i32) {
        self.base.invalidate_virtual_view_with(virtual_view_id, change_types);
    }
}

impl IExploreByTouchCallbacks for FerroAccessHelper {
    fn get_virtual_view_at(&self, x: f32, y: f32) -> i32 {
        let p = self.view.point_to_client(PixelPoint::new(x as i32, y as i32));
        let embedded_root_provider =
            self.root_peer().and_then(|root| root.get_provider::<dyn IEmbeddedRootProvider>());
        let peer = embedded_root_provider.as_ref().and_then(|provider| provider.get_peer_from_point(p));
        match peer {
            Some(peer) => {
                if Self::is_interop_peer(&peer) {
                    return INVALID_ID;
                }

                let parent =
                    peer.get_parent().filter(|parent| !is_container_type(parent.get_automation_control_type()));
                let virtual_view_id = match parent {
                    Some(parent) => self.get_or_create_node_info_providers_from_peer(&parent),
                    None => self.get_or_create_node_info_providers_from_peer(&peer),
                };

                if virtual_view_id == 0 {
                    INVALID_ID
                } else {
                    virtual_view_id
                }
            }
            None => {
                let peer = embedded_root_provider.and_then(|provider| provider.get_focus());
                match peer {
                    // The reference fails for a focused peer that has no virtual view yet;
                    // the port answers with none.
                    Some(peer) if !Self::is_interop_peer(&peer) => self.peer_id(&peer).unwrap_or(INVALID_ID),
                    _ => INVALID_ID,
                }
            }
        }
    }

    fn get_visible_virtual_views(&self) -> Vec<i32> {
        let Some(root) = self.root_peer() else {
            return Vec::new();
        };

        let mut virtual_view_ids = Vec::new();
        for peer in root.get_children().iter() {
            if Self::is_interop_peer(peer) {
                continue;
            }

            virtual_view_ids.push(self.get_or_create_node_info_providers_from_peer(peer));
        }
        virtual_view_ids
    }

    fn on_perform_action_for_virtual_view(
        &self,
        virtual_view_id: i32,
        action: i32,
        arguments: Option<&NodeActionArguments>,
    ) -> bool {
        let Some(providers) = self.get_node_info_providers_from_virtual_view_id(virtual_view_id) else {
            return false;
        };

        let mut result = false;
        for provider in &providers.node_info_providers {
            result |= Self::try_perform_node_action(&**provider, action, arguments);
        }
        result
    }

    fn on_populate_node_for_virtual_view(&self, virtual_view_id: i32, node_info: &mut NodeInfo) {
        let peer = self.peers.borrow().get(&virtual_view_id).cloned();
        let Some(peer) = peer else {
            // The node must still be populated: a node whose text, content description or
            // bounds are unset is rejected.
            node_info.content_description = Some(String::new());
            node_info.enabled = false;
            node_info.focusable = false;
            node_info.screen_reader_focusable = false;
            node_info.bounds_in_screen = Some((0, 0, 0, 0));
            return;
        };

        // UI logical structure
        for child in peer.get_children().iter() {
            if Self::is_interop_peer(child) {
                continue;
            }

            let child_id = self.get_or_create_node_info_providers_from_peer(child);
            node_info.children.push(child_id);
        }

        // UI labels
        if let Some(labeled_by) = peer.get_labeled_by() {
            let labeled_by_id = self.get_or_create_node_info_providers_from_peer(&labeled_by);
            node_info.labeled_by.push(labeled_by_id);
        }

        // UI debug metadata
        node_info.class_name = Some(peer.get_class_name());
        let automation_id = peer.get_automation_id();
        node_info.unique_id = automation_id.clone();
        node_info.view_id_resource_name = automation_id;

        // Common control state
        node_info.enabled = peer.is_enabled();

        // Control focus state
        let can_focus_at_all = peer.is_control_element() && !peer.is_offscreen();
        node_info.screen_reader_focusable = can_focus_at_all;
        node_info.focusable = can_focus_at_all && peer.is_keyboard_focusable();

        node_info.accessibility_focused = peer.has_keyboard_focus();
        node_info.focused = peer.has_keyboard_focus();

        // On-screen bounds
        let bounds = peer.get_bounding_rectangle();
        node_info.bounds_in_screen = Some(bounds_in_screen_of(
            self.view.point_to_screen(bounds.top_left()).unwrap_or_default(),
            self.view.point_to_screen(bounds.bottom_right()).unwrap_or_default(),
        ));

        // UI provider specifics
        let providers = self.peer_node_info_providers.borrow().get(&peer_key(&peer)).cloned();
        if let Some(providers) = providers {
            for node_info_provider in &providers.node_info_providers {
                // A provider whose peer lost its contract says nothing, where the failure
                // of the reference would leave a method the system called.
                let _ = node_info_provider.populate_node_info(node_info);
            }
        }

        // Control text contents
        if node_info.text.is_none() {
            node_info.text = Some(peer.get_name());
        }
        if node_info.content_description.is_none() {
            node_info.content_description = Some(peer.get_help_text());
        }
    }
}

#[cfg(target_os = "android")]
pub(crate) use jni::{populate_host_node, write_node_info, JavaAccessibilityHost, TopLevelAccessView};

#[cfg(target_os = "android")]
mod jni {
    use super::IAccessHelperView;
    use crate::automation::node_info::NodeInfo;
    use crate::explore_by_touch_helper::IAccessibilityHost;
    use crate::interop::java::{
        call_boolean, call_static_object, call_void, new_object, JavaClass, JavaObject, JavaRef, JavaValue,
    };
    use crate::interop::natives::sdk_int;
    use crate::platform::skia_platform::TopLevelImpl;
    use ferroui_base::{PixelPoint, Point};
    use ferroui_controls::platform::ITopLevelImpl;
    use std::rc::Weak;

    /// The top-level of a view as the helper asks it.
    pub(crate) struct TopLevelAccessView {
        top_level_impl: Weak<TopLevelImpl>,
    }

    impl TopLevelAccessView {
        pub(crate) fn new(top_level_impl: Weak<TopLevelImpl>) -> Self {
            Self { top_level_impl }
        }
    }

    impl IAccessHelperView for TopLevelAccessView {
        fn point_to_client(&self, point: PixelPoint) -> Point {
            match self.top_level_impl.upgrade() {
                Some(top_level_impl) => ITopLevelImpl::point_to_client(&*top_level_impl, point),
                None => Point::default(),
            }
        }

        fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
            let root = self.top_level_impl.upgrade()?.input_root()?.try_root_element()?;
            Some(root.point_to_screen(point))
        }
    }

    /// The Java class `FerroAccessHelper` as the host of the helper.
    pub(crate) struct JavaAccessibilityHost {
        java: JavaObject,
    }

    impl JavaAccessibilityHost {
        pub(crate) fn new(java: JavaObject) -> Self {
            Self { java }
        }
    }

    impl IAccessibilityHost for JavaAccessibilityHost {
        fn is_accessibility_enabled(&self) -> bool {
            call_boolean(&self.java, "isAccessibilityEnabled", "()Z", &[])
        }

        fn is_touch_exploration_enabled(&self) -> bool {
            call_boolean(&self.java, "isTouchExplorationEnabled", "()Z", &[])
        }

        fn invalidate(&self) {
            call_void(&self.java, "invalidateHost", "()V", &[]);
        }

        fn is_focused_or_request_focus(&self) -> bool {
            call_boolean(&self.java, "isHostFocusedOrRequestFocus", "()Z", &[])
        }

        fn send_event_for_virtual_view(&self, virtual_view_id: i32, event_type: i32) -> bool {
            call_boolean(
                &self.java,
                "sendEventForVirtualView",
                "(II)Z",
                &[JavaValue::Int(virtual_view_id), JavaValue::Int(event_type)],
            )
        }

        fn send_content_changed(&self, virtual_view_id: i32, change_types: i32) {
            call_void(
                &self.java,
                "sendContentChanged",
                "(II)V",
                &[JavaValue::Int(virtual_view_id), JavaValue::Int(change_types)],
            );
        }
    }

    fn set_boolean(info: &JavaObject, name: &str, value: bool) {
        call_void(info, name, "(Z)V", &[JavaValue::Boolean(value)]);
    }

    fn set_text(info: &JavaObject, name: &str, value: Option<&str>) {
        match value {
            Some(value) => call_void(info, name, "(Ljava/lang/CharSequence;)V", &[JavaValue::String(value)]),
            None => call_void(info, name, "(Ljava/lang/CharSequence;)V", &[JavaValue::Object(None)]),
        }
    }

    /// Adds the virtual views that are the children of the host view to
    /// the node of the host view.
    pub(crate) fn populate_host_node(children: &[i32], info: &JavaObject, host: &JavaObject) {
        for child in children {
            call_void(
                info,
                "addChild",
                "(Landroid/view/View;I)V",
                &[JavaValue::Object(Some(host)), JavaValue::Int(*child)],
            );
        }
    }

    /// Writes the values of a node to the node of the system. `host` is
    /// the view the virtual views belong to.
    pub(crate) fn write_node_info(node: &NodeInfo, info: &JavaObject, host: &JavaObject) {
        for child in &node.children {
            call_void(
                info,
                "addChild",
                "(Landroid/view/View;I)V",
                &[JavaValue::Object(Some(host)), JavaValue::Int(*child)],
            );
        }
        for labeled_by in &node.labeled_by {
            call_void(
                info,
                "setLabeledBy",
                "(Landroid/view/View;I)V",
                &[JavaValue::Object(Some(host)), JavaValue::Int(*labeled_by)],
            );
        }

        set_text(info, "setClassName", node.class_name.as_deref());
        if sdk_int() >= 33 {
            match node.unique_id.as_deref() {
                Some(unique_id) => {
                    call_void(info, "setUniqueId", "(Ljava/lang/String;)V", &[JavaValue::String(unique_id)])
                }
                None => call_void(info, "setUniqueId", "(Ljava/lang/String;)V", &[JavaValue::Object(None)]),
            }
        }
        match node.view_id_resource_name.as_deref() {
            Some(name) => {
                call_void(info, "setViewIdResourceName", "(Ljava/lang/String;)V", &[JavaValue::String(name)])
            }
            None => call_void(info, "setViewIdResourceName", "(Ljava/lang/String;)V", &[JavaValue::Object(None)]),
        }

        set_boolean(info, "setEnabled", node.enabled);
        if sdk_int() >= 28 {
            set_boolean(info, "setScreenReaderFocusable", node.screen_reader_focusable);
        }
        set_boolean(info, "setFocusable", node.focusable);
        set_boolean(info, "setAccessibilityFocused", node.accessibility_focused);
        set_boolean(info, "setFocused", node.focused);

        if let Some((left, top, right, bottom)) = node.bounds_in_screen {
            let rect = new_object(
                &JavaClass::find("android/graphics/Rect"),
                "(IIII)V",
                &[JavaValue::Int(left), JavaValue::Int(top), JavaValue::Int(right), JavaValue::Int(bottom)],
            );
            let rect: &dyn JavaRef = &rect;
            call_void(info, "setBoundsInScreen", "(Landroid/graphics/Rect;)V", &[JavaValue::Object(Some(rect))]);
        }

        set_text(info, "setText", node.text.as_deref());
        set_text(info, "setContentDescription", node.content_description.as_deref());

        for action in &node.actions {
            call_void(info, "addAction", "(I)V", &[JavaValue::Int(*action)]);
        }

        set_boolean(info, "setClickable", node.clickable);
        set_boolean(info, "setCheckable", node.checkable);
        if sdk_int() >= 36 {
            call_void(info, "setChecked", "(I)V", &[JavaValue::Int(node.checked)]);
        } else {
            set_boolean(info, "setChecked", node.checked == crate::automation::node_info::CHECKED_STATE_TRUE);
        }
        set_boolean(info, "setScrollable", node.scrollable);
        set_boolean(info, "setSelected", node.selected);
        set_boolean(info, "setEditable", node.editable);

        if let Some(range_info) = node.range_info {
            let range = call_static_object(
                &JavaClass::find("android/view/accessibility/AccessibilityNodeInfo$RangeInfo"),
                "obtain",
                "(IFFF)Landroid/view/accessibility/AccessibilityNodeInfo$RangeInfo;",
                &[
                    JavaValue::Int(range_info.range_type),
                    JavaValue::Float(range_info.min),
                    JavaValue::Float(range_info.max),
                    JavaValue::Float(range_info.current),
                ],
            );
            if let Some(range) = range {
                let range: &dyn JavaRef = &range;
                call_void(
                    info,
                    "setRangeInfo",
                    "(Landroid/view/accessibility/AccessibilityNodeInfo$RangeInfo;)V",
                    &[JavaValue::Object(Some(range))],
                );
            }
        }
        if let Some((start, end)) = node.text_selection {
            call_void(info, "setTextSelection", "(II)V", &[JavaValue::Int(start), JavaValue::Int(end)]);
        }
        call_void(info, "setLiveRegion", "(I)V", &[JavaValue::Int(node.live_region)]);
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use crate::automation::node_info::*;
    use crate::explore_by_touch_helper::*;
    use ferroui_base::threading::Dispatcher;
    use ferroui_base::StyledElement;
    use ferroui_controls::automation::{AutomationProperties, IsOffscreenBehavior};
    use ferroui_controls::primitives::RangeBase;
    use ferroui_controls::{Button, CheckBox, Control, Expander, Slider, StackPanel, TextBox};

    /// The view: its pixels are the points of the client area times the
    /// scaling.
    struct TestView {
        scaling: f64,
    }

    impl IAccessHelperView for TestView {
        fn point_to_client(&self, point: PixelPoint) -> Point {
            point.to_point(self.scaling)
        }

        fn point_to_screen(&self, point: Point) -> Option<PixelPoint> {
            Some(PixelPoint::from_point(point, self.scaling))
        }
    }

    /// The host: what was sent, and what the system says of accessibility.
    #[derive(Default)]
    struct TestHost {
        enabled: Cell<bool>,
        touch_exploration: Cell<bool>,
        focusable: Cell<bool>,
        invalidated: Cell<u32>,
        events: RefCell<Vec<(i32, i32)>>,
        content_changes: RefCell<Vec<(i32, i32)>>,
    }

    impl IAccessibilityHost for TestHost {
        fn is_accessibility_enabled(&self) -> bool {
            self.enabled.get()
        }

        fn is_touch_exploration_enabled(&self) -> bool {
            self.touch_exploration.get()
        }

        fn invalidate(&self) {
            self.invalidated.set(self.invalidated.get() + 1);
        }

        fn is_focused_or_request_focus(&self) -> bool {
            self.focusable.get()
        }

        fn send_event_for_virtual_view(&self, virtual_view_id: i32, event_type: i32) -> bool {
            self.events.borrow_mut().push((virtual_view_id, event_type));
            true
        }

        fn send_content_changed(&self, virtual_view_id: i32, change_types: i32) {
            self.content_changes.borrow_mut().push((virtual_view_id, change_types));
        }
    }

    fn mark_onscreen(element: &StyledElement, name: &str) {
        AutomationProperties::set_is_offscreen_behavior(element, IsOffscreenBehavior::Onscreen);
        if !name.is_empty() {
            AutomationProperties::set_name(element, Some(name));
        }
    }

    macro_rules! onscreen {
        ($control:expr, $name:expr) => {{
            let control = $control;
            mark_onscreen(&control, $name);
            control
        }};
    }

    struct Fixture {
        helper: Rc<FerroAccessHelper>,
        host: Rc<TestHost>,
    }

    fn helper_of(root: &Control) -> Fixture {
        let host = Rc::new(TestHost::default());
        host.enabled.set(true);
        host.touch_exploration.set(true);
        host.focusable.set(true);
        let helper = FerroAccessHelper::new(
            Rc::new(TestView { scaling: 2.0 }),
            host.clone(),
            ControlAutomationPeer::create_peer_for_element(root),
        );
        Fixture { helper, host }
    }

    fn id_of(fixture: &Fixture, control: &Control) -> i32 {
        fixture.helper.get_or_create_node_info_providers_from_peer(&ControlAutomationPeer::create_peer_for_element(control))
    }

    #[test]
    fn the_container_types_are_the_ones_of_the_reference() {
        for control_type in [
            AutomationControlType::None,
            AutomationControlType::Custom,
            AutomationControlType::Group,
            AutomationControlType::List,
            AutomationControlType::ListItem,
            AutomationControlType::Pane,
            AutomationControlType::Window,
        ] {
            assert!(is_container_type(control_type), "{control_type:?}");
        }
        for control_type in [
            AutomationControlType::Button,
            AutomationControlType::CheckBox,
            AutomationControlType::Edit,
            AutomationControlType::Slider,
            AutomationControlType::Text,
        ] {
            assert!(!is_container_type(control_type), "{control_type:?}");
        }
    }

    #[test]
    fn the_bounds_are_left_top_right_bottom_on_the_screen() {
        assert_eq!((10, 20, 40, 60), bounds_in_screen_of(PixelPoint::new(10, 20), PixelPoint::new(40, 60)));
        assert_eq!((0, 0, 0, 0), bounds_in_screen_of(PixelPoint::default(), PixelPoint::default()));
    }

    #[test]
    fn the_children_of_the_root_are_the_virtual_views_of_the_host_in_their_order() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let button = onscreen!(Button::new(), "Press");
        let text_box = onscreen!(TextBox::new(), "Name");
        let check_box = onscreen!(CheckBox::new(), "Agree");
        let slider = onscreen!(Slider::new(), "Volume");
        panel.children().add(button.clone());
        panel.children().add(text_box.clone());
        panel.children().add(check_box.clone());
        panel.children().add(slider.clone());

        let fixture = helper_of(&panel);
        // The root peer is the virtual view 0; its children follow in their order.
        assert_eq!(vec![1, 2, 3, 4], fixture.helper.get_visible_virtual_views());
        assert_eq!(vec![1, 2, 3, 4], fixture.helper.get_visible_virtual_views());
        assert_eq!(1, id_of(&fixture, &button));
        assert_eq!(4, id_of(&fixture, &slider));

        // A number is not given twice: a peer met later gets the next one.
        let late = onscreen!(Button::new(), "Late");
        panel.children().add(late.clone());
        assert_eq!(vec![1, 2, 3, 4, 5], fixture.helper.get_visible_virtual_views());
    }

    #[test]
    fn the_node_of_a_button_has_its_class_text_bounds_and_the_click_action() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let button = onscreen!(Button::new(), "Press");
        AutomationProperties::set_help_text(&button, Some("Presses"));
        AutomationProperties::set_automation_id(&button, Some("press-button"));
        panel.children().add(button.clone());
        let clicks = Rc::new(Cell::new(0));
        let counter = clicks.clone();
        button.click(move |_, _| counter.set(counter.get() + 1));

        let fixture = helper_of(&panel);
        let id = id_of(&fixture, &button);
        let node = fixture.helper.create_node_for_virtual_view(id);
        assert_eq!(Some("Button".to_string()), node.class_name);
        assert_eq!(Some("Press".to_string()), node.text);
        assert_eq!(Some("Presses".to_string()), node.content_description);
        assert_eq!(Some("press-button".to_string()), node.unique_id);
        assert_eq!(Some("press-button".to_string()), node.view_id_resource_name);
        assert!(node.enabled && node.clickable && node.screen_reader_focusable);
        assert!(!node.checkable && !node.scrollable && !node.editable && !node.selected);
        assert_eq!(Some((0, 0, 0, 0)), node.bounds_in_screen);
        assert_eq!(vec![ACTION_CLICK, ACTION_ACCESSIBILITY_FOCUS, ACTION_FOCUS], node.actions);
        assert_eq!(ACTION_CLICK | ACTION_ACCESSIBILITY_FOCUS | ACTION_FOCUS, node.action_mask());
        assert!(!node.accessibility_focused && !node.focused);

        assert!(fixture.helper.perform_action(id, ACTION_CLICK, None));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, clicks.get());
        // An action the node does not have is not performed.
        assert!(!fixture.helper.perform_action(id, ACTION_SELECT, None));

        // A button that is not enabled refuses the click, which is "not performed".
        button.set_is_enabled(false);
        assert!(!fixture.helper.create_node_for_virtual_view(id).enabled);
        assert!(!fixture.helper.perform_action(id, ACTION_CLICK, None));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, clicks.get());

        // A virtual view nobody has is a node that says nothing, and no action.
        let unknown = fixture.helper.create_node_for_virtual_view(1000);
        assert_eq!(Some(String::new()), unknown.content_description);
        assert_eq!(Some((0, 0, 0, 0)), unknown.bounds_in_screen);
        assert!(!unknown.enabled && !unknown.focusable);
        assert!(!fixture.helper.perform_action(1000, ACTION_CLICK, None));
    }

    #[test]
    fn a_check_box_is_checkable_and_a_click_toggles_it() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let check_box = onscreen!(CheckBox::new(), "Agree");
        check_box.set_is_three_state(true);
        panel.children().add(check_box.clone());

        let fixture = helper_of(&panel);
        let id = id_of(&fixture, &check_box);
        let node = fixture.helper.create_node_for_virtual_view(id);
        assert_eq!(Some("CheckBox".to_string()), node.class_name);
        assert!(node.checkable && node.clickable);
        assert_eq!(CHECKED_STATE_FALSE, node.checked);
        assert!(node.has_action(ACTION_CLICK));

        assert!(fixture.helper.perform_action(id, ACTION_CLICK, None));
        assert_eq!(Some(true), check_box.is_checked());
        assert_eq!(CHECKED_STATE_TRUE, fixture.helper.create_node_for_virtual_view(id).checked);

        check_box.set_is_checked(None);
        assert_eq!(CHECKED_STATE_PARTIAL, fixture.helper.create_node_for_virtual_view(id).checked);
    }

    #[test]
    fn a_text_box_has_its_text_and_takes_text_that_is_set() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let text_box = onscreen!(TextBox::new(), "Name");
        text_box.set_text(Some("ab\u{1F600}"));
        panel.children().add(text_box.clone());

        let fixture = helper_of(&panel);
        let id = id_of(&fixture, &text_box);
        let node = fixture.helper.create_node_for_virtual_view(id);
        // The text of the node is the value, not the name.
        assert_eq!(Some("ab\u{1F600}".to_string()), node.text);
        assert!(node.editable);
        // The selection is at the end, counted in UTF-16 code units.
        assert_eq!(Some((4, 4)), node.text_selection);
        assert_eq!(ACCESSIBILITY_LIVE_REGION_POLITE, node.live_region);
        assert!(node.has_action(ACTION_SET_TEXT));

        // The text of the action is appended to the value, as in the reference.
        let arguments = NodeActionArguments { set_text_char_sequence: Some("cd".to_string()) };
        assert!(fixture.helper.perform_action(id, ACTION_SET_TEXT, Some(&arguments)));
        assert_eq!(Some("ab\u{1F600}cd".to_string()), text_box.text());

        // A change of the value is sent as a change of the text of the virtual view.
        assert!(fixture.host.content_changes.borrow().contains(&(id, CONTENT_CHANGE_TYPE_TEXT)));

        // A read-only text box is not editable.
        text_box.set_is_read_only(true);
        assert!(!fixture.helper.create_node_for_virtual_view(id).editable);
    }

    #[test]
    fn a_slider_has_its_range_and_no_action_of_its_own() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let slider = onscreen!(Slider::new(), "Volume");
        let range: &RangeBase = &slider;
        range.set_minimum(0.0);
        range.set_maximum(10.0);
        range.set_range_value(4.0);
        panel.children().add(slider.clone());

        let fixture = helper_of(&panel);
        let id = id_of(&fixture, &slider);
        let node = fixture.helper.create_node_for_virtual_view(id);
        assert_eq!(Some("Slider".to_string()), node.class_name);
        assert_eq!(Some("Volume".to_string()), node.text);
        assert_eq!(Some(RangeInfo { range_type: RANGE_TYPE_FLOAT, min: 0.0, max: 10.0, current: 4.0 }), node.range_info);
        // The reference gives a range no action: only the focus actions of the helper.
        assert_eq!(vec![ACTION_ACCESSIBILITY_FOCUS, ACTION_FOCUS], node.actions);
        assert!(!fixture.helper.perform_action(id, ACTION_SCROLL_FORWARD, None));
        assert_eq!(4.0, range.value());

        range.set_range_value(7.5);
        assert_eq!(7.5, fixture.helper.create_node_for_virtual_view(id).range_info.unwrap().current);
    }

    #[test]
    fn an_expander_expands_and_collapses() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let expander = onscreen!(Expander::new(), "More");
        panel.children().add(expander.clone());

        let fixture = helper_of(&panel);
        let id = id_of(&fixture, &expander);
        let node = fixture.helper.create_node_for_virtual_view(id);
        assert!(node.has_action(ACTION_EXPAND) && node.has_action(ACTION_COLLAPSE));
        assert!(fixture.helper.perform_action(id, ACTION_EXPAND, None));
        assert!(expander.is_expanded());
        assert!(fixture.helper.perform_action(id, ACTION_COLLAPSE, None));
        assert!(!expander.is_expanded());
    }

    #[test]
    fn the_children_and_the_changes_of_a_peer_are_told() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let inner = onscreen!(StackPanel::new(), "");
        let button = onscreen!(Button::new(), "Press");
        inner.children().add(button.clone());
        panel.children().add(inner.clone());

        let fixture = helper_of(&panel);
        let inner_id = id_of(&fixture, &inner);
        let node = fixture.helper.create_node_for_virtual_view(inner_id);
        let button_id = id_of(&fixture, &button);
        assert_eq!(vec![button_id], node.children);
        // A panel has an empty text and no content, and is no stop of a screen reader.
        assert_eq!(Some(String::new()), node.text);
        assert!(!node.screen_reader_focusable);

        // The name and the help text of a peer, and its children.
        fixture.host.content_changes.borrow_mut().clear();
        let second = onscreen!(Button::new(), "Second");
        inner.children().add(second);
        assert!(fixture.host.content_changes.borrow().contains(&(inner_id, CONTENT_CHANGE_TYPE_SUBTREE)));

        // Nothing is sent while accessibility is off.
        fixture.host.enabled.set(false);
        fixture.host.content_changes.borrow_mut().clear();
        inner.children().add(onscreen!(Button::new(), "Third"));
        assert!(fixture.host.content_changes.borrow().is_empty());
    }

    #[test]
    fn the_accessibility_focus_is_one_virtual_view_and_its_events_are_sent() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let first = onscreen!(Button::new(), "First");
        let second = onscreen!(Button::new(), "Second");
        panel.children().add(first.clone());
        panel.children().add(second.clone());

        let fixture = helper_of(&panel);
        let (first_id, second_id) = (id_of(&fixture, &first), id_of(&fixture, &second));
        assert_eq!(INVALID_ID, fixture.helper.find_focus(FOCUS_ACCESSIBILITY));

        assert!(fixture.helper.perform_action(first_id, ACTION_ACCESSIBILITY_FOCUS, None));
        assert_eq!(first_id, fixture.helper.find_focus(FOCUS_ACCESSIBILITY));
        assert_eq!(first_id, fixture.helper.base().accessibility_focused_virtual_view_id());
        let node = fixture.helper.create_node_for_virtual_view(first_id);
        assert!(node.accessibility_focused);
        assert!(node.has_action(ACTION_CLEAR_ACCESSIBILITY_FOCUS) && !node.has_action(ACTION_ACCESSIBILITY_FOCUS));
        // Asked again, it is not given again.
        assert!(!fixture.helper.perform_action(first_id, ACTION_ACCESSIBILITY_FOCUS, None));

        assert!(fixture.helper.perform_action(second_id, ACTION_ACCESSIBILITY_FOCUS, None));
        assert_eq!(
            vec![
                (first_id, TYPE_VIEW_ACCESSIBILITY_FOCUSED),
                (first_id, TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED),
                (second_id, TYPE_VIEW_ACCESSIBILITY_FOCUSED)
            ],
            *fixture.host.events.borrow()
        );
        assert_eq!(3, fixture.host.invalidated.get());
        assert!(!fixture.helper.perform_action(first_id, ACTION_CLEAR_ACCESSIBILITY_FOCUS, None));
        assert!(fixture.helper.perform_action(second_id, ACTION_CLEAR_ACCESSIBILITY_FOCUS, None));
        assert_eq!(INVALID_ID, fixture.helper.find_focus(FOCUS_ACCESSIBILITY));

        // Without touch exploration the focus is not given.
        fixture.host.touch_exploration.set(false);
        assert!(!fixture.helper.perform_action(first_id, ACTION_ACCESSIBILITY_FOCUS, None));
    }

    #[test]
    fn the_keyboard_focus_of_a_virtual_view_needs_the_focus_of_the_host() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let button = onscreen!(Button::new(), "Press");
        panel.children().add(button.clone());
        let clicks = Rc::new(Cell::new(0));
        let counter = clicks.clone();
        button.click(move |_, _| counter.set(counter.get() + 1));

        let fixture = helper_of(&panel);
        let id = id_of(&fixture, &button);

        fixture.host.focusable.set(false);
        assert!(!fixture.helper.perform_action(id, ACTION_FOCUS, None));
        fixture.host.focusable.set(true);
        assert!(fixture.helper.perform_action(id, ACTION_FOCUS, None));
        assert_eq!(id, fixture.helper.find_focus(FOCUS_INPUT));
        assert_eq!(id, fixture.helper.base().keyboard_focused_virtual_view_id());
        let node = fixture.helper.create_node_for_virtual_view(id);
        assert!(node.focused && node.has_action(ACTION_CLEAR_FOCUS) && !node.has_action(ACTION_FOCUS));

        // The enter key clicks the virtual view that has the keyboard focus: once per
        // press, not with a modifier, not when the key is released.
        assert!(fixture.helper.dispatch_key_event(0, KEYCODE_ENTER, true, 0));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, clicks.get());
        assert!(!fixture.helper.dispatch_key_event(0, KEYCODE_ENTER, true, 1));
        assert!(!fixture.helper.dispatch_key_event(0, KEYCODE_ENTER, false, 0));
        assert!(!fixture.helper.dispatch_key_event(KEY_ACTION_UP, KEYCODE_ENTER, true, 0));
        assert!(!fixture.helper.dispatch_key_event(0, 29, true, 0));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(1, clicks.get());

        // A change of the focus of the host takes the keyboard focus from the virtual view.
        fixture.helper.on_focus_changed(false);
        assert_eq!(INVALID_ID, fixture.helper.find_focus(FOCUS_INPUT));
        assert_eq!(vec![(id, TYPE_VIEW_FOCUSED), (id, TYPE_VIEW_FOCUSED)], *fixture.host.events.borrow());
        assert_eq!(INVALID_ID, fixture.helper.find_focus(0));
    }

    #[test]
    fn a_hover_outside_of_every_peer_is_not_taken() {
        let _scope = Dispatcher::unit_test_scope();
        let panel = onscreen!(StackPanel::new(), "");
        let fixture = helper_of(&panel);

        // A panel is no embedded root: no peer is found at a point, and none has the focus.
        assert_eq!(INVALID_ID, fixture.helper.get_virtual_view_at(10.0, 10.0));
        assert!(!fixture.helper.dispatch_hover_event(ACTION_HOVER_MOVE, 10.0, 10.0));
        assert!(!fixture.helper.dispatch_hover_event(ACTION_HOVER_EXIT, 10.0, 10.0));
        assert!(fixture.host.events.borrow().is_empty());

        // While accessibility or touch exploration is off, a hover is not looked at.
        fixture.host.touch_exploration.set(false);
        assert!(!fixture.helper.dispatch_hover_event(ACTION_HOVER_ENTER, 10.0, 10.0));
    }

    #[test]
    fn a_node_has_each_action_once() {
        let mut node = NodeInfo::new();
        node.add_action(ACTION_CLICK);
        node.add_action(ACTION_CLICK);
        node.add_action(ACTION_SELECT);
        assert_eq!(vec![ACTION_CLICK, ACTION_SELECT], node.actions);
        assert_eq!(20, node.action_mask());
        assert_eq!(None, node.class_name);
    }
}
