// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::controls::*;
use crate::core::events::*;
use crate::core::*;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) fn same<T: ?Sized>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => Rc::ptr_eq(left, right),
        (None, None) => true,
        _ => false,
    }
}

impl FactoryBase {
    /// Gets the current weak global tracking snapshot.
    pub fn global_dock_tracking_state(&self) -> GlobalDockTrackingState {
        self.global_tracking.borrow().clone()
    }
    /// Gets the tracked dockable if alive.
    pub fn current_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.global_dock_tracking_state().dockable()
    }
    /// Gets the tracked root if alive.
    pub fn current_root_dock(&self) -> Option<Rc<dyn IRootDock>> {
        self.global_dock_tracking_state().root_dock()
    }
    /// Gets the tracked floating window if alive.
    pub fn current_dock_window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.global_dock_tracking_state().window()
    }
    /// Gets the tracked window's current host.
    pub fn current_host_window(&self) -> Option<Rc<dyn IHostWindow>> {
        self.global_dock_tracking_state().host_window()
    }

    pub(super) fn initialize_global_dock_tracking(&self, root: Rc<dyn IRootDock>) {
        let dockable = Self::resolve_tracked_root(root.clone());
        let resolved = Self::resolve_root_dock(dockable.clone()).unwrap_or(root);
        let window = resolved.window();
        self.hooks().update_global_dock_tracking(
            dockable,
            Some(resolved),
            window,
            DockTrackingChangeReason::LayoutInitialized,
        );
    }

    fn resolve_tracked_root(root: Rc<dyn IRootDock>) -> Option<Rc<dyn IDockable>> {
        if let Some(focused) = root.focused_dockable() {
            return Some(focused);
        }
        let mut current = root.active_dockable();
        let mut visited = HashMap::new();
        while let Some(dock) = current.clone().and_then(|item| item.as_dock()) {
            let Some(active) = dock.active_dockable() else {
                break;
            };
            let item = current.as_ref().expect("source current item exists");
            if visited
                .insert(Rc::as_ptr(item).cast::<()>() as usize, item.clone())
                .is_some()
            {
                break;
            }
            current = Some(active);
        }
        current
    }

    /// Resolves a candidate through focused-then-active descendants, stopping at reference cycles.
    pub fn resolve_tracked_dockable(
        dockable: Option<Rc<dyn IDockable>>,
    ) -> Option<Rc<dyn IDockable>> {
        let mut current = dockable;
        let mut visited = HashMap::new();
        while let Some(dock) = current.clone().and_then(|item| item.as_dock()) {
            let item = current.as_ref().expect("source current item exists");
            if visited
                .insert(Rc::as_ptr(item).cast::<()>() as usize, item.clone())
                .is_some()
            {
                break;
            }
            let next = dock.focused_dockable().or_else(|| dock.active_dockable());
            let Some(next) = next else { break };
            current = Some(next);
        }
        current
    }

    /// Resolves the first root on the source owner chain.
    pub fn resolve_root_dock(dockable: Option<Rc<dyn IDockable>>) -> Option<Rc<dyn IRootDock>> {
        if let Some(root) = dockable.clone().and_then(|item| item.as_root_dock()) {
            return Some(root);
        }
        let mut current = dockable;
        while let Some(item) = current {
            if let Some(root) = item.clone().as_root_dock() {
                return Some(root);
            }
            current = item.owner();
        }
        None
    }

    /// Replaces tracking and raises the protected event only when resolved identities change.
    pub fn update_global_dock_tracking(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        reason: DockTrackingChangeReason,
    ) {
        let resolved_root = root
            .or_else(|| Self::resolve_root_dock(dockable.clone()))
            .or_else(|| window.as_ref().and_then(|window| window.layout()));
        let resolved_window =
            window.or_else(|| resolved_root.as_ref().and_then(|root| root.window()));
        let resolved_dockable =
            if dockable.is_none() && reason == DockTrackingChangeReason::WindowActivated {
                Self::resolve_tracked_dockable(
                    resolved_root
                        .as_ref()
                        .and_then(|root| root.focused_dockable())
                        .or_else(|| {
                            resolved_root
                                .as_ref()
                                .and_then(|root| root.active_dockable())
                        }),
                )
            } else {
                dockable
            };
        let next = GlobalDockTrackingState::new(
            resolved_dockable.clone(),
            resolved_root.clone(),
            resolved_window.clone(),
        );
        let previous = self.global_dock_tracking_state();
        if same(&previous.dockable(), &next.dockable())
            && same(&previous.root_dock(), &next.root_dock())
            && same(&previous.window(), &next.window())
        {
            return;
        }
        self.global_tracking.replace(next.clone());
        self.hooks()
            .on_global_dock_tracking_changed(previous, next, reason);
    }

    /// Clears tracking when the removed window or its layout matches the current state.
    pub fn clear_global_dock_tracking_for_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        reason: DockTrackingChangeReason,
    ) {
        let Some(window) = window else { return };
        let tracked_window = self.current_dock_window();
        let tracked_root = self.current_root_dock();
        if !same(&tracked_window, &Some(window.clone())) && !same(&tracked_root, &window.layout()) {
            return;
        }
        self.hooks()
            .update_global_dock_tracking(None, None, None, reason);
    }

    /// Raises the global tracking event with shared previous/current snapshots.
    pub fn on_global_dock_tracking_changed(
        &self,
        previous: GlobalDockTrackingState,
        current: GlobalDockTrackingState,
        reason: DockTrackingChangeReason,
    ) {
        if self.tracking_changed.has_handlers() {
            self.tracking_changed
                .raise(&GlobalDockTrackingChangedEventArgs::new(
                    previous, current, reason,
                ));
        }
    }
}
