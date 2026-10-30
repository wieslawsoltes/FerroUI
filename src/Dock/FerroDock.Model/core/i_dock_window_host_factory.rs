// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::*;
use std::rc::Rc;

/// The `IDockWindowHostFactory` docking contract.
pub trait IDockWindowHostFactory {
    /// Creates a host for the supplied window, or returns null to leave it unhosted.
    fn create_host_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
    ) -> Option<Rc<dyn IHostWindow>>;
}

impl PartialEq for dyn IDockWindowHostFactory {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
