// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_system::task::Shared;
use ferroui_system::ObservableError;
/// Reload contract for dockables and routed view models.
pub trait IReloadable {
    /// Reloads the current view state; the task is the `Shared` of its result.
    fn reload_async(&self) -> Shared<Result<(), ObservableError>>;
}
