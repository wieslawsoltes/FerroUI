// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IDockDock` docking contract.
pub trait IDockDock: IDock {
    /// Gets or sets a value which indicates whether the last child of the fills the remaining space in the panel.
    fn last_child_fill(&self) -> bool;
    /// Sets `LastChildFill`.
    fn set_last_child_fill(&self, value: bool);
}

impl PartialEq for dyn IDockDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
