// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockWindowClosePolicy` docking contract.
pub trait IDockWindowClosePolicy {
    /// Returns whether closing this host preserves its window model and content.
    fn preserve_layout_on_host_close(&self, window: Rc<dyn IDockWindow>) -> bool;
}

impl PartialEq for dyn IDockWindowClosePolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
