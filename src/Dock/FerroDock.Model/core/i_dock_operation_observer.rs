// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The `IDockOperationObserver` docking contract.
pub trait IDockOperationObserver {
    /// Begins observing a validated operation before its first layout mutation.
    fn begin_dock_operation(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> Option<Rc<dyn IDockOperationScope>>;
}

/// The `IDockOperationScope` docking contract.
pub trait IDockOperationScope: IDisposable {
    /// Reports that execution succeeded. Disposal alone does not report completion,
    /// including when execution throws or returns false.
    fn complete(&self);
}

impl PartialEq for dyn IDockOperationObserver {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}

impl PartialEq for dyn IDockOperationScope {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
