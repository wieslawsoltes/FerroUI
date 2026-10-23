//! Port of `Diagnostics/Conventions.cs`.

use super::screenshots::FilePickerHandler;
use super::IScreenshotHandler;
use std::rc::Rc;

/// The defaults of DevTools.
pub struct Conventions;

impl Conventions {
    /// The screenshot handler of [`DevToolsOptions`](super::DevToolsOptions) by default: one
    /// [`FilePickerHandler`] (per thread; the objects of the port are not shared between threads).
    pub fn default_screenshot_handler() -> Rc<dyn IScreenshotHandler> {
        ferroui_base::ferro_thread_local! {
            static DEFAULT_SCREENSHOT_HANDLER: Rc<dyn IScreenshotHandler> = Rc::new(FilePickerHandler::new());
        }
        DEFAULT_SCREENSHOT_HANDLER.with(Rc::clone)
    }
}
