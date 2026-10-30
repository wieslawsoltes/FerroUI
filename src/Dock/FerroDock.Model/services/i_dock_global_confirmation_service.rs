// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::IDisposable;
use std::rc::Rc;
/// Tracks global confirmation scopes for overlays across windows.
pub trait IDockGlobalConfirmationService: INotifyPropertyChanged {
    /// Gets whether any confirmation scope is open.
    fn is_confirmation_open(&self) -> bool;
    /// Gets the global message.
    fn message(&self) -> Option<String>;
    /// Begins a scope with an optional message; dispose explicitly to end it.
    fn begin(&self, message: Option<String>) -> Rc<dyn IDisposable>;
}

impl PartialEq for dyn IDockGlobalConfirmationService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
