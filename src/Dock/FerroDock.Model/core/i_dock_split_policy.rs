// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockSplitPolicy` docking contract.
pub trait IDockSplitPolicy {
    /// Determines whether a tool split should create a document pane.
    fn use_document_dock(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDock>,
        operation: DockOperation,
    ) -> bool;
}

impl PartialEq for dyn IDockSplitPolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
