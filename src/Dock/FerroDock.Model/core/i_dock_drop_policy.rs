// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockDropPolicy` docking contract.
pub trait IDockDropPolicy {
    /// Determines whether a source may dock at a destination using the requested operation.
    fn can_drop(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> bool;
}

impl PartialEq for dyn IDockDropPolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
