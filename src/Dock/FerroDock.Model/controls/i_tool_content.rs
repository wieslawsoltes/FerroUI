// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use crate::BoxedValue;

/// The `IToolContent` docking contract.
pub trait IToolContent: IDockable {
    /// Gets or sets tool content.
    fn content(&self) -> Option<BoxedValue>;
    /// Sets `Content`.
    fn set_content(&self, value: Option<BoxedValue>);
}

impl PartialEq for dyn IToolContent {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
