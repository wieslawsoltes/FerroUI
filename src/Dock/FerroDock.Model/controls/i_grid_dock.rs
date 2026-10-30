// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IGridDock` docking contract.
pub trait IGridDock: IDock {
    /// Gets or sets column definitions string.
    fn column_definitions(&self) -> Option<String>;
    /// Sets `ColumnDefinitions`.
    fn set_column_definitions(&self, value: Option<String>);
    /// Gets or sets row definitions string.
    fn row_definitions(&self) -> Option<String>;
    /// Sets `RowDefinitions`.
    fn set_row_definitions(&self, value: Option<String>);
}

impl PartialEq for dyn IGridDock {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
