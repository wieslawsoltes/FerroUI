// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Provides a stable optional identifier for recycling dockable content.
pub trait IControlRecyclingIdProvider {
    /// Gets the content's optional control recycling identifier.
    fn get_control_recycling_id(&self) -> Option<String>;
}

impl PartialEq for dyn IControlRecyclingIdProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
