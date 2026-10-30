// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockState` docking contract.
pub trait IDockState {
    /// Saves the state of the supplied dock.
    fn save(&self, dock: Rc<dyn IDock>);

    /// Restores saved state to the supplied dock.
    fn restore(&self, dock: Rc<dyn IDock>);

    /// Clears the saved state.
    fn reset(&self);
}

impl PartialEq for dyn IDockState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
