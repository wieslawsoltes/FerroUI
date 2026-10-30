// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockOuterSplitPolicy` docking contract.
pub trait IDockOuterSplitPolicy {
    /// Resolves the hovered target to itself or an enclosing dock to split beside.
    fn get_outer_split_target(
        &self,
        source: Rc<dyn IDockable>,
        target: Rc<dyn IDockable>,
        operation: DockOperation,
    ) -> Option<Rc<dyn IDock>>;
}

impl PartialEq for dyn IDockOuterSplitPolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
