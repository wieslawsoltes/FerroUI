// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::DockCommandBarDefinition;
use crate::collections::ReadOnlyListHandle;
use ferroui_system::events::Event;
use std::rc::Rc;
/// Provides the original command bar definitions for a dockable.
pub trait IDockCommandBarProvider {
    /// Gets the source read-only definition list.
    fn get_command_bars(&self) -> ReadOnlyListHandle<Rc<DockCommandBarDefinition>>;
    /// Raised when command bar definitions change.
    fn command_bars_changed(&self) -> &Event<()>;
}
impl PartialEq for dyn IDockCommandBarProvider {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
