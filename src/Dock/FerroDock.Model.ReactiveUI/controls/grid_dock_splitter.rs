// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::core::{DockableBase, DockableBaseImpl};
use crate::impl_dockable;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A reactive grid dock splitter model.
pub struct GridDockSplitter {
    base: DockableBase,
    resize_direction: RefCell<GridResizeDirection>,
}
reactive_object!(GridDockSplitter, base);
impl DockableBaseImpl for GridDockSplitter {}
impl_dockable!(
    GridDockSplitter,
    fn as_splitter(self: Rc<Self>) -> Option<Rc<dyn ISplitter>> {
        Some(self)
    },
    fn as_grid_dock_splitter(self: Rc<Self>) -> Option<Rc<dyn IGridDockSplitter>> {
        Some(self)
    }
);
impl ISplitter for GridDockSplitter {}
impl GridDockSplitter {
    /// Creates a shared splitter with the source defaults.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak: &Weak<Self>| Self::construct(weak.clone()))
    }
    /// Constructs this component with the enclosing model's reactive identity.
    pub fn construct<T: DockableBaseImpl + IGridDockSplitter + 'static>(runtime: Weak<T>) -> Self {
        let runtime: Weak<dyn DockableBaseImpl> = runtime;
        Self {
            base: DockableBase::construct(runtime),
            resize_direction: RefCell::new(GridResizeDirection::Columns),
        }
    }
    /// Gets `ResizeDirection`.
    pub fn resize_direction(&self) -> GridResizeDirection {
        *self.resize_direction.borrow()
    }
    /// Sets `ResizeDirection`, notifying only when changed.
    pub fn set_resize_direction(&self, value: GridResizeDirection) {
        self.base.runtime().raise_and_set_if_changed(
            &self.resize_direction,
            value,
            "ResizeDirection",
        );
    }
}
impl IGridDockSplitter for GridDockSplitter {
    fn resize_direction(&self) -> GridResizeDirection {
        self.resize_direction()
    }
    fn set_resize_direction(&self, value: GridResizeDirection) {
        self.set_resize_direction(value);
    }
}
