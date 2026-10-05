/// Identifies one of the clipboards of the platform.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ClipboardType {
    /// The default clipboard.
    #[default]
    Default = 0,

    /// The primary selection clipboard (X11, Wayland).
    PrimarySelection = 1,
}
