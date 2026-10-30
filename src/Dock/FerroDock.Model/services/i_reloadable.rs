// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_system::threading::tasks::Task;
/// Reload contract for dockables and routed view models.
pub trait IReloadable {
    /// Reloads the current view state.
    fn reload_async(&self) -> Task<()>;
}
