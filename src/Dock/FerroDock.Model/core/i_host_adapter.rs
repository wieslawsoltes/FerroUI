// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

/// The `IHostAdapter` docking contract.
pub trait IHostAdapter {
    /// `Save`.
    fn save(&self);
    /// `Present`.
    fn present(&self, is_dialog: bool);
    /// `Exit`.
    fn exit(&self);
    /// `SetActive`.
    fn set_active(&self);
}

impl PartialEq for dyn IHostAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
