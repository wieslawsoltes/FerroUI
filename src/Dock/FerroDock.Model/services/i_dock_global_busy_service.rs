// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Tracks global busy state for overlays across windows.
pub trait IDockGlobalBusyService: INotifyPropertyChanged {
    /// Returns whether any window is busy.
    fn is_busy(&self) -> bool;
    /// Gets the global busy message.
    fn message(&self) -> Option<String>;
    /// Begins a global busy scope, ended by explicitly disposing the returned handle.
    fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable>;
}

impl PartialEq for dyn IDockGlobalBusyService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
