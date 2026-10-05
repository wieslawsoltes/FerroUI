use super::{ITopLevelImpl, ITrayIconImpl, IWindowImpl};
use std::rc::Rc;

/// The windowing platform: creates the windows and other top-levels of the
/// backend. Registered as a service under `dyn IWindowingPlatform`.
pub trait IWindowingPlatform {
    /// Creates a window.
    fn create_window(&self) -> Rc<dyn IWindowImpl>;

    /// Creates a top-level that can be embedded into a foreign window.
    fn create_embeddable_top_level(&self) -> Rc<dyn ITopLevelImpl>;

    /// Creates a window that can be embedded into a foreign window.
    fn create_embeddable_window(&self) -> Rc<dyn IWindowImpl>;

    /// Creates a tray icon, if the platform supports them.
    fn create_tray_icon(&self) -> Option<Rc<dyn ITrayIconImpl>>;

    /// Fills `z_order` with the z-order of each of `windows`; a window with
    /// a higher value is above a window with a lower one.
    ///
    /// Both slices have the same length.
    fn get_windows_z_order(&self, windows: &[Rc<dyn IWindowImpl>], z_order: &mut [i64]);
}
