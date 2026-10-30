// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::IDockable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An optional source dockable (or window owner) dirty-tracking predicate.
pub type WorkspaceDockableFilter = Rc<dyn Fn(Option<Rc<dyn IDockable>>) -> bool>;

/// Shared mutable options for workspace layout change tracking.
pub struct DockWorkspaceTrackingOptions {
    filter: RefCell<Option<WorkspaceDockableFilter>>,
    track_window_moves: Cell<bool>,
}
impl Default for DockWorkspaceTrackingOptions {
    fn default() -> Self {
        Self {
            filter: RefCell::new(None),
            track_window_moves: Cell::new(true),
        }
    }
}
impl DockWorkspaceTrackingOptions {
    /// Creates default options with window move tracking enabled.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }
    /// Gets the optional dockable/window-owner filter.
    pub fn dockable_filter(&self) -> Option<WorkspaceDockableFilter> {
        self.filter.borrow().clone()
    }
    /// Sets the filter. Changes also affect managers already using these options.
    pub fn set_dockable_filter(&self, value: Option<WorkspaceDockableFilter>) {
        self.filter.replace(value);
    }
    /// Gets whether window drag-end events mark the layout dirty.
    pub fn track_window_moves(&self) -> bool {
        self.track_window_moves.get()
    }
    /// Sets whether window drag-end events mark the layout dirty.
    pub fn set_track_window_moves(&self, value: bool) {
        self.track_window_moves.set(value);
    }
}
