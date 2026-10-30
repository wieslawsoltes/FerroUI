// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
/// Provides the source hooks for visual tree attachment and detachment.
pub trait IVisualTreeLifecycle {
    /// Called when the instance is attached to the visual tree.
    fn on_attached_to_visual_tree(&self);
    /// Called when the instance is detached from the visual tree.
    fn on_detached_from_visual_tree(&self);
}
impl PartialEq for dyn IVisualTreeLifecycle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
