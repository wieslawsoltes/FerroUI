// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::collections::ListHandle;
use crate::core::IDockWindow;
use crate::services::IWindowLifecycleService;
use std::rc::Rc;

impl FactoryBase {
    /// Gets the shared mutable lifecycle services list. Null entries are skipped.
    pub fn window_lifecycle_services(&self) -> ListHandle<Option<Rc<dyn IWindowLifecycleService>>> {
        self.lifecycle_services.clone()
    }

    /// Notifies lifecycle services in live list order after a window closes.
    pub fn notify_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        if self.window_lifecycle_services().count() == 0 {
            return;
        }
        for service in self.window_lifecycle_services().iter().flatten() {
            service.on_window_closed(window.clone());
        }
    }

    /// Notifies lifecycle services in live list order after a window is removed.
    pub fn notify_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        if self.window_lifecycle_services().count() == 0 {
            return;
        }
        for service in self.window_lifecycle_services().iter().flatten() {
            service.on_window_removed(window.clone());
        }
    }
}
