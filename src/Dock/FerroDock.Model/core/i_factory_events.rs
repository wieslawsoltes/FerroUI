// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::events::*;
use crate::core::*;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The `IFactoryEvents` docking contract.
pub trait IFactoryEvents {
    /// Subscribes to `ActiveDockableChanged`; dispose the handle to unsubscribe.
    fn active_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&ActiveDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `FocusedDockableChanged`; dispose the handle to unsubscribe.
    fn focused_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&FocusedDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `GlobalDockTrackingChanged`; dispose the handle to unsubscribe.
    fn global_dock_tracking_changed(
        &self,
        handler: Rc<dyn Fn(&GlobalDockTrackingChangedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableInit`; dispose the handle to unsubscribe.
    fn dockable_init(&self, handler: Rc<dyn Fn(&DockableInitEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableAdded`; dispose the handle to unsubscribe.
    fn dockable_added(&self, handler: Rc<dyn Fn(&DockableAddedEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableRemoved`; dispose the handle to unsubscribe.
    fn dockable_removed(
        &self,
        handler: Rc<dyn Fn(&DockableRemovedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableClosing`; dispose the handle to unsubscribe.
    fn dockable_closing(
        &self,
        handler: Rc<dyn Fn(&DockableClosingEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableClosed`; dispose the handle to unsubscribe.
    fn dockable_closed(&self, handler: Rc<dyn Fn(&DockableClosedEventArgs)>)
        -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableMoved`; dispose the handle to unsubscribe.
    fn dockable_moved(&self, handler: Rc<dyn Fn(&DockableMovedEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableDocked`; dispose the handle to unsubscribe.
    fn dockable_docked(&self, handler: Rc<dyn Fn(&DockableDockedEventArgs)>)
        -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableUndocked`; dispose the handle to unsubscribe.
    fn dockable_undocked(
        &self,
        handler: Rc<dyn Fn(&DockableUndockedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableSwapped`; dispose the handle to unsubscribe.
    fn dockable_swapped(
        &self,
        handler: Rc<dyn Fn(&DockableSwappedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockablePinned`; dispose the handle to unsubscribe.
    fn dockable_pinned(&self, handler: Rc<dyn Fn(&DockablePinnedEventArgs)>)
        -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableUnpinned`; dispose the handle to unsubscribe.
    fn dockable_unpinned(
        &self,
        handler: Rc<dyn Fn(&DockableUnpinnedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableHidden`; dispose the handle to unsubscribe.
    fn dockable_hidden(&self, handler: Rc<dyn Fn(&DockableHiddenEventArgs)>)
        -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableRestored`; dispose the handle to unsubscribe.
    fn dockable_restored(
        &self,
        handler: Rc<dyn Fn(&DockableRestoredEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowOpened`; dispose the handle to unsubscribe.
    fn window_opened(&self, handler: Rc<dyn Fn(&WindowOpenedEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowClosing`; dispose the handle to unsubscribe.
    fn window_closing(&self, handler: Rc<dyn Fn(&WindowClosingEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowClosed`; dispose the handle to unsubscribe.
    fn window_closed(&self, handler: Rc<dyn Fn(&WindowClosedEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowAdded`; dispose the handle to unsubscribe.
    fn window_added(&self, handler: Rc<dyn Fn(&WindowAddedEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowRemoved`; dispose the handle to unsubscribe.
    fn window_removed(&self, handler: Rc<dyn Fn(&WindowRemovedEventArgs)>) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowMoveDragBegin`; dispose the handle to unsubscribe.
    fn window_move_drag_begin(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragBeginEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowMoveDrag`; dispose the handle to unsubscribe.
    fn window_move_drag(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowMoveDragEnd`; dispose the handle to unsubscribe.
    fn window_move_drag_end(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEndEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowActivated`; dispose the handle to unsubscribe.
    fn window_activated(
        &self,
        handler: Rc<dyn Fn(&WindowActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableActivated`; dispose the handle to unsubscribe.
    fn dockable_activated(
        &self,
        handler: Rc<dyn Fn(&DockableActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `WindowDeactivated`; dispose the handle to unsubscribe.
    fn window_deactivated(
        &self,
        handler: Rc<dyn Fn(&WindowDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Subscribes to `DockableDeactivated`; dispose the handle to unsubscribe.
    fn dockable_deactivated(
        &self,
        handler: Rc<dyn Fn(&DockableDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable>;
    /// Called when the active dockable changed.
    fn on_active_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the focused dockable changed.
    fn on_focused_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been init.
    fn on_dockable_init(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been added.
    fn on_dockable_added(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been removed.
    fn on_dockable_removed(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable is closing.
    fn on_dockable_closing(&self, dockable: Option<Rc<dyn IDockable>>) -> bool;
    /// Called when the dockable has been closed.
    fn on_dockable_closed(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been moved.
    fn on_dockable_moved(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been docked.
    fn on_dockable_docked(&self, dockable: Option<Rc<dyn IDockable>>, operation: DockOperation);
    /// Called when the dockable has been undocked.
    fn on_dockable_undocked(&self, dockable: Option<Rc<dyn IDockable>>, operation: DockOperation);
    /// Called when the dockable has been swapped.
    fn on_dockable_swapped(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been pinned.
    fn on_dockable_pinned(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been unpinned.
    fn on_dockable_unpinned(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been hidden.
    fn on_dockable_hidden(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the dockable has been restored.
    fn on_dockable_restored(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the window has been opened.
    fn on_window_opened(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when the window is closing.
    fn on_window_closing(&self, window: Option<Rc<dyn IDockWindow>>) -> bool;
    /// Called when the window has been closed.
    fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when the window has been added.
    fn on_window_added(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when the window has been removed.
    fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called before the window dragging starts.
    fn on_window_move_drag_begin(&self, window: Option<Rc<dyn IDockWindow>>) -> bool;
    /// Called when the window is dragged.
    fn on_window_move_drag(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called after the window dragging ended.
    fn on_window_move_drag_end(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when the window has been activated.
    fn on_window_activated(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when a window has been activated for an explicitly resolved root dock.
    fn on_window_activated_with_root(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        root_dock: Option<Rc<dyn IRootDock>>,
    );
    /// Called when the dockable has been activated.
    fn on_dockable_activated(&self, dockable: Option<Rc<dyn IDockable>>);
    /// Called when the window has been deactivated.
    fn on_window_deactivated(&self, window: Option<Rc<dyn IDockWindow>>);
    /// Called when the dockable has been deactivated.
    fn on_dockable_deactivated(&self, dockable: Option<Rc<dyn IDockable>>);
}

impl PartialEq for dyn IFactoryEvents {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
