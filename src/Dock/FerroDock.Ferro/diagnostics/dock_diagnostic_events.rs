// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model::core::IDockable;
use ferroui_system::events::Event;
use std::rc::Rc;
ferroui_base::ferro_thread_local! {
    static SELECT_DOCKABLE_REQUESTED:Rc<Event<Option<Rc<dyn IDockable>>>>=Rc::new(Event::new());
}
/// Original diagnostic selection event raised by control hit testing.
/// The native event is UI-thread scoped; process-wide managed event behavior is not provided.
pub struct DockDiagnosticEvents;
impl DockDiagnosticEvents {
    /// Gets the shared selection event. Add returns a token for explicit removal.
    /// Handlers retain their original captures until removal; raising uses the shared
    /// System multicast snapshot, including for reentrant subscriber changes.
    pub fn select_dockable_requested() -> Rc<Event<Option<Rc<dyn IDockable>>>> {
        SELECT_DOCKABLE_REQUESTED.with(Clone::clone)
    }
    pub(crate) fn raise_select_dockable_requested(dockable: Option<Rc<dyn IDockable>>) {
        Self::select_dockable_requested().raise(&dockable);
    }
}
#[cfg(test)]
#[path = "dock_diagnostic_events_tests.rs"]
mod tests;
