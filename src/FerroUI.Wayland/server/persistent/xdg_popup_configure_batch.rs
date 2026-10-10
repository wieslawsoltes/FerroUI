//! What a compositor says about a popup between two `xdg_surface.configure`
//! events (the port of `XdgPopupConfigureBatch.cs`).

/// Worker-side accumulator for a single xdg_popup configure batch, sealed by
/// the wrapping xdg_surface.configure(serial). Mirrors `XdgConfigureBatch`
/// but carries the popup-specific (x, y, width, height) payload.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct XdgPopupConfigureBatch {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub serial: u32,
    /// Not in the reference: the batch is the first of a popup that was created again
    /// because the compositor has no `xdg_popup.reposition` (`xdg_wm_base` before version
    /// 3). The surface of such a popup is unmapped and needs a frame, although nothing of
    /// its content changed.
    pub recreated: bool,
}
