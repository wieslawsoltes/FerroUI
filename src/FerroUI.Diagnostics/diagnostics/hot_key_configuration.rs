//! Port of `Diagnostics/HotKeyConfiguration.cs`.

use ferroui_base::input::{Key, KeyGesture, KeyModifiers};
use std::cell::Cell;
use std::rc::Rc;

/// The key gestures that activate the features of DevTools.
///
/// A class upstream whose members are set once, when it is created (`init`): the members are
/// read with `foo()` and given with the chaining `with_foo(v)` while the configuration is built.
#[derive(Debug)]
pub struct HotKeyConfiguration {
    value_frames_freeze: Cell<KeyGesture>,
    value_frames_unfreeze: Cell<KeyGesture>,
    inspect_hovered_control: Cell<KeyGesture>,
    toggle_popup_freeze: Cell<KeyGesture>,
    screenshot_selected_control: Cell<KeyGesture>,
}

impl PartialEq for HotKeyConfiguration {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for HotKeyConfiguration {
    fn default() -> Self {
        Self {
            value_frames_freeze: Cell::new(KeyGesture::new(Key::S, KeyModifiers::ALT)),
            value_frames_unfreeze: Cell::new(KeyGesture::new(Key::D, KeyModifiers::ALT)),
            inspect_hovered_control: Cell::new(KeyGesture::new(
                Key::None,
                KeyModifiers::SHIFT.union(KeyModifiers::CONTROL),
            )),
            toggle_popup_freeze: Cell::new(KeyGesture::new(Key::F, KeyModifiers::ALT.union(KeyModifiers::CONTROL))),
            screenshot_selected_control: Cell::new(KeyGesture::from_key(Key::F8)),
        }
    }
}

impl HotKeyConfiguration {
    /// C# `new HotKeyConfiguration()`: the default gestures.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Freezes refreshing the Value Frames inspector for the selected Control
    pub fn value_frames_freeze(&self) -> KeyGesture {
        self.value_frames_freeze.get()
    }

    /// Initializes [`value_frames_freeze`](Self::value_frames_freeze).
    pub fn with_value_frames_freeze(self: Rc<Self>, value: KeyGesture) -> Rc<Self> {
        self.value_frames_freeze.set(value);
        self
    }

    /// Resumes refreshing the Value Frames inspector for the selected Control
    pub fn value_frames_unfreeze(&self) -> KeyGesture {
        self.value_frames_unfreeze.get()
    }

    /// Initializes [`value_frames_unfreeze`](Self::value_frames_unfreeze).
    pub fn with_value_frames_unfreeze(self: Rc<Self>, value: KeyGesture) -> Rc<Self> {
        self.value_frames_unfreeze.set(value);
        self
    }

    /// Inspects the hovered Control in the Logical or Visual Tree Page
    pub fn inspect_hovered_control(&self) -> KeyGesture {
        self.inspect_hovered_control.get()
    }

    /// Initializes [`inspect_hovered_control`](Self::inspect_hovered_control).
    pub fn with_inspect_hovered_control(self: Rc<Self>, value: KeyGesture) -> Rc<Self> {
        self.inspect_hovered_control.set(value);
        self
    }

    /// Toggles the freezing of Popups which prevents visible Popups from closing so they can be inspected
    pub fn toggle_popup_freeze(&self) -> KeyGesture {
        self.toggle_popup_freeze.get()
    }

    /// Initializes [`toggle_popup_freeze`](Self::toggle_popup_freeze).
    pub fn with_toggle_popup_freeze(self: Rc<Self>, value: KeyGesture) -> Rc<Self> {
        self.toggle_popup_freeze.set(value);
        self
    }

    /// Saves a Screenshot of the Selected Control in the Logical or Visual Tree Page
    pub fn screenshot_selected_control(&self) -> KeyGesture {
        self.screenshot_selected_control.get()
    }

    /// Initializes [`screenshot_selected_control`](Self::screenshot_selected_control).
    pub fn with_screenshot_selected_control(self: Rc<Self>, value: KeyGesture) -> Rc<Self> {
        self.screenshot_selected_control.set(value);
        self
    }
}
