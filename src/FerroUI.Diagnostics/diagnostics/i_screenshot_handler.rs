//! Port of `Diagnostics/IScreenshotHandler.cs`.

use ferroui_base::input::LocalBoxFuture;
use ferroui_base::Ref;
use ferroui_controls::Control;

/// Allowed to define custom handler for Shreeshot
pub trait IScreenshotHandler {
    /// Handle the Screenshot
    ///
    /// C# `Task Take(Control control)`: the future completes when the screenshot is taken; an
    /// error is what the task of the original faults with.
    fn take(&self, control: &Ref<Control>) -> LocalBoxFuture<std::io::Result<()>>;
}

impl PartialEq for dyn IScreenshotHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
