/// The window type hint of an X11 window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum X11NetWmWindowType {
    #[default]
    Normal = 0,
    Dialog = 1,
    Utility = 2,
    Menu = 3,
    Toolbar = 4,
    Splash = 5,
    Dock = 6,
    Desktop = 7,
}

/// X11-specific options of a top-level.
pub trait IX11OptionsToplevelImplFeature {
    fn set_net_wm_window_type(&self, type_: X11NetWmWindowType);
    fn set_wm_class(&self, class_name: Option<&str>);
}
