// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::tracking::same;
use super::FactoryBase;
use crate::controls::*;
use crate::core::events::*;
use crate::core::*;
use ferroui_system::events::Event;
use ferroui_system::IDisposable;
use std::cell::Cell;
use std::rc::{Rc, Weak};

type RootWindowContext = (Option<Rc<dyn IRootDock>>, Option<Rc<dyn IDockWindow>>);

struct EventSubscription<A: 'static> {
    event: Weak<Event<A>>,
    token: Cell<Option<u64>>,
}
impl<A: 'static> IDisposable for EventSubscription<A> {
    fn dispose(&self) {
        if let Some(token) = self.token.take() {
            if let Some(event) = self.event.upgrade() {
                event.remove(token);
            }
        }
    }
}
pub(crate) fn subscribe<A: 'static>(
    event: &Rc<Event<A>>,
    handler: Rc<dyn Fn(&A)>,
) -> Rc<dyn IDisposable> {
    let token = event.add(handler);
    Rc::new(EventSubscription {
        event: Rc::downgrade(event),
        token: Cell::new(Some(token)),
    })
}

impl FactoryBase {
    /// Invokes the dockable close hook, raises mutable cancellation and returns the final decision.
    pub fn on_dockable_closing(&self, dockable: Option<Rc<dyn IDockable>>) -> bool {
        let can_close = dockable.as_ref().is_none_or(|dockable| dockable.on_close());
        let args = DockableClosingEventArgs::new(dockable);
        args.set_cancel(!can_close);
        self.dockable_closing.raise(&args);
        !args.cancel()
    }

    /// Applies the optional preservation policy before the window hook and cancellable event.
    pub fn on_window_closing(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        let preserve = window.as_ref().is_some_and(|window| {
            self.runtime()
                .as_dock_window_close_policy()
                .is_some_and(|policy| policy.preserve_layout_on_host_close(window.clone()))
        });
        let can_close = preserve || window.as_ref().is_none_or(|window| window.on_close());
        let args = WindowClosingEventArgs::new(window);
        args.set_cancel(!can_close);
        self.window_closing.raise(&args);
        !args.cancel()
    }

    /// Clears tracking, raises the closed event and dispatches lifecycle services.
    pub fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.hooks().clear_global_dock_tracking_for_window(
            window.clone(),
            DockTrackingChangeReason::WindowClosed,
        );
        if self.window_closed.has_handlers() {
            self.window_closed
                .raise(&WindowClosedEventArgs::new(window.clone()));
        }
        self.hooks().notify_window_closed(window);
    }

    /// Clears tracking, raises the removed event and dispatches lifecycle services.
    pub fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.hooks().clear_global_dock_tracking_for_window(
            window.clone(),
            DockTrackingChangeReason::WindowRemoved,
        );
        if self.window_removed.has_handlers() {
            self.window_removed
                .raise(&WindowRemovedEventArgs::new(window.clone()));
        }
        self.hooks().notify_window_removed(window);
    }

    /// Invokes the drag-begin hook and returns the final event cancellation decision.
    pub fn on_window_move_drag_begin(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        let can_move = window
            .as_ref()
            .is_none_or(|window| window.on_move_drag_begin());
        let args = WindowMoveDragBeginEventArgs::new(window);
        args.set_cancel(!can_move);
        self.window_move_drag_begin.raise(&args);
        !args.cancel()
    }

    /// Invokes the model drag callback before raising the event.
    pub fn on_window_move_drag(&self, window: Option<Rc<dyn IDockWindow>>) {
        if let Some(window) = &window {
            window.on_move_drag();
        }
        if self.window_move_drag.has_handlers() {
            self.window_move_drag
                .raise(&WindowMoveDragEventArgs::new(window));
        }
    }

    /// Invokes the model drag-end callback before raising the event.
    pub fn on_window_move_drag_end(&self, window: Option<Rc<dyn IDockWindow>>) {
        if let Some(window) = &window {
            window.on_move_drag_end();
        }
        if self.window_move_drag_end.has_handlers() {
            self.window_move_drag_end
                .raise(&WindowMoveDragEndEventArgs::new(window));
        }
    }

    /// Subscribes to window activation; dispose the returned handle to unsubscribe.
    pub fn window_activated(
        &self,
        handler: Rc<dyn Fn(&WindowActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_activated, handler)
    }
    /// Subscribes to window deactivation; dispose the returned handle to unsubscribe.
    pub fn window_deactivated(
        &self,
        handler: Rc<dyn Fn(&WindowDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_deactivated, handler)
    }

    /// Dispatches activation using the window's current layout.
    pub fn on_window_activated(&self, window: Option<Rc<dyn IDockWindow>>) {
        let root = window.as_ref().and_then(|window| window.layout());
        self.runtime().on_window_activated_with_root(window, root);
    }

    /// Updates tracking, restores focused-owner activation and raises window activation.
    pub fn on_window_activated_with_root(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        root: Option<Rc<dyn IRootDock>>,
    ) {
        let dockable = Self::resolve_tracked_dockable(
            root.as_ref()
                .and_then(|root| root.focused_dockable())
                .or_else(|| root.as_ref().and_then(|root| root.active_dockable())),
        );
        self.hooks().update_global_dock_tracking(
            dockable,
            root.clone(),
            window.clone(),
            DockTrackingChangeReason::WindowActivated,
        );
        if let Some(focused) = root.as_ref().and_then(|root| root.focused_dockable()) {
            if let Some(owner) = focused.owner().and_then(|owner| owner.as_dock()) {
                self.runtime()
                    .set_focused_dockable(Some(owner), Some(focused));
            }
        }
        if self.window_activated.has_handlers() {
            self.window_activated
                .raise(&WindowActivatedEventArgs::new(window));
        }
    }

    /// Deactivates the focused pane, clears matching window tracking and raises deactivation.
    pub fn on_window_deactivated(&self, window: Option<Rc<dyn IDockWindow>>) {
        if let Some(owner) = window
            .as_ref()
            .and_then(|window| window.layout())
            .and_then(|root| root.focused_dockable())
            .and_then(|focused| focused.owner())
            .and_then(|owner| owner.as_dock())
        {
            Self::set_is_active(Some(owner), false);
        }
        if window.is_some() && same(&self.current_dock_window(), &window) {
            self.hooks().update_global_dock_tracking(
                None,
                None,
                None,
                DockTrackingChangeReason::WindowDeactivated,
            );
        }
        if self.window_deactivated.has_handlers() {
            self.window_deactivated
                .raise(&WindowDeactivatedEventArgs::new(window));
        }
    }

    /// Subscribes to global tracking changes; dispose the returned handle to unsubscribe.
    pub fn global_dock_tracking_changed(
        &self,
        handler: Rc<dyn Fn(&GlobalDockTrackingChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.tracking_changed, handler)
    }
    /// Subscribes to dockable initialization; dispose the returned handle to unsubscribe.
    pub fn dockable_init(
        &self,
        handler: Rc<dyn Fn(&DockableInitEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_initialized, handler)
    }
    /// Subscribes to active dockable changes; dispose the returned handle to unsubscribe.
    pub fn active_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&ActiveDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.active_changed, handler)
    }
    /// Subscribes to focus changes; dispose the returned handle to unsubscribe.
    pub fn focused_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&FocusedDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.focused_changed, handler)
    }
    /// Subscribes to dockable activation; dispose the returned handle to unsubscribe.
    pub fn dockable_activated(
        &self,
        handler: Rc<dyn Fn(&DockableActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.activated, handler)
    }
    /// Subscribes to dockable deactivation; dispose the returned handle to unsubscribe.
    pub fn dockable_deactivated(
        &self,
        handler: Rc<dyn Fn(&DockableDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.deactivated, handler)
    }

    /// Raises initialization with mutable shared context and applies an observer replacement.
    pub fn on_dockable_init(&self, dockable: Option<Rc<dyn IDockable>>) {
        let args = DockableInitEventArgs::new(dockable.clone());
        args.set_context(dockable.as_ref().and_then(|item| item.context()));
        self.dockable_initialized.raise(&args);
        if let Some(dockable) = dockable {
            if !same(&args.context(), &dockable.context()) {
                dockable.set_context(args.context());
            }
        }
    }

    fn update_tracking_for_change(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        reason: DockTrackingChangeReason,
        track_null: bool,
    ) -> RootWindowContext {
        let tracked = Self::resolve_tracked_dockable(dockable.clone());
        let root = Self::resolve_root_dock(tracked.clone().or(dockable));
        let window = root.as_ref().and_then(|root| root.window());
        if tracked.is_some() {
            if self.should_update_global_tracking_for_root(root.clone()) {
                self.hooks().update_global_dock_tracking(
                    tracked,
                    root.clone(),
                    window.clone(),
                    reason,
                );
            }
        } else if track_null {
            if let Some(current_root) = self.current_root_dock() {
                let current_window = self.current_dock_window().or_else(|| current_root.window());
                let current_dockable = Self::resolve_tracked_dockable(
                    current_root
                        .focused_dockable()
                        .or_else(|| current_root.active_dockable()),
                );
                self.hooks().update_global_dock_tracking(
                    current_dockable,
                    Some(current_root),
                    current_window,
                    reason,
                );
            }
        }
        (root, window)
    }

    /// Updates tracking, raises the active-change event, then reconciles the requested model.
    pub fn on_active_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>) {
        let (root, window) = self.update_tracking_for_change(
            dockable.clone(),
            DockTrackingChangeReason::ActiveDockableChanged,
            true,
        );
        if self.active_changed.has_handlers() {
            self.active_changed
                .raise(&ActiveDockableChangedEventArgs::with_context(
                    dockable.clone(),
                    root,
                    window,
                ));
        }
        self.synchronize_docking_window_state(dockable);
    }

    /// Updates tracking, raises the focus-change event, then reconciles the requested model.
    pub fn on_focused_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>) {
        let (root, window) = self.update_tracking_for_change(
            dockable.clone(),
            DockTrackingChangeReason::FocusedDockableChanged,
            true,
        );
        if self.focused_changed.has_handlers() {
            self.focused_changed
                .raise(&FocusedDockableChangedEventArgs::with_context(
                    dockable.clone(),
                    root,
                    window,
                ));
        }
        self.synchronize_docking_window_state(dockable);
    }

    /// Updates anchored tracking, raises activation and reconciles the requested model.
    pub fn on_dockable_activated(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.update_tracking_for_change(
            dockable.clone(),
            DockTrackingChangeReason::DockableActivated,
            false,
        );
        if self.activated.has_handlers() {
            self.activated
                .raise(&DockableActivatedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }

    /// Chooses the source tracked fallback, raises deactivation and reconciles state.
    pub fn on_dockable_deactivated(&self, dockable: Option<Rc<dyn IDockable>>) {
        if dockable.is_some() && same(&self.current_dockable(), &dockable) {
            let root = self
                .current_root_dock()
                .or_else(|| Self::resolve_root_dock(dockable.clone()));
            let mut next = root.as_ref().and_then(|root| root.focused_dockable());
            if same(&next, &dockable) {
                next = root.as_ref().and_then(|root| root.active_dockable());
            }
            if same(&next, &dockable) {
                next = None;
            }
            self.hooks().update_global_dock_tracking(
                next,
                root,
                self.current_dock_window(),
                DockTrackingChangeReason::DockableDeactivated,
            );
        }
        if self.deactivated.has_handlers() {
            self.deactivated
                .raise(&DockableDeactivatedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }

    fn should_update_global_tracking_for_root(&self, root: Option<Rc<dyn IRootDock>>) -> bool {
        let Some(root) = root else { return false };
        if self.current_root_dock().is_none() || same(&self.current_root_dock(), &Some(root)) {
            return true;
        }
        self.is_current_global_tracking_root_stale()
    }

    fn is_current_global_tracking_root_stale(&self) -> bool {
        let Some(root) = self.current_root_dock() else {
            return false;
        };
        if let Some(window) = self.current_dock_window() {
            if !same(&window.layout(), &Some(root.clone())) {
                return true;
            }
            if root.window().is_some() && !same(&root.window(), &Some(window)) {
                return true;
            }
            return false;
        }
        if root.window().is_some() {
            return true;
        }
        if self.runtime().dock_controls().count() == 0 {
            return false;
        }
        for control in self.runtime().dock_controls().iter() {
            if same(&control.layout(), &Some(root.clone() as Rc<dyn IDock>)) {
                return false;
            }
        }
        true
    }
}

impl FactoryBase {
    /// Subscribes to DockableAdded; dispose the returned handle to unsubscribe.
    pub fn dockable_added(
        &self,
        handler: Rc<dyn Fn(&DockableAddedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_added, handler)
    }
    /// Subscribes to DockableRemoved; dispose the returned handle to unsubscribe.
    pub fn dockable_removed(
        &self,
        handler: Rc<dyn Fn(&DockableRemovedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_removed, handler)
    }
    /// Subscribes to DockableClosing; dispose the returned handle to unsubscribe.
    pub fn dockable_closing(
        &self,
        handler: Rc<dyn Fn(&DockableClosingEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_closing, handler)
    }
    /// Subscribes to DockableClosed; dispose the returned handle to unsubscribe.
    pub fn dockable_closed(
        &self,
        handler: Rc<dyn Fn(&DockableClosedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_closed, handler)
    }
    /// Subscribes to DockableMoved; dispose the returned handle to unsubscribe.
    pub fn dockable_moved(
        &self,
        handler: Rc<dyn Fn(&DockableMovedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_moved, handler)
    }
    /// Subscribes to DockableDocked; dispose the returned handle to unsubscribe.
    pub fn dockable_docked(
        &self,
        handler: Rc<dyn Fn(&DockableDockedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_docked, handler)
    }
    /// Subscribes to DockableUndocked; dispose the returned handle to unsubscribe.
    pub fn dockable_undocked(
        &self,
        handler: Rc<dyn Fn(&DockableUndockedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_undocked, handler)
    }
    /// Subscribes to DockableSwapped; dispose the returned handle to unsubscribe.
    pub fn dockable_swapped(
        &self,
        handler: Rc<dyn Fn(&DockableSwappedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_swapped, handler)
    }
    /// Subscribes to DockablePinned; dispose the returned handle to unsubscribe.
    pub fn dockable_pinned(
        &self,
        handler: Rc<dyn Fn(&DockablePinnedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_pinned, handler)
    }
    /// Subscribes to DockableUnpinned; dispose the returned handle to unsubscribe.
    pub fn dockable_unpinned(
        &self,
        handler: Rc<dyn Fn(&DockableUnpinnedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_unpinned, handler)
    }
    /// Subscribes to DockableHidden; dispose the returned handle to unsubscribe.
    pub fn dockable_hidden(
        &self,
        handler: Rc<dyn Fn(&DockableHiddenEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_hidden, handler)
    }
    /// Subscribes to DockableRestored; dispose the returned handle to unsubscribe.
    pub fn dockable_restored(
        &self,
        handler: Rc<dyn Fn(&DockableRestoredEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.dockable_restored, handler)
    }
    /// Subscribes to WindowOpened; dispose the returned handle to unsubscribe.
    pub fn window_opened(
        &self,
        handler: Rc<dyn Fn(&WindowOpenedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_opened, handler)
    }
    /// Subscribes to WindowClosing; dispose the returned handle to unsubscribe.
    pub fn window_closing(
        &self,
        handler: Rc<dyn Fn(&WindowClosingEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_closing, handler)
    }
    /// Subscribes to WindowClosed; dispose the returned handle to unsubscribe.
    pub fn window_closed(
        &self,
        handler: Rc<dyn Fn(&WindowClosedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_closed, handler)
    }
    /// Subscribes to WindowAdded; dispose the returned handle to unsubscribe.
    pub fn window_added(&self, handler: Rc<dyn Fn(&WindowAddedEventArgs)>) -> Rc<dyn IDisposable> {
        subscribe(&self.window_added, handler)
    }
    /// Subscribes to WindowRemoved; dispose the returned handle to unsubscribe.
    pub fn window_removed(
        &self,
        handler: Rc<dyn Fn(&WindowRemovedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_removed, handler)
    }
    /// Subscribes to WindowMoveDragBegin; dispose the returned handle to unsubscribe.
    pub fn window_move_drag_begin(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragBeginEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_move_drag_begin, handler)
    }
    /// Subscribes to WindowMoveDrag; dispose the returned handle to unsubscribe.
    pub fn window_move_drag(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_move_drag, handler)
    }
    /// Subscribes to WindowMoveDragEnd; dispose the returned handle to unsubscribe.
    pub fn window_move_drag_end(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEndEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.window_move_drag_end, handler)
    }
    /// Raises DockableAdded in the source callback order.
    pub fn on_dockable_added(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_added.has_handlers() {
            self.dockable_added
                .raise(&DockableAddedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises DockableRemoved in the source callback order.
    pub fn on_dockable_removed(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_removed.has_handlers() {
            self.dockable_removed
                .raise(&DockableRemovedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises DockableClosed in the source callback order.
    pub fn on_dockable_closed(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_closed.has_handlers() {
            self.dockable_closed
                .raise(&DockableClosedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises DockableMoved in the source callback order.
    pub fn on_dockable_moved(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_moved.has_handlers() {
            self.dockable_moved
                .raise(&DockableMovedEventArgs::new(dockable.clone()));
        }
    }
    /// Raises DockableSwapped in the source callback order.
    pub fn on_dockable_swapped(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_swapped.has_handlers() {
            self.dockable_swapped
                .raise(&DockableSwappedEventArgs::new(dockable.clone()));
        }
    }
    /// Raises DockablePinned in the source callback order.
    pub fn on_dockable_pinned(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_pinned.has_handlers() {
            self.dockable_pinned
                .raise(&DockablePinnedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises DockableUnpinned in the source callback order.
    pub fn on_dockable_unpinned(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_unpinned.has_handlers() {
            self.dockable_unpinned
                .raise(&DockableUnpinnedEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises DockableHidden in the source callback order.
    pub fn on_dockable_hidden(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_hidden.has_handlers() {
            self.dockable_hidden
                .raise(&DockableHiddenEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises DockableRestored in the source callback order.
    pub fn on_dockable_restored(&self, dockable: Option<Rc<dyn IDockable>>) {
        if self.dockable_restored.has_handlers() {
            self.dockable_restored
                .raise(&DockableRestoredEventArgs::new(dockable.clone()));
        }
        self.synchronize_docking_window_state(dockable);
    }
    /// Raises WindowAdded in the source callback order.
    pub fn on_window_added(&self, window: Option<Rc<dyn IDockWindow>>) {
        if self.window_added.has_handlers() {
            self.window_added
                .raise(&WindowAddedEventArgs::new(window.clone()));
        }
    }
    /// Raises WindowOpened in the source callback order.
    pub fn on_window_opened(&self, window: Option<Rc<dyn IDockWindow>>) {
        if self.window_opened.has_handlers() {
            self.window_opened
                .raise(&WindowOpenedEventArgs::new(window.clone()));
        }
    }
    /// Raises DockableDocked with the requested docking operation.
    pub fn on_dockable_docked(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        operation: DockOperation,
    ) {
        if self.dockable_docked.has_handlers() {
            self.dockable_docked
                .raise(&DockableDockedEventArgs::new(dockable, operation));
        }
    }
    /// Raises DockableUndocked with the requested docking operation.
    pub fn on_dockable_undocked(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        operation: DockOperation,
    ) {
        if self.dockable_undocked.has_handlers() {
            self.dockable_undocked
                .raise(&DockableUndockedEventArgs::new(dockable, operation));
        }
    }
}

impl IFactoryEvents for FactoryBase {
    fn active_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&ActiveDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::active_dockable_changed(self, handler)
    }
    fn focused_dockable_changed(
        &self,
        handler: Rc<dyn Fn(&FocusedDockableChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::focused_dockable_changed(self, handler)
    }
    fn global_dock_tracking_changed(
        &self,
        handler: Rc<dyn Fn(&GlobalDockTrackingChangedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::global_dock_tracking_changed(self, handler)
    }
    fn dockable_init(&self, handler: Rc<dyn Fn(&DockableInitEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_init(self, handler)
    }
    fn dockable_added(&self, handler: Rc<dyn Fn(&DockableAddedEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_added(self, handler)
    }
    fn dockable_removed(
        &self,
        handler: Rc<dyn Fn(&DockableRemovedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_removed(self, handler)
    }
    fn dockable_closing(
        &self,
        handler: Rc<dyn Fn(&DockableClosingEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_closing(self, handler)
    }
    fn dockable_closed(
        &self,
        handler: Rc<dyn Fn(&DockableClosedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_closed(self, handler)
    }
    fn dockable_moved(&self, handler: Rc<dyn Fn(&DockableMovedEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_moved(self, handler)
    }
    fn dockable_docked(
        &self,
        handler: Rc<dyn Fn(&DockableDockedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_docked(self, handler)
    }
    fn dockable_undocked(
        &self,
        handler: Rc<dyn Fn(&DockableUndockedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_undocked(self, handler)
    }
    fn dockable_swapped(
        &self,
        handler: Rc<dyn Fn(&DockableSwappedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_swapped(self, handler)
    }
    fn dockable_pinned(
        &self,
        handler: Rc<dyn Fn(&DockablePinnedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_pinned(self, handler)
    }
    fn dockable_unpinned(
        &self,
        handler: Rc<dyn Fn(&DockableUnpinnedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_unpinned(self, handler)
    }
    fn dockable_hidden(
        &self,
        handler: Rc<dyn Fn(&DockableHiddenEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_hidden(self, handler)
    }
    fn dockable_restored(
        &self,
        handler: Rc<dyn Fn(&DockableRestoredEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_restored(self, handler)
    }
    fn window_opened(&self, handler: Rc<dyn Fn(&WindowOpenedEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::window_opened(self, handler)
    }
    fn window_closing(&self, handler: Rc<dyn Fn(&WindowClosingEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::window_closing(self, handler)
    }
    fn window_closed(&self, handler: Rc<dyn Fn(&WindowClosedEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::window_closed(self, handler)
    }
    fn window_added(&self, handler: Rc<dyn Fn(&WindowAddedEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::window_added(self, handler)
    }
    fn window_removed(&self, handler: Rc<dyn Fn(&WindowRemovedEventArgs)>) -> Rc<dyn IDisposable> {
        FactoryBase::window_removed(self, handler)
    }
    fn window_move_drag_begin(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragBeginEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::window_move_drag_begin(self, handler)
    }
    fn window_move_drag(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::window_move_drag(self, handler)
    }
    fn window_move_drag_end(
        &self,
        handler: Rc<dyn Fn(&WindowMoveDragEndEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::window_move_drag_end(self, handler)
    }
    fn window_activated(
        &self,
        handler: Rc<dyn Fn(&WindowActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::window_activated(self, handler)
    }
    fn dockable_activated(
        &self,
        handler: Rc<dyn Fn(&DockableActivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_activated(self, handler)
    }
    fn window_deactivated(
        &self,
        handler: Rc<dyn Fn(&WindowDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::window_deactivated(self, handler)
    }
    fn dockable_deactivated(
        &self,
        handler: Rc<dyn Fn(&DockableDeactivatedEventArgs)>,
    ) -> Rc<dyn IDisposable> {
        FactoryBase::dockable_deactivated(self, handler)
    }
    fn on_active_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_active_dockable_changed(self, dockable)
    }
    fn on_focused_dockable_changed(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_focused_dockable_changed(self, dockable)
    }
    fn on_dockable_init(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_init(self, dockable)
    }
    fn on_dockable_added(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_added(self, dockable)
    }
    fn on_dockable_removed(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_removed(self, dockable)
    }
    fn on_dockable_closing(&self, dockable: Option<Rc<dyn IDockable>>) -> bool {
        FactoryBase::on_dockable_closing(self, dockable)
    }
    fn on_dockable_closed(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_closed(self, dockable)
    }
    fn on_dockable_moved(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_moved(self, dockable)
    }
    fn on_dockable_docked(&self, dockable: Option<Rc<dyn IDockable>>, operation: DockOperation) {
        FactoryBase::on_dockable_docked(self, dockable, operation)
    }
    fn on_dockable_undocked(&self, dockable: Option<Rc<dyn IDockable>>, operation: DockOperation) {
        FactoryBase::on_dockable_undocked(self, dockable, operation)
    }
    fn on_dockable_swapped(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_swapped(self, dockable)
    }
    fn on_dockable_pinned(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_pinned(self, dockable)
    }
    fn on_dockable_unpinned(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_unpinned(self, dockable)
    }
    fn on_dockable_hidden(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_hidden(self, dockable)
    }
    fn on_dockable_restored(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_restored(self, dockable)
    }
    fn on_window_opened(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_opened(self, window)
    }
    fn on_window_closing(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        FactoryBase::on_window_closing(self, window)
    }
    fn on_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_closed(self, window)
    }
    fn on_window_added(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_added(self, window)
    }
    fn on_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_removed(self, window)
    }
    fn on_window_move_drag_begin(&self, window: Option<Rc<dyn IDockWindow>>) -> bool {
        FactoryBase::on_window_move_drag_begin(self, window)
    }
    fn on_window_move_drag(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_move_drag(self, window)
    }
    fn on_window_move_drag_end(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_move_drag_end(self, window)
    }
    fn on_window_activated(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_activated(self, window)
    }
    fn on_window_activated_with_root(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        root_dock: Option<Rc<dyn IRootDock>>,
    ) {
        FactoryBase::on_window_activated_with_root(self, window, root_dock)
    }
    fn on_dockable_activated(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_activated(self, dockable)
    }
    fn on_window_deactivated(&self, window: Option<Rc<dyn IDockWindow>>) {
        FactoryBase::on_window_deactivated(self, window)
    }
    fn on_dockable_deactivated(&self, dockable: Option<Rc<dyn IDockable>>) {
        FactoryBase::on_dockable_deactivated(self, dockable)
    }
}
