use super::{CustomWindowStylesCallback, CustomWndProcHookCallback, ITopLevelImpl, WindowCornerPreference};

/// Win32-specific options of a top-level.
pub trait IWin32OptionsTopLevelImpl: ITopLevelImpl {
    /// Gets the callback to set the window styles.
    fn window_styles_callback(&self) -> Option<CustomWindowStylesCallback>;

    /// Sets the callback to set the window styles.
    fn set_window_styles_callback(&self, value: Option<CustomWindowStylesCallback>);

    /// Gets the custom callback for the window procedure.
    fn wnd_proc_hook_callback(&self) -> Option<CustomWndProcHookCallback>;

    /// Sets the custom callback for the window procedure.
    fn set_wnd_proc_hook_callback(&self, value: Option<CustomWndProcHookCallback>);

    /// Sets hints that configure the shape of window corners.
    fn set_window_corner_preference(&self, preference: WindowCornerPreference);
}
