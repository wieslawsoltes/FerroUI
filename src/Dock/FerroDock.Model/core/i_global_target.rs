// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// The `IGlobalTarget` docking contract.
pub trait IGlobalTarget {}

impl PartialEq for dyn IGlobalTarget {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
