// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::*;
use std::rc::Rc;
/// Source aggregate of per-host and global overlay service references.
pub trait IHostOverlayServices {
    /// Gets the per-host busy service.
    fn busy(&self) -> Rc<dyn IDockBusyService>;
    /// Gets the per-host dialog service.
    fn dialogs(&self) -> Rc<dyn IDockDialogService>;
    /// Gets the per-host confirmation service.
    fn confirmations(&self) -> Rc<dyn IDockConfirmationService>;
    /// Gets the shared global busy service.
    fn global_busy_service(&self) -> Rc<dyn IDockGlobalBusyService>;
    /// Gets the shared global dialog service.
    fn global_dialog_service(&self) -> Rc<dyn IDockGlobalDialogService>;
    /// Gets the shared global confirmation service.
    fn global_confirmation_service(&self) -> Rc<dyn IDockGlobalConfirmationService>;
}

impl PartialEq for dyn IHostOverlayServices {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
