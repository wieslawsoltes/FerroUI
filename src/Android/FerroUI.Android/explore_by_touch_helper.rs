//! What the access helper of the reference inherits from the explore by
//! touch helper of the AndroidX libraries, which the port does not use
//! (docs/porting/android-platform.md, section 7.3): the virtual view that
//! has the accessibility focus, the one that has the keyboard focus and the
//! one under the finger; the node of a virtual view around what the access
//! helper says of it; the focus actions of a node; the accessibility
//! events of a virtual view.
//!
//! The state and the rules are here, where the tests of the crate run
//! them; what is a call into the view system (the accessibility manager,
//! the events, the node of the host view) is behind
//! [`IAccessibilityHost`], answered by the Java class `FerroAccessHelper`.
//!
//! Not built: moving the keyboard focus between virtual views with the
//! direction keys and the tab key (the focus strategy of the library). The
//! keyboard focus of the framework moves between its own elements.

// Members of the class of the reference that nothing of the port calls yet are kept with it.
#![allow(dead_code)]

use crate::automation::node_info::{
    NodeInfo, ACTION_ACCESSIBILITY_FOCUS, ACTION_CLEAR_ACCESSIBILITY_FOCUS, ACTION_CLEAR_FOCUS, ACTION_CLICK,
    ACTION_FOCUS, CONTENT_CHANGE_TYPE_SUBTREE, CONTENT_CHANGE_TYPE_UNDEFINED, DEFAULT_CLASS_NAME, FOCUS_ACCESSIBILITY,
    FOCUS_INPUT,
};
use crate::automation::NodeActionArguments;
use std::cell::Cell;
use std::rc::Rc;

/// The number that stands for no virtual view.
pub const INVALID_ID: i32 = i32::MIN;
/// The number that stands for the host view itself (`View.NO_ID`).
pub const HOST_ID: i32 = -1;

/// `AccessibilityEvent.TYPE_VIEW_FOCUSED`.
pub const TYPE_VIEW_FOCUSED: i32 = 8;
/// `AccessibilityEvent.TYPE_VIEW_HOVER_ENTER`.
pub const TYPE_VIEW_HOVER_ENTER: i32 = 128;
/// `AccessibilityEvent.TYPE_VIEW_HOVER_EXIT`.
pub const TYPE_VIEW_HOVER_EXIT: i32 = 256;
/// `AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUSED`.
pub const TYPE_VIEW_ACCESSIBILITY_FOCUSED: i32 = 32768;
/// `AccessibilityEvent.TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED`.
pub const TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED: i32 = 65536;

/// `MotionEvent.ACTION_HOVER_MOVE`, `_ENTER` and `_EXIT`.
pub const ACTION_HOVER_MOVE: i32 = 7;
pub const ACTION_HOVER_ENTER: i32 = 9;
pub const ACTION_HOVER_EXIT: i32 = 10;

/// `KeyEvent.ACTION_UP`.
pub const KEY_ACTION_UP: i32 = 1;
/// `KeyEvent.KEYCODE_DPAD_CENTER` and `KEYCODE_ENTER`.
pub const KEYCODE_DPAD_CENTER: i32 = 23;
pub const KEYCODE_ENTER: i32 = 66;

/// The helper as the node info providers see it: the owner of the virtual
/// views, which is told when one of them changed.
pub(crate) trait IVirtualViewOwner {
    /// The virtual view changed in a way that is not said.
    fn invalidate_virtual_view(&self, virtual_view_id: i32);

    /// The virtual view changed; `change_types` are content change types
    /// of an accessibility event.
    fn invalidate_virtual_view_with(&self, virtual_view_id: i32, change_types: i32);
}

/// What the helper asks of the host view and of the system.
pub(crate) trait IAccessibilityHost {
    /// Whether accessibility is enabled in the system.
    fn is_accessibility_enabled(&self) -> bool;

    /// Whether touch exploration is enabled in the system.
    fn is_touch_exploration_enabled(&self) -> bool;

    /// Asks for the host view to be drawn again.
    fn invalidate(&self);

    /// Whether the host view has the focus, or took it when asked.
    fn is_focused_or_request_focus(&self) -> bool;

    /// Sends an accessibility event of the type for the virtual view
    /// through the parent of the host view; whether it was sent.
    fn send_event_for_virtual_view(&self, virtual_view_id: i32, event_type: i32) -> bool;

    /// Sends the event that the content of the virtual view changed.
    fn send_content_changed(&self, virtual_view_id: i32, change_types: i32);
}

/// What a helper answers about its virtual views (the abstract members of
/// the library class).
pub(crate) trait IExploreByTouchCallbacks {
    /// The virtual view at a point of the host view, or [`INVALID_ID`].
    fn get_virtual_view_at(&self, x: f32, y: f32) -> i32;

    /// The virtual views that are the children of the host view.
    fn get_visible_virtual_views(&self) -> Vec<i32>;

    /// Says what the node of a virtual view has.
    fn on_populate_node_for_virtual_view(&self, virtual_view_id: i32, node_info: &mut NodeInfo);

    /// Performs an action of a virtual view; whether it was performed.
    fn on_perform_action_for_virtual_view(
        &self,
        virtual_view_id: i32,
        action: i32,
        arguments: Option<&NodeActionArguments>,
    ) -> bool;
}

pub(crate) struct ExploreByTouchHelper {
    host: Rc<dyn IAccessibilityHost>,
    accessibility_focused_virtual_view_id: Cell<i32>,
    keyboard_focused_virtual_view_id: Cell<i32>,
    hovered_virtual_view_id: Cell<i32>,
}

impl ExploreByTouchHelper {
    pub(crate) fn new(host: Rc<dyn IAccessibilityHost>) -> Self {
        Self {
            host,
            accessibility_focused_virtual_view_id: Cell::new(INVALID_ID),
            keyboard_focused_virtual_view_id: Cell::new(INVALID_ID),
            hovered_virtual_view_id: Cell::new(INVALID_ID),
        }
    }

    pub(crate) fn accessibility_focused_virtual_view_id(&self) -> i32 {
        self.accessibility_focused_virtual_view_id.get()
    }

    pub(crate) fn keyboard_focused_virtual_view_id(&self) -> i32 {
        self.keyboard_focused_virtual_view_id.get()
    }

    /// A hover event of the host view: the virtual view under the finger
    /// is told to assistive technology while touch exploration is on.
    /// Whether the event was taken.
    pub(crate) fn dispatch_hover_event(&self, callbacks: &dyn IExploreByTouchCallbacks, action: i32, x: f32, y: f32) -> bool {
        if !self.host.is_accessibility_enabled() || !self.host.is_touch_exploration_enabled() {
            return false;
        }

        match action {
            ACTION_HOVER_MOVE | ACTION_HOVER_ENTER => {
                let virtual_view_id = callbacks.get_virtual_view_at(x, y);
                self.update_hovered_virtual_view(virtual_view_id);
                virtual_view_id != INVALID_ID
            }
            ACTION_HOVER_EXIT => {
                if self.hovered_virtual_view_id.get() != INVALID_ID {
                    self.update_hovered_virtual_view(INVALID_ID);
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn update_hovered_virtual_view(&self, virtual_view_id: i32) {
        if self.hovered_virtual_view_id.get() == virtual_view_id {
            return;
        }

        let previous_virtual_view_id = self.hovered_virtual_view_id.replace(virtual_view_id);

        // Stay consistent with framework behavior by sending ENTER/EXIT pairs
        // in reverse order. This is accurate as of API 18.
        self.send_event_for_virtual_view(virtual_view_id, TYPE_VIEW_HOVER_ENTER);
        self.send_event_for_virtual_view(previous_virtual_view_id, TYPE_VIEW_HOVER_EXIT);
    }

    /// A key event of the host view the framework did not handle: the
    /// enter key and the centre of the direction pad click the virtual
    /// view that has the keyboard focus. Whether the event was taken.
    pub(crate) fn dispatch_key_event(
        &self,
        callbacks: &dyn IExploreByTouchCallbacks,
        action: i32,
        key_code: i32,
        has_no_modifiers: bool,
        repeat_count: i32,
    ) -> bool {
        let mut handled = false;

        if action != KEY_ACTION_UP {
            match key_code {
                KEYCODE_DPAD_CENTER | KEYCODE_ENTER => {
                    if has_no_modifiers && repeat_count == 0 {
                        self.click_keyboard_focused_virtual_view(callbacks);
                        handled = true;
                    }
                }
                // The direction keys and the tab key move the keyboard
                // focus between virtual views in the library; not built.
                _ => {}
            }
        }

        handled
    }

    fn click_keyboard_focused_virtual_view(&self, callbacks: &dyn IExploreByTouchCallbacks) -> bool {
        let keyboard_focused = self.keyboard_focused_virtual_view_id.get();
        keyboard_focused != INVALID_ID
            && callbacks.on_perform_action_for_virtual_view(keyboard_focused, ACTION_CLICK, None)
    }

    /// The focus of the host view changed: a virtual view does not keep
    /// the keyboard focus.
    pub(crate) fn on_focus_changed(&self, _gain_focus: bool) {
        let keyboard_focused = self.keyboard_focused_virtual_view_id.get();
        if keyboard_focused != INVALID_ID {
            self.clear_keyboard_focus_for_virtual_view(keyboard_focused);
        }
        // The library moves the focus into the virtual views when the
        // host view gains it from a direction; not built.
    }

    pub(crate) fn send_event_for_virtual_view(&self, virtual_view_id: i32, event_type: i32) -> bool {
        if virtual_view_id == INVALID_ID || !self.host.is_accessibility_enabled() {
            return false;
        }

        self.host.send_event_for_virtual_view(virtual_view_id, event_type)
    }

    /// Notifies the accessibility framework that the properties of the
    /// host view have changed.
    pub(crate) fn invalidate_root(&self) {
        self.invalidate_virtual_view_with(HOST_ID, CONTENT_CHANGE_TYPE_SUBTREE);
    }

    pub(crate) fn invalidate_virtual_view(&self, virtual_view_id: i32) {
        self.invalidate_virtual_view_with(virtual_view_id, CONTENT_CHANGE_TYPE_UNDEFINED);
    }

    pub(crate) fn invalidate_virtual_view_with(&self, virtual_view_id: i32, change_types: i32) {
        if virtual_view_id != INVALID_ID && self.host.is_accessibility_enabled() {
            self.host.send_content_changed(virtual_view_id, change_types);
        }
    }

    /// The virtual view that has the focus of the type (`FOCUS_INPUT` or
    /// `FOCUS_ACCESSIBILITY`), or [`INVALID_ID`].
    pub(crate) fn find_focus(&self, focus_type: i32) -> i32 {
        match focus_type {
            FOCUS_ACCESSIBILITY => self.accessibility_focused_virtual_view_id.get(),
            FOCUS_INPUT => self.keyboard_focused_virtual_view_id.get(),
            _ => INVALID_ID,
        }
    }

    /// The node of a virtual view: what the helper says of it, and the
    /// focus state and focus actions the virtual view has here.
    ///
    /// # Panics
    /// Panics as the library throws: when the helper gave the node neither
    /// a text nor a content description, or no bounds, or one of the two
    /// accessibility focus actions.
    pub(crate) fn create_node_for_child(&self, callbacks: &dyn IExploreByTouchCallbacks, virtual_view_id: i32) -> NodeInfo {
        let mut node = NodeInfo::new();

        // Ensure the client has good defaults.
        node.enabled = true;
        node.focusable = true;
        node.class_name = Some(DEFAULT_CLASS_NAME.to_string());

        // Allow the client to populate the node.
        callbacks.on_populate_node_for_virtual_view(virtual_view_id, &mut node);

        // Make sure the developer is following the rules.
        if node.text.is_none() && node.content_description.is_none() {
            panic!("Callbacks must add text or a content description in populateNodeForVirtualViewId()");
        }

        if node.bounds_in_screen.is_none() {
            panic!("Callbacks must set parent bounds or screen bounds in populateNodeForVirtualViewId()");
        }

        if node.has_action(ACTION_ACCESSIBILITY_FOCUS) {
            panic!("Callbacks must not add ACTION_ACCESSIBILITY_FOCUS in populateNodeForVirtualViewId()");
        }
        if node.has_action(ACTION_CLEAR_ACCESSIBILITY_FOCUS) {
            panic!("Callbacks must not add ACTION_CLEAR_ACCESSIBILITY_FOCUS in populateNodeForVirtualViewId()");
        }

        // Manage internal accessibility focus state.
        if self.accessibility_focused_virtual_view_id.get() == virtual_view_id {
            node.accessibility_focused = true;
            node.add_action(ACTION_CLEAR_ACCESSIBILITY_FOCUS);
        } else {
            node.accessibility_focused = false;
            node.add_action(ACTION_ACCESSIBILITY_FOCUS);
        }

        // Manage internal keyboard focus state.
        let is_focused = self.keyboard_focused_virtual_view_id.get() == virtual_view_id;
        if is_focused {
            node.add_action(ACTION_CLEAR_FOCUS);
        } else if node.focusable {
            node.add_action(ACTION_FOCUS);
        }
        node.focused = is_focused;

        node
    }

    /// An action of a virtual view: the four focus actions are the
    /// helper's own, every other is the callbacks'.
    pub(crate) fn perform_action_for_child(
        &self,
        callbacks: &dyn IExploreByTouchCallbacks,
        virtual_view_id: i32,
        action: i32,
        arguments: Option<&NodeActionArguments>,
    ) -> bool {
        match action {
            ACTION_ACCESSIBILITY_FOCUS => self.request_accessibility_focus(virtual_view_id),
            ACTION_CLEAR_ACCESSIBILITY_FOCUS => self.clear_accessibility_focus(virtual_view_id),
            ACTION_FOCUS => self.request_keyboard_focus_for_virtual_view(virtual_view_id),
            ACTION_CLEAR_FOCUS => self.clear_keyboard_focus_for_virtual_view(virtual_view_id),
            _ => callbacks.on_perform_action_for_virtual_view(virtual_view_id, action, arguments),
        }
    }

    fn request_accessibility_focus(&self, virtual_view_id: i32) -> bool {
        if !self.host.is_accessibility_enabled() || !self.host.is_touch_exploration_enabled() {
            return false;
        }
        // TODO: Check virtual view visibility.
        let focused = self.accessibility_focused_virtual_view_id.get();
        if focused != virtual_view_id {
            // Clear focus from the previously focused view, if applicable.
            if focused != INVALID_ID {
                self.clear_accessibility_focus(focused);
            }

            // Set focus on the new view.
            self.accessibility_focused_virtual_view_id.set(virtual_view_id);

            // TODO: Only invalidate virtual view bounds.
            self.host.invalidate();
            self.send_event_for_virtual_view(virtual_view_id, TYPE_VIEW_ACCESSIBILITY_FOCUSED);
            return true;
        }
        false
    }

    fn clear_accessibility_focus(&self, virtual_view_id: i32) -> bool {
        if self.accessibility_focused_virtual_view_id.get() == virtual_view_id {
            self.accessibility_focused_virtual_view_id.set(INVALID_ID);
            self.host.invalidate();
            self.send_event_for_virtual_view(virtual_view_id, TYPE_VIEW_ACCESSIBILITY_FOCUS_CLEARED);
            return true;
        }
        false
    }

    /// Attempts to give the keyboard focus to a virtual view.
    pub(crate) fn request_keyboard_focus_for_virtual_view(&self, virtual_view_id: i32) -> bool {
        if !self.host.is_focused_or_request_focus() {
            // Host must have real keyboard focus.
            return false;
        }

        let keyboard_focused = self.keyboard_focused_virtual_view_id.get();
        if keyboard_focused == virtual_view_id {
            // The virtual view already has focus.
            return false;
        }

        if keyboard_focused != INVALID_ID {
            self.clear_keyboard_focus_for_virtual_view(keyboard_focused);
        }

        if virtual_view_id == INVALID_ID {
            return false;
        }

        self.keyboard_focused_virtual_view_id.set(virtual_view_id);

        self.send_event_for_virtual_view(virtual_view_id, TYPE_VIEW_FOCUSED);

        true
    }

    /// Attempts to clear the keyboard focus from a virtual view.
    pub(crate) fn clear_keyboard_focus_for_virtual_view(&self, virtual_view_id: i32) -> bool {
        if self.keyboard_focused_virtual_view_id.get() != virtual_view_id {
            // The virtual view is not focused.
            return false;
        }

        self.keyboard_focused_virtual_view_id.set(INVALID_ID);

        self.send_event_for_virtual_view(virtual_view_id, TYPE_VIEW_FOCUSED);

        true
    }
}
