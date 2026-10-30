// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IUniformGridDock` docking contract.
pub trait IUniformGridDock: IDock {
    /// Gets or sets number of rows.
    fn rows(&self) -> i32;
    /// Sets `Rows`.
    fn set_rows(&self, value: i32);
    /// Gets or sets number of columns.
    fn columns(&self) -> i32;
    /// Sets `Columns`.
    fn set_columns(&self, value: i32);
}

impl PartialEq for dyn IUniformGridDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
