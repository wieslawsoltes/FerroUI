// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IDockControlState` docking contract.
pub trait IDockControlState: IDockManagerState {}

impl PartialEq for dyn IDockControlState {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
