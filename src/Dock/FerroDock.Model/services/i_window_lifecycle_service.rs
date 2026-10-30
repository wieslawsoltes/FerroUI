// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::IDockWindow;
use std::rc::Rc;

/// Handles window lifecycle events for cleanup and service coordination.
pub trait IWindowLifecycleService {
    /// Called when a window has been closed.
    fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when a window has been removed from the layout.
    fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>);
}

impl PartialEq for dyn IWindowLifecycleService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
