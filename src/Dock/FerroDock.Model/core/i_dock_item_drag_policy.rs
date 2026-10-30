// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockItemDragPolicy` docking contract.
pub trait IDockItemDragPolicy {
    /// Determines whether a pointer gesture can drag this item.
    fn can_drag_item(&self, dockable: Rc<dyn IDockable>) -> bool;
    /// Determines whether dragging can reorder this item within its strip.
    fn can_reorder_item(&self, dockable: Rc<dyn IDockable>) -> bool;
}

impl PartialEq for dyn IDockItemDragPolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
