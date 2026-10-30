// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// Optional capability values used by root and dock policies.
pub trait IDockCapabilityPolicy {
    /// Gets the optional close capability.
    fn can_close(&self) -> Option<bool>;
    /// Sets the optional close capability.
    fn set_can_close(&self, value: Option<bool>);
    /// Gets the optional pin capability.
    fn can_pin(&self) -> Option<bool>;
    /// Sets the optional pin capability.
    fn set_can_pin(&self, value: Option<bool>);
    /// Gets the optional float capability.
    fn can_float(&self) -> Option<bool>;
    /// Sets the optional float capability.
    fn set_can_float(&self, value: Option<bool>);
    /// Gets the optional drag capability.
    fn can_drag(&self) -> Option<bool>;
    /// Sets the optional drag capability.
    fn set_can_drag(&self, value: Option<bool>);
    /// Gets the optional drop capability.
    fn can_drop(&self) -> Option<bool>;
    /// Sets the optional drop capability.
    fn set_can_drop(&self, value: Option<bool>);
    /// Gets the optional dock-as-document capability.
    fn can_dock_as_document(&self) -> Option<bool>;
    /// Sets the optional dock-as-document capability.
    fn set_can_dock_as_document(&self, value: Option<bool>);
}

impl PartialEq for dyn IDockCapabilityPolicy {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
