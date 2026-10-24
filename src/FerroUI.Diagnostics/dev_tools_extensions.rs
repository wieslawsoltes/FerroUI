//! Port of `DevToolsExtensions.cs`: the extension methods that attach DevTools to a top level
//! or an application.

use crate::diagnostics::{DevTools, DevToolsOptions};
use ferroui_base::input::KeyGesture;
use ferroui_controls::{Application, TopLevel};
use std::rc::Rc;

/// Extension methods for attaching DevTools..
///
/// Implemented by top levels (windows) and by the application.
pub trait DevToolsExtensions {
    /// Attaches DevTools, to be opened with the F12 key.
    fn attach_dev_tools(&self);

    /// Attaches DevTools, to be opened with the specified options.
    ///
    /// `options` are the additional settings of DevTools.
    ///
    /// For an application, attach DevTools should only be called after application
    /// initialization is complete. A good point is `on_framework_initialization_completed`:
    ///
    /// ```ignore
    /// impl ApplicationImpl for App {
    ///     fn on_framework_initialization_completed(this: &Self) {
    ///         // Create the main window of the lifetime of the application...
    ///         Self::parent_on_framework_initialization_completed(this);
    ///         this.attach_dev_tools_with_options(DevToolsOptions::new().with_startup_screen_index(Some(1)));
    ///     }
    /// }
    /// ```
    fn attach_dev_tools_with_options(&self, options: Rc<DevToolsOptions>);
}

/// The extension method of top levels for attaching DevTools with a key gesture (upstream has it
/// for top levels only).
pub trait DevToolsTopLevelExtensions {
    /// Attaches DevTools to a window, to be opened with the specified key gesture.
    ///
    /// `gesture` is the key gesture to open DevTools.
    fn attach_dev_tools_with_gesture(&self, gesture: KeyGesture);
}

impl DevToolsExtensions for TopLevel {
    /// Attaches DevTools to a window, to be opened with the F12 key.
    fn attach_dev_tools(&self) {
        DevTools::attach(self, DevToolsOptions::new());
    }

    /// Attaches DevTools to a window, to be opened with the specified options.
    fn attach_dev_tools_with_options(&self, options: Rc<DevToolsOptions>) {
        DevTools::attach(self, options);
    }
}

impl DevToolsTopLevelExtensions for TopLevel {
    fn attach_dev_tools_with_gesture(&self, gesture: KeyGesture) {
        DevTools::attach_with_gesture(self, gesture);
    }
}

impl DevToolsExtensions for Application {
    /// Attaches DevTools to an application, to be opened with the F12 key.
    fn attach_dev_tools(&self) {
        DevTools::attach_application(self, DevToolsOptions::new());
    }

    /// Attaches DevTools to an application, to be opened with the specified options.
    fn attach_dev_tools_with_options(&self, options: Rc<DevToolsOptions>) {
        DevTools::attach_application(self, options);
    }
}
