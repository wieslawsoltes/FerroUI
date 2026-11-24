//! Port of `Server/Persistent/WaylandKeyInfo.cs` (upstream 12.2.0, `8dae868308`).

use ferroui_base::input::raw::RawKeyEventArgs;
use ferroui_base::input::{Key, PhysicalKey, PlatformKeyInfo, RawInputModifiers};

/// A key resolved against the xkb state on the Wayland thread.
#[derive(Clone, Debug, PartialEq)]
pub struct WaylandKeyInfo {
    pub key: Key,
    pub physical_key: PhysicalKey,
    pub key_symbol: Option<String>,
    /// The text with every modifier applied (`KeyEventArgs::key_text`), presses only.
    pub key_text: Option<String>,
    /// The text of the key symbol with no modifier transformation
    /// (`KeyEventArgs::alternate_key_text`), presses only.
    pub alternate_key_text: Option<String>,
    /// The pressed modifiers the layout used (`KeyEventArgs::consumed_modifiers`), presses only.
    pub consumed_modifiers: Option<RawInputModifiers>,
    pub platform_key_info: PlatformKeyInfo,
}

impl WaylandKeyInfo {
    /// Copies the texts, the consumed modifiers and the platform data to `args`.
    pub fn apply_to(&self, args: &RawKeyEventArgs) {
        args.set_key_text(self.key_text.clone());
        args.set_alternate_key_text(self.alternate_key_text.clone());
        args.set_consumed_modifiers(self.consumed_modifiers);
        args.set_platform_key_info(Some(self.platform_key_info));
    }
}
