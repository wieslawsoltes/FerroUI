// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::FactoryBase;
use crate::controls::{IRootDock, IToolDock};
use crate::core::*;
use std::rc::Rc;

/// Native virtual dispatch for the source protected factory extension points.
/// Compose FactoryBase into a complete IFactory and return it from factory_base.
/// Overrides can call the component method directly to invoke the source base body.
pub trait FactoryBaseImpl: IFactory {
    /// Returns the shared abstract factory component belonging to this runtime.
    fn factory_base(&self) -> &FactoryBase;

    /// Tracks a generated dockable using its current Context value.
    fn track_items_source_dockable(&self, dockable: Rc<dyn IDockable>, owner: Rc<dyn IDockable>) {
        self.factory_base()
            .track_items_source_dockable(dockable, owner);
    }
    /// Tracks a generated dockable with its explicit source item, including null.
    fn track_items_source_dockable_with_item(
        &self,
        dockable: Rc<dyn IDockable>,
        owner: Rc<dyn IDockable>,
        item: Option<crate::BoxedValue>,
    ) {
        self.factory_base()
            .track_items_source_dockable_with_item(dockable, owner, item);
    }
    /// Removes generated-container tracking and reverse item entries.
    fn untrack_items_source_dockable(&self, dockable: Rc<dyn IDockable>) {
        self.factory_base().untrack_items_source_dockable(dockable);
    }
    /// Resolves the originally tracked source owner independently of current ownership.
    fn get_tracked_items_source_owner(
        &self,
        dockable: Rc<dyn IDockable>,
    ) -> Option<Rc<dyn IDockable>> {
        self.factory_base().get_tracked_items_source_owner(dockable)
    }
    /// Closes while bypassing command eligibility, retaining cancellation and lifecycle semantics.
    fn close_dockable_core(&self, dockable: Rc<dyn IDockable>) {
        self.factory_base().close_dockable_core(dockable);
    }
    /// Removes the source item and reports whether a source-backed dock recognized the container.
    fn handle_items_source_dockable_closing(&self, dockable: Rc<dyn IDockable>) -> bool {
        self.factory_base()
            .handle_items_source_dockable_closing(dockable)
    }

    /// Toggles pinning without command capability checks.
    fn pin_dockable_core(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.factory_base().pin_dockable_core(dockable);
    }

    /// Toggles pinning within an explicit root and optional unpin destination.
    fn pin_dockable_core_with_root(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
        unpin: Option<Rc<dyn IToolDock>>,
    ) {
        self.factory_base()
            .pin_dockable_core_with_root(dockable, root, unpin);
    }

    /// Chooses the next active child after an active item is removed.
    fn get_active_dockable_after_removal(
        &self,
        dock: Option<Rc<dyn IDock>>,
        removed: Option<Rc<dyn IDockable>>,
        removed_index: i32,
    ) -> Option<Rc<dyn IDockable>> {
        self.factory_base()
            .get_active_dockable_after_removal(dock, removed, removed_index)
    }

    /// Resolves the outer root for floating window registration.
    fn resolve_window_collection_root(
        &self,
        root: Option<Rc<dyn IRootDock>>,
    ) -> Option<Rc<dyn IRootDock>> {
        self.factory_base().resolve_window_collection_root(root)
    }

    /// Applies optional window options through the overridable source hook.
    fn apply_window_options(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.factory_base().apply_window_options(window, options);
    }

    /// Prepares source-window ownership options before the dockable moves.
    fn prepare_window_options_for_dockable(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.factory_base()
            .prepare_window_options_for_dockable(dockable, options);
    }

    /// Invokes the overridable closed-window lifecycle notification.
    fn notify_window_closed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.factory_base().notify_window_closed(window);
    }

    /// Invokes the overridable removed-window lifecycle notification.
    fn notify_window_removed(&self, window: Option<Rc<dyn IDockWindow>>) {
        self.factory_base().notify_window_removed(window);
    }

    /// Updates global tracking through the source overridable transition point.
    fn update_global_dock_tracking(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
        root: Option<Rc<dyn IRootDock>>,
        window: Option<Rc<dyn IDockWindow>>,
        reason: DockTrackingChangeReason,
    ) {
        self.factory_base()
            .update_global_dock_tracking(dockable, root, window, reason);
    }

    /// Clears tracking for a removed or closed window.
    fn clear_global_dock_tracking_for_window(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        reason: DockTrackingChangeReason,
    ) {
        self.factory_base()
            .clear_global_dock_tracking_for_window(window, reason);
    }

    /// Raises the tracking event after the current state has been replaced.
    fn on_global_dock_tracking_changed(
        &self,
        previous: GlobalDockTrackingState,
        current: GlobalDockTrackingState,
        reason: DockTrackingChangeReason,
    ) {
        self.factory_base()
            .on_global_dock_tracking_changed(previous, current, reason);
    }

    /// Determines whether initializing pane selection should also request focus.
    fn should_focus_active_dockable(
        &self,
        _dockable: Option<Rc<dyn IDockable>>,
        _owner: Option<Rc<dyn IDock>>,
    ) -> bool {
        true
    }

    /// Updates the logical state and related flags for one dockable.
    fn update_docking_window_state(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.factory_base().update_docking_window_state(dockable);
    }

    /// Updates the dockable and its visible and split-view descendants.
    fn update_docking_window_state_recursive(&self, dockable: Option<Rc<dyn IDockable>>) {
        self.factory_base()
            .update_docking_window_state_recursive(dockable);
    }

    /// Resolves location, floating and hidden state from current ownership.
    fn resolve_docking_window_state(
        &self,
        dockable: Option<Rc<dyn IDockable>>,
    ) -> DockingWindowState {
        self.factory_base().resolve_docking_window_state(dockable)
    }

    /// Applies a requested model property change and reconciles the resulting flags.
    fn on_docking_window_state_property_changed(
        &self,
        dockable: Rc<dyn IDockable>,
        property: DockingWindowStateProperty,
    ) {
        self.factory_base()
            .on_docking_window_state_property_changed(dockable, property);
    }
}
