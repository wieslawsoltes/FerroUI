//! Port of `Diagnostics/DevToolsOptions.cs`.

use super::{Conventions, DevToolsViewKind, HotKeyConfiguration, IScreenshotHandler};
use ferroui_base::input::{Key, KeyGesture};
use ferroui_base::media::IBrush;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::Size;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Describes options used to customize DevTools.
///
/// A class upstream: a shared handle (`Rc<DevToolsOptions>`) whose properties are read with
/// `foo()`, set with `set_foo(v)` and given with the chaining `with_foo(v)` while the options
/// are built; the members that upstream sets only when the object is created (`init`) have no
/// `set_foo`.
pub struct DevToolsOptions {
    gesture: Cell<KeyGesture>,
    show_as_child_window: Cell<bool>,
    size: Cell<Size>,
    startup_screen_index: Cell<Option<i32>>,
    show_implemented_interfaces: Cell<bool>,
    screenshot_handler: RefCell<Rc<dyn IScreenshotHandler>>,
    theme_variant: RefCell<Option<ThemeVariant>>,
    focus_highlighter_brush: RefCell<Option<Rc<dyn IBrush>>>,
    launch_view: Cell<DevToolsViewKind>,
    hot_keys: RefCell<Rc<HotKeyConfiguration>>,
}

impl PartialEq for DevToolsOptions {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Default for DevToolsOptions {
    fn default() -> Self {
        Self {
            gesture: Cell::new(KeyGesture::from_key(Key::F12)),
            show_as_child_window: Cell::new(true),
            size: Cell::new(Size::new(1280.0, 720.0)),
            startup_screen_index: Cell::new(None),
            show_implemented_interfaces: Cell::new(true),
            screenshot_handler: RefCell::new(Conventions::default_screenshot_handler()),
            theme_variant: RefCell::new(None),
            focus_highlighter_brush: RefCell::new(None),
            launch_view: Cell::new(DevToolsViewKind::default()),
            hot_keys: RefCell::new(HotKeyConfiguration::new()),
        }
    }
}

impl DevToolsOptions {
    /// C# `new DevToolsOptions()`: the default options.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Gets or sets the key gesture used to open DevTools.
    pub fn gesture(&self) -> KeyGesture {
        self.gesture.get()
    }

    pub fn set_gesture(&self, value: KeyGesture) {
        self.gesture.set(value);
    }

    pub fn with_gesture(self: Rc<Self>, value: KeyGesture) -> Rc<Self> {
        self.set_gesture(value);
        self
    }

    /// Gets or sets a value indicating whether DevTools should be displayed as a child window
    /// of the window being inspected. The default value is true.
    ///
    /// This setting is ignored if DevTools is attached to an application.
    pub fn show_as_child_window(&self) -> bool {
        self.show_as_child_window.get()
    }

    pub fn set_show_as_child_window(&self, value: bool) {
        self.show_as_child_window.set(value);
    }

    pub fn with_show_as_child_window(self: Rc<Self>, value: bool) -> Rc<Self> {
        self.set_show_as_child_window(value);
        self
    }

    /// Gets or sets the initial size of the DevTools window. The default value is 1280x720.
    pub fn size(&self) -> Size {
        self.size.get()
    }

    pub fn set_size(&self, value: Size) {
        self.size.set(value);
    }

    pub fn with_size(self: Rc<Self>, value: Size) -> Rc<Self> {
        self.set_size(value);
        self
    }

    /// Get or set the startup screen index where the DevTools window will be displayed.
    pub fn startup_screen_index(&self) -> Option<i32> {
        self.startup_screen_index.get()
    }

    pub fn set_startup_screen_index(&self, value: Option<i32>) {
        self.startup_screen_index.set(value);
    }

    pub fn with_startup_screen_index(self: Rc<Self>, value: Option<i32>) -> Rc<Self> {
        self.set_startup_screen_index(value);
        self
    }

    /// Gets or sets a value indicating whether DevTools should be displayed implemented interfaces on Control details. The default value is true.
    pub fn show_implemented_interfaces(&self) -> bool {
        self.show_implemented_interfaces.get()
    }

    pub fn set_show_implemented_interfaces(&self, value: bool) {
        self.show_implemented_interfaces.set(value);
    }

    pub fn with_show_implemented_interfaces(self: Rc<Self>, value: bool) -> Rc<Self> {
        self.set_show_implemented_interfaces(value);
        self
    }

    /// Allow to customize SreenshotHandler
    ///
    /// Default handler is [`FilePickerHandler`](super::screenshots::FilePickerHandler)
    pub fn screenshot_handler(&self) -> Rc<dyn IScreenshotHandler> {
        self.screenshot_handler.borrow().clone()
    }

    pub fn set_screenshot_handler(&self, value: Rc<dyn IScreenshotHandler>) {
        *self.screenshot_handler.borrow_mut() = value;
    }

    pub fn with_screenshot_handler(self: Rc<Self>, value: Rc<dyn IScreenshotHandler>) -> Rc<Self> {
        self.set_screenshot_handler(value);
        self
    }

    /// Gets or sets whether DevTools theme.
    pub fn theme_variant(&self) -> Option<ThemeVariant> {
        self.theme_variant.borrow().clone()
    }

    pub fn set_theme_variant(&self, value: Option<ThemeVariant>) {
        *self.theme_variant.borrow_mut() = value;
    }

    pub fn with_theme_variant(self: Rc<Self>, value: Option<ThemeVariant>) -> Rc<Self> {
        self.set_theme_variant(value);
        self
    }

    /// Get or set Focus Highlighter brush
    pub fn focus_highlighter_brush(&self) -> Option<Rc<dyn IBrush>> {
        self.focus_highlighter_brush.borrow().clone()
    }

    pub fn set_focus_highlighter_brush(&self, value: Option<Rc<dyn IBrush>>) {
        *self.focus_highlighter_brush.borrow_mut() = value;
    }

    pub fn with_focus_highlighter_brush(self: Rc<Self>, value: Option<Rc<dyn IBrush>>) -> Rc<Self> {
        self.set_focus_highlighter_brush(value);
        self
    }

    /// Set the [kind](DevToolsViewKind) of diagnostic view that show at launch of DevTools
    pub fn launch_view(&self) -> DevToolsViewKind {
        self.launch_view.get()
    }

    /// Initializes [`launch_view`](Self::launch_view) (`init` upstream).
    pub fn with_launch_view(self: Rc<Self>, value: DevToolsViewKind) -> Rc<Self> {
        self.launch_view.set(value);
        self
    }

    /// Gets or inits the [`HotKeyConfiguration`] used to activate DevTools features
    pub fn hot_keys(&self) -> Rc<HotKeyConfiguration> {
        self.hot_keys.borrow().clone()
    }

    /// Initializes [`hot_keys`](Self::hot_keys) (`init` upstream).
    pub fn with_hot_keys(self: Rc<Self>, value: Rc<HotKeyConfiguration>) -> Rc<Self> {
        *self.hot_keys.borrow_mut() = value;
        self
    }
}
