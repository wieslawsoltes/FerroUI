//! What a node of the accessibility tree of the system says: the values
//! the access helper and the node info providers set, kept as plain data
//! and written to the `AccessibilityNodeInfo` of the system in one place
//! (`ferro_access_helper.rs`). It stands for the compatibility wrapper of
//! a node the reference fills, and it is what the tests of the crate read.
//! The constants are those of the Android framework.

/// `AccessibilityNodeInfo.ACTION_FOCUS`.
pub const ACTION_FOCUS: i32 = 1;
/// `AccessibilityNodeInfo.ACTION_CLEAR_FOCUS`.
pub const ACTION_CLEAR_FOCUS: i32 = 2;
/// `AccessibilityNodeInfo.ACTION_SELECT`.
pub const ACTION_SELECT: i32 = 4;
/// `AccessibilityNodeInfo.ACTION_CLICK`.
pub const ACTION_CLICK: i32 = 16;
/// `AccessibilityNodeInfo.ACTION_ACCESSIBILITY_FOCUS`.
pub const ACTION_ACCESSIBILITY_FOCUS: i32 = 64;
/// `AccessibilityNodeInfo.ACTION_CLEAR_ACCESSIBILITY_FOCUS`.
pub const ACTION_CLEAR_ACCESSIBILITY_FOCUS: i32 = 128;
/// `AccessibilityNodeInfo.ACTION_SCROLL_FORWARD`.
pub const ACTION_SCROLL_FORWARD: i32 = 4096;
/// `AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD`.
pub const ACTION_SCROLL_BACKWARD: i32 = 8192;
/// `AccessibilityNodeInfo.ACTION_EXPAND`.
pub const ACTION_EXPAND: i32 = 262144;
/// `AccessibilityNodeInfo.ACTION_COLLAPSE`.
pub const ACTION_COLLAPSE: i32 = 524288;
/// `AccessibilityNodeInfo.ACTION_SET_TEXT`.
pub const ACTION_SET_TEXT: i32 = 2097152;

/// `AccessibilityNodeInfo.FOCUS_INPUT`.
pub const FOCUS_INPUT: i32 = 1;
/// `AccessibilityNodeInfo.FOCUS_ACCESSIBILITY`.
pub const FOCUS_ACCESSIBILITY: i32 = 2;

/// `AccessibilityNodeInfo.RangeInfo.RANGE_TYPE_FLOAT`.
pub const RANGE_TYPE_FLOAT: i32 = 1;

/// `AccessibilityNodeInfo.CHECKED_STATE_FALSE`, `_TRUE` and `_PARTIAL`.
pub const CHECKED_STATE_FALSE: i32 = 0;
pub const CHECKED_STATE_TRUE: i32 = 1;
pub const CHECKED_STATE_PARTIAL: i32 = 2;

/// `AccessibilityEvent.CONTENT_CHANGE_TYPE_UNDEFINED`, `_SUBTREE`, `_TEXT`
/// and `_CONTENT_DESCRIPTION`.
pub const CONTENT_CHANGE_TYPE_UNDEFINED: i32 = 0;
pub const CONTENT_CHANGE_TYPE_SUBTREE: i32 = 1;
pub const CONTENT_CHANGE_TYPE_TEXT: i32 = 2;
pub const CONTENT_CHANGE_TYPE_CONTENT_DESCRIPTION: i32 = 4;

/// `View.ACCESSIBILITY_LIVE_REGION_NONE` and `_POLITE`.
pub const ACCESSIBILITY_LIVE_REGION_NONE: i32 = 0;
pub const ACCESSIBILITY_LIVE_REGION_POLITE: i32 = 1;

/// The class name of a node nobody gave one.
pub const DEFAULT_CLASS_NAME: &str = "android.view.View";

/// The range of a node (`AccessibilityNodeInfo.RangeInfo`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeInfo {
    pub range_type: i32,
    pub min: f32,
    pub max: f32,
    pub current: f32,
}

/// The values of a node of a virtual view.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeInfo {
    /// The virtual views that are the children of the node, in order.
    pub children: Vec<i32>,
    /// The virtual views that label the node.
    pub labeled_by: Vec<i32>,
    pub class_name: Option<String>,
    pub unique_id: Option<String>,
    pub view_id_resource_name: Option<String>,
    pub enabled: bool,
    pub screen_reader_focusable: bool,
    pub focusable: bool,
    pub accessibility_focused: bool,
    pub focused: bool,
    /// Left, top, right and bottom on the screen; `None` until set.
    pub bounds_in_screen: Option<(i32, i32, i32, i32)>,
    pub text: Option<String>,
    pub content_description: Option<String>,
    /// The actions of the node, each once, in the order they were added.
    pub actions: Vec<i32>,
    pub clickable: bool,
    pub checkable: bool,
    /// One of the checked states.
    pub checked: i32,
    pub scrollable: bool,
    pub selected: bool,
    pub editable: bool,
    pub range_info: Option<RangeInfo>,
    /// The start and the end of the selection of the text.
    pub text_selection: Option<(i32, i32)>,
    pub live_region: i32,
}

impl Default for NodeInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeInfo {
    /// A node as the system obtains it: nothing set.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            labeled_by: Vec::new(),
            class_name: None,
            unique_id: None,
            view_id_resource_name: None,
            enabled: false,
            screen_reader_focusable: false,
            focusable: false,
            accessibility_focused: false,
            focused: false,
            bounds_in_screen: None,
            text: None,
            content_description: None,
            actions: Vec::new(),
            clickable: false,
            checkable: false,
            checked: CHECKED_STATE_FALSE,
            scrollable: false,
            selected: false,
            editable: false,
            range_info: None,
            text_selection: None,
            live_region: ACCESSIBILITY_LIVE_REGION_NONE,
        }
    }

    /// Adds an action to the node; an action the node has is not added
    /// again.
    pub fn add_action(&mut self, action: i32) {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
    }

    /// Whether the node has the action.
    pub fn has_action(&self, action: i32) -> bool {
        self.actions.contains(&action)
    }

    /// The actions of the node as the bit mask the system reports.
    pub fn action_mask(&self) -> i32 {
        self.actions.iter().fold(0, |mask, action| mask | action)
    }
}
