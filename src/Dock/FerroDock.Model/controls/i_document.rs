// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;

/// The `IDocument` docking contract.
pub trait IDocument: IDockable + ILocalTarget {}

impl PartialEq for dyn IDocument {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
