/// Whether the user can zoom the content of a scroll viewer (WinUI
/// `Microsoft.UI.Xaml.Controls.ZoomMode`; not upstream, see
/// docs/porting/winui-composition.md, phase 7).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ZoomMode {
    /// The user cannot zoom.
    #[default]
    Disabled = 0,
    /// The user can zoom (Ctrl and the mouse wheel).
    Enabled = 1,
}
